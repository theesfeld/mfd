//! High-level vehicle session: ELM + J1979 + optional UDS.

use crate::obd::elm::Elm;
use crate::obd::error::{Error, Result};
use crate::obd::j1979::{self, LiveValue};
use crate::obd::replay::ReplayTransport;
#[cfg(target_os = "linux")]
use crate::obd::transport::BtSppTransport;
use crate::obd::transport::{SerialTransport, Transport};
use crate::obd::uds;
use std::path::Path;
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

pub struct Session {
    elm: Elm,
}

impl Session {
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

    pub fn from_env() -> Result<Option<Self>> {
        let replay = std::env::var_os("MFD_OBD_REPLAY").map(std::path::PathBuf::from);
        let port = std::env::var("MFD_OBD_PORT").ok().filter(|s| !s.is_empty());
        let bt = std::env::var("MFD_OBD_BT").ok().filter(|s| !s.is_empty());
        if replay.is_none() && port.is_none() && bt.is_none() {
            return Ok(None);
        }
        let baud = std::env::var("MFD_OBD_BAUD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(115_200);
        let ch = std::env::var("MFD_OBD_BT_CHANNEL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        Ok(Some(Self::connect(ConnectOpts {
            serial_path: port,
            baud,
            bt_mac: bt,
            bt_channel: ch,
            replay,
            timeout: Duration::from_millis(4_000),
        })?))
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
                    // Next support PID is indicated by last bit of each group
                    all.extend(
                        found
                            .iter()
                            .copied()
                            .filter(|&p| !j1979::SUPPORT_PIDS.contains(&p)),
                    );
                    // Continue chain only if next support PID listed
                    let next = sp.saturating_add(0x20);
                    if !found.contains(&next) && sp != 0x00 {
                        // still try continue once
                    }
                }
                Err(_) => break,
            }
        }
        all.sort_unstable();
        all.dedup();
        if all.is_empty() {
            // Fallback: priority list
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
                // Mode 07 may be unsupported — keep stored
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
        // Find ASCII in payload
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
