//! High-level vehicle session: ELM + J1979 + optional UDS.
//!
//! Connection is **retry-friendly**: callers can loop [`Session::connect`] /
//! [`Session::connect_resilient`] until the adapter answers.

use crate::obd::elm::Elm;
use crate::obd::error::{Error, Result};
use crate::obd::j1979::{self, LiveValue};
use crate::obd::replay::ReplayTransport;
#[cfg(target_os = "linux")]
use crate::obd::transport::BtSppTransport;
use crate::obd::transport::{
    bluez_power_on, discover_obd_macs, normalize_mac, rfcomm_channel_candidates, SerialTransport,
    Transport,
};
use crate::obd::uds;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub struct ConnectOpts {
    pub serial_path: Option<String>,
    pub baud: u32,
    pub bt_mac: Option<String>,
    pub bt_channel: u8,
    pub replay: Option<std::path::PathBuf>,
    pub timeout: Duration,
}

impl Default for ConnectOpts {
    fn default() -> Self {
        Self {
            serial_path: None,
            baud: 115_200,
            bt_mac: None,
            bt_channel: 1,
            replay: None,
            timeout: Duration::from_millis(4_000),
        }
    }
}

impl ConnectOpts {
    /// Build opts from env. `None` if no OBD source is configured.
    pub fn from_env() -> Option<Self> {
        let replay = std::env::var_os("MFD_OBD_REPLAY").map(std::path::PathBuf::from);
        let port = std::env::var("MFD_OBD_PORT").ok().filter(|s| !s.is_empty());
        let bt = std::env::var("MFD_OBD_BT").ok().filter(|s| !s.is_empty());
        if replay.is_none() && port.is_none() && bt.is_none() {
            return None;
        }
        let baud = std::env::var("MFD_OBD_BAUD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(115_200);
        let ch = std::env::var("MFD_OBD_BT_CHANNEL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        Some(Self {
            serial_path: port,
            baud,
            bt_mac: bt,
            bt_channel: ch,
            replay,
            timeout: Duration::from_millis(4_000),
        })
    }

    pub fn is_bluetooth(&self) -> bool {
        self.bt_mac.is_some() && self.serial_path.is_none() && self.replay.is_none()
    }

    pub fn is_replay(&self) -> bool {
        self.replay.is_some()
    }
}

pub struct Session {
    elm: Elm,
}

impl Session {
    /// One-shot connect + ELM init (no retry).
    pub fn connect(opts: ConnectOpts) -> Result<Self> {
        let transport: Box<dyn Transport> = if let Some(p) = opts.replay {
            Box::new(ReplayTransport::from_path(p)?)
        } else if let Some(mac) = opts.bt_mac {
            #[cfg(target_os = "linux")]
            {
                Box::new(BtSppTransport::new(mac, opts.bt_channel, opts.timeout)?)
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = mac;
                return Err(Error::Adapter(
                    "Bluetooth SPP is only implemented on Linux".into(),
                ));
            }
        } else if let Some(path) = opts.serial_path {
            Box::new(SerialTransport::new(path, opts.baud, opts.timeout))
        } else {
            return Err(Error::Adapter(
                "set serial path, bt MAC, or replay path".into(),
            ));
        };
        let mut elm = Elm::new(transport, opts.timeout);
        elm.open_and_init()?;
        elm.try_stn_hints();
        Ok(Self { elm })
    }

    /// Connect once from env (fails if env unset or connect fails).
    pub fn from_env() -> Result<Option<Self>> {
        let Some(opts) = ConnectOpts::from_env() else {
            return Ok(None);
        };
        Ok(Some(Self::connect(opts)?))
    }

    /// Keep trying until the adapter answers or `stop` is set.
    ///
    /// Bluetooth path:
    /// 1. Power BlueZ adapter
    /// 2. Prefer configured MAC; also try paired OBD-like names
    /// 3. Try RFCOMM channels (configured + common set)
    /// 4. Back off and retry forever (product needs the bus)
    ///
    /// `on_status(msg)` is called each attempt for glass/telemetry.
    pub fn connect_resilient(
        opts: &ConnectOpts,
        stop: &AtomicBool,
        mut on_status: impl FnMut(&str),
    ) -> Result<Self> {
        // Replay / serial: still retry open (device may appear late) but no BT scan.
        if opts.replay.is_some() {
            let mut attempt = 0u32;
            loop {
                if stop.load(Ordering::Relaxed) {
                    return Err(Error::Adapter("stopped while connecting".into()));
                }
                attempt = attempt.wrapping_add(1);
                on_status(&format!("REPLAY try {attempt}"));
                match Self::connect(ConnectOpts {
                    replay: opts.replay.clone(),
                    timeout: opts.timeout,
                    ..Default::default()
                }) {
                    Ok(s) => return Ok(s),
                    Err(e) => {
                        on_status(&format!("REPLAY fail: {e}"));
                        sleep_backoff(attempt, stop);
                    }
                }
            }
        }

        if let Some(ref path) = opts.serial_path {
            let mut attempt = 0u32;
            loop {
                if stop.load(Ordering::Relaxed) {
                    return Err(Error::Adapter("stopped while connecting".into()));
                }
                attempt = attempt.wrapping_add(1);
                on_status(&format!("SERIAL {path} try {attempt}"));
                match Self::connect(ConnectOpts {
                    serial_path: Some(path.clone()),
                    baud: opts.baud,
                    timeout: opts.timeout,
                    ..Default::default()
                }) {
                    Ok(s) => return Ok(s),
                    Err(e) => {
                        on_status(&format!("SERIAL fail: {e}"));
                        sleep_backoff(attempt, stop);
                    }
                }
            }
        }

        // ── Bluetooth resilient path ──────────────────────────────────────
        bluez_power_on();
        let preferred = opts.bt_mac.as_deref().and_then(normalize_mac);
        let channels = rfcomm_channel_candidates(opts.bt_channel);
        let mut attempt = 0u32;
        let mut last_hint = 0u32;

        loop {
            if stop.load(Ordering::Relaxed) {
                return Err(Error::Adapter("stopped while connecting".into()));
            }
            attempt = attempt.wrapping_add(1);
            let macs = discover_obd_macs(preferred.as_deref());
            if macs.is_empty() {
                on_status("SEARCH no OBD MAC — set MFD_OBD_BT or pair dongle");
                if attempt.saturating_sub(last_hint) >= 5 {
                    last_hint = attempt;
                    eprintln!(
                        "mfd obd: no Bluetooth OBD found. Pair the dongle:\n  \
                         bluetoothctl\n    \
                         power on\n    \
                         scan on   # put dongle in pairing mode if needed\n    \
                         pair AA:BB:CC:DD:EE:FF\n    \
                         trust AA:BB:CC:DD:EE:FF\n    \
                         connect AA:BB:CC:DD:EE:FF\n  \
                         then: export MFD_OBD_BT=AA:BB:CC:DD:EE:FF"
                    );
                }
                sleep_backoff(attempt, stop);
                continue;
            }

            let mut last_err = String::new();
            for mac in &macs {
                if stop.load(Ordering::Relaxed) {
                    return Err(Error::Adapter("stopped while connecting".into()));
                }
                for &ch in &channels {
                    if stop.load(Ordering::Relaxed) {
                        return Err(Error::Adapter("stopped while connecting".into()));
                    }
                    on_status(&format!("BT {mac} ch{ch} try {attempt}"));
                    match Self::connect(ConnectOpts {
                        bt_mac: Some(mac.clone()),
                        bt_channel: ch,
                        timeout: opts.timeout,
                        ..Default::default()
                    }) {
                        Ok(s) => {
                            eprintln!("mfd obd: connected {mac} RFCOMM ch{ch}");
                            return Ok(s);
                        }
                        Err(e) => {
                            last_err = e.to_string();
                            // Next channel / MAC quickly; no long sleep inside matrix.
                        }
                    }
                }
            }

            on_status(&format!("SEARCH fail: {last_err}"));
            if attempt.saturating_sub(last_hint) >= 8 {
                last_hint = attempt;
                let pref = preferred.as_deref().unwrap_or("AA:BB:CC:DD:EE:FF");
                eprintln!(
                    "mfd obd: still searching for OBD Bluetooth (last: {last_err}).\n  \
                     Preferred MAC: {pref}\n  \
                     Tried channels: {channels:?}\n  \
                     Candidates: {macs:?}\n  \
                     If unpaired: put dongle in pairing mode, then:\n    \
                     bluetoothctl pair {pref} && bluetoothctl trust {pref} && bluetoothctl connect {pref}\n  \
                     Only one RFCOMM client at a time (close other OBD apps)."
                );
            }
            sleep_backoff(attempt, stop);
        }
    }

    pub fn name(&self) -> &str {
        self.elm.name()
    }

    pub fn identity(&self) -> &str {
        self.elm.identity()
    }

    pub fn protocol(&self) -> &str {
        self.elm.protocol()
    }

    pub fn elm_mut(&mut self) -> &mut Elm {
        &mut self.elm
    }

    pub fn read_pid(&mut self, pid: u8) -> Result<LiveValue> {
        let cmd = j1979::mode01_command(pid);
        let bytes = self.elm.request_hex(&cmd)?;
        j1979::decode_mode01(&bytes)
    }

    /// Discover supported Mode 01 PIDs via 0100 / 0120 / … bitmaps.
    pub fn discover_mode01_pids(&mut self) -> Result<Vec<u8>> {
        let mut all = Vec::new();
        for &sp in j1979::SUPPORT_PIDS {
            let cmd = j1979::mode01_command(sp);
            match self.elm.request_hex(&cmd) {
                Ok(bytes) => {
                    let found = j1979::parse_support_bitmap(sp, &bytes);
                    if found.is_empty() {
                        break;
                    }
                    all.extend(
                        found
                            .iter()
                            .copied()
                            .filter(|&p| !j1979::SUPPORT_PIDS.contains(&p)),
                    );
                    let _next = sp.saturating_add(0x20);
                }
                Err(_) => break,
            }
        }
        all.sort_unstable();
        all.dedup();
        if all.is_empty() {
            all.extend_from_slice(j1979::PRIORITY_PIDS);
        }
        Ok(all)
    }

    /// Raw ELM request; returns hex payload bytes (for logging).
    pub fn request_raw(&mut self, cmd: &str) -> Result<Vec<u8>> {
        self.elm.request_hex(cmd)
    }

    /// Mode 03 stored DTCs (read-only).
    pub fn read_dtc_stored(&mut self) -> Result<Vec<j1979::Dtc>> {
        self.read_dtc_mode("03", j1979::DtcKind::Stored)
    }

    /// Mode 07 pending DTCs (read-only).
    pub fn read_dtc_pending(&mut self) -> Result<Vec<j1979::Dtc>> {
        self.read_dtc_mode("07", j1979::DtcKind::Pending)
    }

    /// Mode 0A permanent DTCs (read-only).
    pub fn read_dtc_permanent(&mut self) -> Result<Vec<j1979::Dtc>> {
        self.read_dtc_mode("0A", j1979::DtcKind::Permanent)
    }

    /// Load all DTC classes and merge (immediate fault inventory).
    pub fn read_all_dtcs(&mut self) -> Result<Vec<j1979::Dtc>> {
        let mut lists = Vec::new();
        match self.read_dtc_stored() {
            Ok(v) => lists.push(v),
            Err(Error::NoData) => lists.push(Vec::new()),
            Err(e) => return Err(e),
        }
        match self.read_dtc_pending() {
            Ok(v) => lists.push(v),
            Err(Error::NoData) => lists.push(Vec::new()),
            Err(e) => {
                let _ = e;
                lists.push(Vec::new());
            }
        }
        match self.read_dtc_permanent() {
            Ok(v) => lists.push(v),
            Err(Error::NoData) => lists.push(Vec::new()),
            Err(_) => lists.push(Vec::new()),
        }
        Ok(j1979::merge_dtcs(&lists))
    }

    fn read_dtc_mode(&mut self, mode: &str, kind: j1979::DtcKind) -> Result<Vec<j1979::Dtc>> {
        let raw = self.elm.cmd(mode)?;
        if raw.to_ascii_uppercase().contains("NO DATA") {
            return Ok(Vec::new());
        }
        let bytes = crate::obd::elm::parse_elm_hex_payload(&raw)?;
        j1979::decode_dtc_response(&bytes, kind)
    }

    pub fn read_vin_mode09(&mut self) -> Result<String> {
        // Mode 09 PID 02 — VIN (often multi-frame; ELM may return concatenated)
        let raw = self.elm.cmd("0902")?;
        let bytes = crate::obd::elm::parse_elm_hex_payload(&raw)?;
        let ascii: String = bytes
            .iter()
            .filter(|b| b.is_ascii_alphanumeric())
            .map(|b| *b as char)
            .collect();
        if ascii.len() >= 17 {
            Ok(ascii.chars().take(17).collect())
        } else if !ascii.is_empty() {
            Ok(ascii)
        } else {
            Err(Error::NoData)
        }
    }

    pub fn read_did(&mut self, header: &str, did: u16) -> Result<Vec<u8>> {
        uds::read_did_on_module(&mut self.elm, header, did)
    }

    pub fn tester_present(&mut self) -> Result<()> {
        uds::tester_present(&mut self.elm, true)
    }

    pub fn extended_session(&mut self) -> Result<Vec<u8>> {
        uds::diagnostic_session_control(&mut self.elm, 0x03)
    }
}

/// Open capture directory for replay if `path` is a directory with frames.
pub fn resolve_replay_path(path: &Path) -> &Path {
    path
}

fn sleep_backoff(attempt: u32, stop: &AtomicBool) {
    // 0.5s → 1s → 2s → … cap 5s
    let ms = match attempt {
        0 | 1 => 500,
        2 => 1000,
        3 => 2000,
        4 => 3000,
        _ => 5000,
    };
    let step = 100u64;
    let mut left = ms;
    while left > 0 && !stop.load(Ordering::Relaxed) {
        let d = left.min(step);
        thread::sleep(Duration::from_millis(d));
        left = left.saturating_sub(d);
    }
}
