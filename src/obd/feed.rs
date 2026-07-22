//! Background poller → [`VehicleSnapshot`](crate::auto::VehicleSnapshot).
//!
//! **Link policy:** when BT/serial/replay env is set, the feed **always starts**
//! and **keeps searching / reconnecting** until the process exits. A single failed
//! connect does **not** drop the glass to silent SIM.
//!
//! Startup: resilient connect → capability probe (BIT) → DTC + Mode 01 / Ford DID poll.
//! Optional **capture** via `MFD_OBD_CAPTURE` (same process owns Bluetooth — only one
//! RFCOMM client can attach to the ELM).
//! Optional **crush** via `MFD_OBD_CRUSH=1`: discover every Mode 01 PID + multi-module
//! known UDS DIDs and log every TX/RX.
//!
//! Read-only — never Mode 04 clear.

use crate::auto::caps::VehicleCaps;
use crate::auto::probe;
use crate::auto::{DtcEntry, DtcKind, VehicleSnapshot};
use crate::obd::capture::CaptureWriter;
use crate::obd::error::Error as ObdError;
use crate::obd::ford::{self, DecodedDid, HDR_ABS, HDR_BCM, HDR_IPC, HDR_PCM, HDR_PSCM};
use crate::obd::j1979::{self, PRIORITY_PIDS};
use crate::obd::session::{ConnectOpts, Session};
use crate::obd::uds::{self, PROBE_DIDS};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
struct Telemetry {
    values: HashMap<String, f64>,
    status: String,
    error: Option<String>,
    vin: Option<String>,
    ticks: u64,
    dtcs: Vec<DtcEntry>,
    dtc_loaded: bool,
    caps: VehicleCaps,
    capture_dir: Option<String>,
    /// BT · SERIAL · REPLAY
    link_kind: String,
    /// MAC or path
    link_addr: String,
    link_channel: String,
    adapter_id: String,
    protocol: String,
    /// SEARCH · CONN · BIT · LIVE · RECONN
    link_phase: String,
}

/// Background OBD poller (native stack).
pub struct ObdFeed {
    stop: Arc<AtomicBool>,
    tele: Arc<Mutex<Telemetry>>,
    _join: Option<JoinHandle<()>>,
}

impl ObdFeed {
    /// Start if `MFD_OBD_PORT`, `MFD_OBD_BT`, or `MFD_OBD_REPLAY` is set.
    ///
    /// Starts the worker **immediately** even if the dongle is offline — the
    /// worker keeps searching / reconnecting. Never fails open into silent SIM
    /// when OBD env is configured.
    pub fn try_start_from_env() -> Option<Self> {
        let opts = ConnectOpts::from_env()?;
        match Self::start_with_opts(opts) {
            Ok(f) => Some(f),
            Err(e) => {
                eprintln!("mfd obd: failed to start feed thread: {e}");
                None
            }
        }
    }

    /// Start resilient feed from connect options (does not require a live session yet).
    pub fn start_with_opts(opts: ConnectOpts) -> Result<Self, String> {
        let (kind, addr, channel) = link_from_opts(&opts);
        let stop = Arc::new(AtomicBool::new(false));
        let tele = Arc::new(Mutex::new(Telemetry {
            status: format!("SEARCH {kind} {addr}"),
            caps: VehicleCaps {
                phase: "SEARCH OBD".into(),
                link: format!("{kind} {addr}"),
                ..Default::default()
            },
            link_kind: kind,
            link_addr: addr,
            link_channel: channel,
            link_phase: "SEARCH".into(),
            ..Default::default()
        }));
        let stop_t = Arc::clone(&stop);
        let tele_t = Arc::clone(&tele);

        let join = thread::Builder::new()
            .name("mfd-obd".into())
            .spawn(move || supervisor_loop(opts, stop_t, tele_t))
            .map_err(|e| e.to_string())?;

        Ok(Self {
            stop,
            tele,
            _join: Some(join),
        })
    }

    /// Legacy: start from an already-open session (one-shot; no outer reconnect).
    pub fn from_session(session: Session) -> Result<Self, String> {
        let (kind, addr, channel) = link_from_env_and_name(session.name());
        let stop = Arc::new(AtomicBool::new(false));
        let tele = Arc::new(Mutex::new(Telemetry {
            status: format!("probe {}", session.name()),
            caps: VehicleCaps {
                phase: "POWER ON".into(),
                link: format!("{kind} {addr}"),
                ..Default::default()
            },
            link_kind: kind,
            link_addr: addr,
            link_channel: channel,
            adapter_id: session.identity().to_string(),
            protocol: session.protocol().to_string(),
            link_phase: "BIT".into(),
            ..Default::default()
        }));
        let stop_t = Arc::clone(&stop);
        let tele_t = Arc::clone(&tele);

        let join = thread::Builder::new()
            .name("mfd-obd".into())
            .spawn(move || {
                let mut cap = None;
                let _ = run_session(session, &stop_t, &tele_t, &mut cap, false);
                finish_cap(cap);
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            stop,
            tele,
            _join: Some(join),
        })
    }

    pub fn apply_to(&self, v: &mut VehicleSnapshot) {
        let Ok(t) = self.tele.lock() else {
            return;
        };
        apply_telemetry(&t, v);
    }

    pub fn caps(&self) -> VehicleCaps {
        self.tele.lock().map(|t| t.caps.clone()).unwrap_or_default()
    }

    pub fn status_line(&self) -> String {
        self.tele
            .lock()
            .map(|t| {
                let addr = if t.link_addr.is_empty() {
                    "—".into()
                } else {
                    t.link_addr.clone()
                };
                let phase = if t.link_phase.is_empty() {
                    "…"
                } else {
                    t.link_phase.as_str()
                };
                if phase == "SEARCH" || phase == "RECONN" || phase == "CONN" {
                    return format!(
                        "{} {} · {addr} · {}",
                        t.link_kind,
                        phase,
                        if t.status.is_empty() {
                            "…"
                        } else {
                            &t.status
                        }
                    );
                }
                if !t.caps.ready {
                    format!(
                        "BIT {:.0}% {} · {} {}",
                        t.caps.progress * 100.0,
                        t.caps.phase,
                        t.link_kind,
                        addr
                    )
                } else if let Some(e) = &t.error {
                    format!("{} ERR {e} · {addr}", t.link_kind)
                } else {
                    let d = if t.dtc_loaded {
                        format!(" DTC{}", t.dtcs.len())
                    } else {
                        String::new()
                    };
                    let cap = t
                        .capture_dir
                        .as_ref()
                        .map(|p| format!(" CAP {}", short_path(p)))
                        .unwrap_or_default();
                    format!(
                        "{} LIVE · {addr} · {} · t{}{d}{cap}",
                        t.link_kind,
                        if t.protocol.is_empty() {
                            "—"
                        } else {
                            &t.protocol
                        },
                        t.ticks
                    )
                }
            })
            .unwrap_or_else(|_| "OBD lock".into())
    }
}

/// Outer loop: connect forever, run session, on link death reconnect.
fn supervisor_loop(opts: ConnectOpts, stop: Arc<AtomicBool>, tele: Arc<Mutex<Telemetry>>) {
    eprintln!(
        "mfd obd: supervisor start (resilient connect) · {}",
        opts.bt_mac
            .as_deref()
            .or(opts.serial_path.as_deref())
            .unwrap_or("replay")
    );
    let mut cap: Option<CaptureWriter> = None;
    let mut ever_live = false;
    while !stop.load(Ordering::Relaxed) {
        let phase = if ever_live { "RECONN" } else { "SEARCH" };
        set_phase(
            &tele,
            phase,
            if ever_live { "reconnect" } else { "searching" },
        );

        let session = match Session::connect_resilient(&opts, &stop, |msg| {
            set_phase(&tele, if ever_live { "RECONN" } else { "CONN" }, msg);
        }) {
            Ok(s) => s,
            Err(e) => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                eprintln!("mfd obd: connect aborted: {e}");
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        // Refresh address from live transport name (bt://MAC:ch).
        if let Ok(mut t) = tele.lock() {
            let name = session.name().to_string();
            if let Some(rest) = name.strip_prefix("bt://") {
                if let Some((mac, ch)) = rest.split_once(':') {
                    t.link_addr = mac.to_string();
                    t.link_channel = ch.to_string();
                } else {
                    t.link_addr = rest.to_string();
                }
            }
            t.adapter_id = session.identity().to_string();
            t.protocol = session.protocol().to_string();
            t.error = None;
            t.link_phase = "BIT".into();
            t.status = format!("probe {}", session.name());
        }

        let adapter = format!("{} · {}", session.name(), session.protocol());
        if cap.is_none() {
            cap = open_capture(&adapter);
            if let Some(ref c) = cap {
                if let Ok(mut t) = tele.lock() {
                    t.capture_dir = Some(c.dir().display().to_string());
                }
            }
        }

        match run_session(session, &stop, &tele, &mut cap, ever_live) {
            SessionEnd::Stop => break,
            SessionEnd::LinkLost(reason) => {
                ever_live = true;
                eprintln!("mfd obd: link lost ({reason}) — reconnecting…");
                if let Ok(mut t) = tele.lock() {
                    t.error = Some(reason);
                    t.link_phase = "RECONN".into();
                }
                // Brief pause before next connect (avoid spin).
                thread::sleep(Duration::from_millis(800));
            }
        }
    }
    finish_cap(cap);
    eprintln!("mfd obd: supervisor stop");
}

fn set_phase(tele: &Arc<Mutex<Telemetry>>, phase: &str, detail: &str) {
    if let Ok(mut t) = tele.lock() {
        t.link_phase = phase.into();
        t.status = detail.into();
        if !t.caps.ready {
            t.caps.phase = match phase {
                "SEARCH" => "SEARCH OBD".into(),
                "RECONN" => "RECONNECT".into(),
                "CONN" => "CONNECTING".into(),
                _ => detail.into(),
            };
            t.caps.link = format!("{} {}", t.link_kind, t.link_addr);
        }
        // Keep glass honest while hunting: not LIVE.
        if phase == "SEARCH" || phase == "RECONN" || phase == "CONN" {
            t.error = Some(detail.into());
        }
    }
}

enum SessionEnd {
    Stop,
    LinkLost(String),
}

/// Resolve link kind / address / channel from connect options.
fn link_from_opts(opts: &ConnectOpts) -> (String, String, String) {
    if let Some(ref mac) = opts.bt_mac {
        return ("BT".into(), mac.clone(), opts.bt_channel.to_string());
    }
    if let Some(ref port) = opts.serial_path {
        return ("SERIAL".into(), port.clone(), "-".into());
    }
    if let Some(ref rep) = opts.replay {
        return ("REPLAY".into(), rep.display().to_string(), "-".into());
    }
    ("OBD".into(), "—".into(), "-".into())
}

/// Resolve link kind / address / channel for glass from env + transport name.
fn link_from_env_and_name(transport_name: &str) -> (String, String, String) {
    if let Ok(mac) = std::env::var("MFD_OBD_BT") {
        if !mac.is_empty() {
            let ch = std::env::var("MFD_OBD_BT_CHANNEL").unwrap_or_else(|_| "1".into());
            return ("BT".into(), mac, ch);
        }
    }
    if let Ok(port) = std::env::var("MFD_OBD_PORT") {
        if !port.is_empty() {
            return ("SERIAL".into(), port, "-".into());
        }
    }
    if let Ok(rep) = std::env::var("MFD_OBD_REPLAY") {
        if !rep.is_empty() {
            return ("REPLAY".into(), rep, "-".into());
        }
    }
    // Fallback: parse transport name (bt://MAC, path, …)
    let n = transport_name.trim();
    if let Some(rest) = n.strip_prefix("bt://") {
        return ("BT".into(), rest.to_string(), "1".into());
    }
    if n.contains("replay") {
        return ("REPLAY".into(), n.into(), "-".into());
    }
    if n.starts_with('/') {
        return ("SERIAL".into(), n.into(), "-".into());
    }
    ("OBD".into(), n.into(), "-".into())
}

impl Drop for ObdFeed {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self._join.take() {
            let _ = j.join();
        }
    }
}

fn short_path(p: &str) -> String {
    let path = std::path::Path::new(p);
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(p)
        .to_string()
}

fn map_kind(k: j1979::DtcKind) -> DtcKind {
    match k {
        j1979::DtcKind::Stored => DtcKind::Stored,
        j1979::DtcKind::Pending => DtcKind::Pending,
        j1979::DtcKind::Permanent => DtcKind::Permanent,
    }
}

fn env_truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1")
            | Some("true")
            | Some("TRUE")
            | Some("yes")
            | Some("YES")
            | Some("on")
            | Some("ON")
    )
}

fn open_capture(adapter: &str) -> Option<CaptureWriter> {
    let path = std::env::var_os("MFD_OBD_CAPTURE").map(PathBuf::from)?;
    if path.as_os_str().is_empty() {
        return None;
    }
    let software = format!("mfd-feed {}", env!("CARGO_PKG_VERSION"));
    match CaptureWriter::create(&path, &software, adapter) {
        Ok(mut c) => {
            c.set_caps(serde_json::json!({
                "link": adapter,
                "crush": env_truthy("MFD_OBD_CRUSH"),
                "source": "ObdFeed",
            }));
            eprintln!("mfd obd: capture → {}", path.display());
            Some(c)
        }
        Err(e) => {
            eprintln!("mfd obd: capture open failed: {e}");
            None
        }
    }
}

fn cap_frame(cap: &mut Option<CaptureWriter>, dir: &str, data: &str, note: Option<&str>) {
    if let Some(c) = cap.as_mut() {
        let _ = c.log_frame(dir, "hs", data, note);
    }
}

fn cap_frame_always(cap: &mut Option<CaptureWriter>, dir: &str, data: &str, note: Option<&str>) {
    if let Some(c) = cap.as_mut() {
        let _ = c.log_frame_always(dir, "hs", data, note);
    }
}

fn set_value(t: &mut Telemetry, name: &str, value: f64) {
    // Avoid allocating a new String key every poll when the key already exists.
    if let Some(v) = t.values.get_mut(name) {
        *v = value;
    } else {
        t.values.insert(name.to_string(), value);
    }
}

fn cap_signal(
    cap: &mut Option<CaptureWriter>,
    name: &str,
    value: f64,
    unit: &str,
    mode: u8,
    pid: u8,
) {
    if let Some(c) = cap.as_mut() {
        let _ = c.log_signal(name, value, unit, mode, pid, "hs");
    }
}

fn load_dtcs(session: &mut Session, tele: &Arc<Mutex<Telemetry>>, cap: &mut Option<CaptureWriter>) {
    match session.read_all_dtcs() {
        Ok(list) => {
            for d in &list {
                cap_frame_always(cap, "rx", &d.code, Some(&format!("dtc {}", d.kind.label())));
            }
            if let Ok(mut t) = tele.lock() {
                t.dtcs = list
                    .into_iter()
                    .map(|d| DtcEntry {
                        code: d.code,
                        kind: map_kind(d.kind),
                    })
                    .collect();
                t.dtc_loaded = true;
                t.error = None;
            }
        }
        Err(e) => {
            cap_frame_always(cap, "rx", &format!("ERR:{e}"), Some("dtc"));
            if let Ok(mut t) = tele.lock() {
                let msg = e.to_string();
                if !msg.contains("NO DATA") {
                    t.error = Some(format!("DTC {msg}"));
                }
                t.dtc_loaded = true;
            }
        }
    }
}

/// Live DID entry for continuous poll (owned header + static name).
struct LiveDid {
    header: String,
    did: u16,
    name: &'static str,
}

fn run_session(
    mut session: Session,
    stop: &AtomicBool,
    tele: &Arc<Mutex<Telemetry>>,
    cap: &mut Option<CaptureWriter>,
    skip_full_probe: bool,
) -> SessionEnd {
    // Refresh adapter identity / protocol after session is live.
    if let Ok(mut t) = tele.lock() {
        let id = session.identity().to_string();
        let proto = session.protocol().to_string();
        if !id.is_empty() {
            t.adapter_id = id;
        }
        if !proto.is_empty() {
            t.protocol = proto;
        }
        t.link_phase = "BIT".into();
        t.error = None;
    }
    // Discover phase: full wire log; continuous poll samples frames (signals always).
    if let Some(c) = cap.as_mut() {
        c.set_log_all_frames(true);
    }
    let crush = env_truthy("MFD_OBD_CRUSH");

    // ── BIT / capability probe (full on first link; light re-init after reconnect) ──
    let need_probe = !skip_full_probe
        || tele
            .lock()
            .map(|t| !t.caps.ready || t.caps.page_list.is_empty())
            .unwrap_or(true);
    if need_probe {
        let mut caps = probe::run_live_probe(&mut session);
        if caps.page_list.is_empty() {
            caps.finalize_pages();
        }
        if let Ok(mut t) = tele.lock() {
            let link = format!("{} {}", t.link_kind, t.link_addr);
            caps.link = link;
            t.caps = caps;
            t.status = format!("live {}", session.name());
            t.adapter_id = session.identity().to_string();
            t.protocol = session.protocol().to_string();
            t.link_phase = "LIVE".into();
            t.error = None;
        }
    } else if let Ok(mut t) = tele.lock() {
        t.status = format!("live {}", session.name());
        t.link_phase = "LIVE".into();
        t.error = None;
        t.adapter_id = session.identity().to_string();
        t.protocol = session.protocol().to_string();
    }
    if stop.load(Ordering::Relaxed) {
        return SessionEnd::Stop;
    }

    // Mode 09 VIN + extras
    if let Ok(vin) = session.read_vin_mode09() {
        cap_frame(cap, "rx", &format!("VIN:{vin}"), Some("mode09"));
        if let Some(c) = cap.as_mut() {
            c.set_vin(&vin);
        }
        if let Ok(mut t) = tele.lock() {
            t.vin = Some(vin);
        }
    }
    for cmd in ["090A", "0904", "0906"] {
        cap_frame(cap, "tx", cmd, Some("mode09"));
        match session.request_raw(cmd) {
            Ok(b) => cap_frame(cap, "rx", &uds::hex_bytes(&b), Some("mode09")),
            Err(e) => cap_frame(cap, "rx", &format!("ERR:{e}"), Some("mode09")),
        }
    }

    load_dtcs(&mut session, tele, cap);
    let _ = ford::prepare_pcm_read(&mut session);

    // Discover Mode 01 PIDs (always when live; essential for crush)
    let mut poll_pids: Vec<u8> = session
        .discover_mode01_pids()
        .unwrap_or_else(|_| PRIORITY_PIDS.to_vec());
    for &p in PRIORITY_PIDS {
        if !poll_pids.contains(&p) {
            poll_pids.push(p);
        }
    }
    {
        let list = poll_pids
            .iter()
            .map(|p| format!("{p:02X}"))
            .collect::<Vec<_>>()
            .join(",");
        cap_frame(cap, "rx", &format!("PIDS:{list}"), Some("discover"));
        eprintln!("mfd obd: {} Mode 01 PIDs", poll_pids.len());
    }

    let mut live_dids: Vec<LiveDid> = Vec::new();
    let ford_dids: Vec<_> = ford::feed_poll_dids().collect();

    if crush {
        // Multi-module known DIDs (fast) — full range scan stays in mfd-obd-capture --crush
        let modules: &[(&str, &str)] = &[
            ("PCM", HDR_PCM),
            ("BCM", HDR_BCM),
            ("ABS", HDR_ABS),
            ("IPC", HDR_IPC),
            ("PSCM", HDR_PSCM),
        ];
        for &(mod_name, hdr) in modules {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            eprintln!("mfd obd: UDS module {mod_name} {hdr}");
            cap_frame(cap, "tx", &format!("ATSH{hdr}"), Some(mod_name));
            let _ = session.elm_mut().set_header(hdr);
            let _ = session.extended_session();
            let _ = session.tester_present();
            for &(did, name) in PROBE_DIDS {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let req = format!("22{did:04X}");
                cap_frame(cap, "tx", &req, Some(name));
                match session.read_did(hdr, did) {
                    Ok(data) => {
                        cap_frame(cap, "rx", &uds::hex_bytes(&data), Some(name));
                        live_dids.push(LiveDid {
                            header: hdr.into(),
                            did,
                            name,
                        });
                    }
                    Err(e) => {
                        cap_frame(cap, "rx", &format!("ERR:{e}"), Some(name));
                    }
                }
            }
            if hdr == HDR_PCM {
                for def in ford::probe_dids() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let _ = session.elm_mut().set_header(def.header);
                    let req = format!("22{:04X}", def.did);
                    cap_frame(cap, "tx", &req, Some(def.name));
                    match ford::read_did(&mut session, def) {
                        Ok(DecodedDid::Number { name, value, unit }) => {
                            cap_frame(cap, "rx", &format!("{value}"), Some(name));
                            cap_signal(cap, name, value, unit, 0x22, (def.did & 0xFF) as u8);
                            if let Ok(mut t) = tele.lock() {
                                t.values.insert(name.to_string(), value);
                            }
                            live_dids.push(LiveDid {
                                header: def.header.into(),
                                did: def.did,
                                name: def.name,
                            });
                        }
                        Ok(DecodedDid::Text { name, value }) => {
                            cap_frame(cap, "rx", &value, Some(name));
                            if name == "vin" && value.len() >= 11 {
                                if let Some(c) = cap.as_mut() {
                                    c.set_vin(&value);
                                }
                                if let Ok(mut t) = tele.lock() {
                                    t.vin = Some(value);
                                }
                            }
                            live_dids.push(LiveDid {
                                header: def.header.into(),
                                did: def.did,
                                name: def.name,
                            });
                        }
                        Ok(DecodedDid::Hex { name, value }) => {
                            cap_frame(cap, "rx", &value, Some(name));
                            live_dids.push(LiveDid {
                                header: def.header.into(),
                                did: def.did,
                                name: def.name,
                            });
                        }
                        Err(e) => {
                            cap_frame(cap, "rx", &format!("ERR:{e}"), Some(def.name));
                        }
                    }
                }
            }
        }
        live_dids.sort_by(|a, b| (a.header.as_str(), a.did).cmp(&(b.header.as_str(), b.did)));
        live_dids.dedup_by(|a, b| a.header == b.header && a.did == b.did);
        cap_frame(
            cap,
            "rx",
            &format!("LIVE_DIDS:{}", live_dids.len()),
            Some("discover"),
        );
        eprintln!(
            "mfd obd: crush ready — {} PIDs · {} live DIDs",
            poll_pids.len(),
            live_dids.len()
        );
    }

    if let Some(c) = cap.as_mut() {
        c.set_log_all_frames(false); // continuous: sample frames, keep signals
        let _ = c.flush();
    }

    let mut i = 0usize;
    let mut fi = 0usize;
    let mut di = 0usize;
    let mut keep = 0u32;
    let mut hard_fails = 0u32;
    let link_name = session.name().to_string();
    while !stop.load(Ordering::Relaxed) {
        keep = keep.wrapping_add(1);
        if let Some(c) = cap.as_mut() {
            c.tick_poll();
        }
        // DTC refresh less often — expensive and allocates.
        if keep % 200 == 1 {
            load_dtcs(&mut session, tele, cap);
        }
        if keep % 40 == 0 {
            cap_frame(cap, "tx", "3E80", Some("TesterPresent"));
            let _ = session.tester_present();
        }

        // Ford high-priority DIDs
        if keep % 8 == 0 && !ford_dids.is_empty() {
            let def = ford_dids[fi % ford_dids.len()];
            fi = fi.wrapping_add(1);
            let req = format!("22{:04X}", def.did);
            cap_frame(cap, "tx", &req, Some(def.name));
            match ford::read_did(&mut session, def) {
                Ok(DecodedDid::Number { name, value, unit }) => {
                    cap_frame(cap, "rx", &format!("{value}"), Some(name));
                    cap_signal(cap, name, value, unit, 0x22, (def.did & 0xFF) as u8);
                    if let Ok(mut t) = tele.lock() {
                        set_value(&mut t, name, value);
                        t.ticks = t.ticks.wrapping_add(1);
                    }
                }
                Ok(DecodedDid::Text { name: "vin", value }) => {
                    cap_frame(cap, "rx", &value, Some("vin"));
                    if value.len() >= 11 {
                        if let Some(c) = cap.as_mut() {
                            c.set_vin(&value);
                        }
                        if let Ok(mut t) = tele.lock() {
                            t.vin = Some(value);
                        }
                    }
                }
                Ok(other) => {
                    let (n, s) = match other {
                        DecodedDid::Text { name, value } => (name, value),
                        DecodedDid::Hex { name, value } => (name, value),
                        DecodedDid::Number { name, value, .. } => (name, format!("{value}")),
                    };
                    cap_frame(cap, "rx", &s, Some(n));
                }
                Err(e) => {
                    cap_frame(cap, "rx", &format!("ERR:{e}"), Some(def.name));
                }
            }
        }

        // Rotate live DIDs discovered in crush
        if crush && !live_dids.is_empty() && keep % 5 == 0 {
            let entry = &live_dids[di % live_dids.len()];
            di = di.wrapping_add(1);
            let req = format!("22{:04X}", entry.did);
            cap_frame(cap, "tx", &req, Some(entry.name));
            match session.read_did(&entry.header, entry.did) {
                Ok(data) => {
                    let hex = uds::hex_bytes(&data);
                    cap_frame(cap, "rx", &hex, Some(entry.name));
                    if let Some(&b0) = data.first() {
                        cap_signal(
                            cap,
                            entry.name,
                            b0 as f64,
                            "raw",
                            0x22,
                            (entry.did & 0xFF) as u8,
                        );
                    }
                }
                Err(e) => {
                    cap_frame(cap, "rx", &format!("ERR:{e}"), Some(entry.name));
                }
            }
        }

        // Mode 01 (primary high-rate path)
        let pid = poll_pids[i % poll_pids.len()];
        i = i.wrapping_add(1);
        let cmd = j1979::mode01_command(pid);
        cap_frame(cap, "tx", &cmd, None);
        match session.read_pid(pid) {
            Ok(v) => {
                let key: &str = if v.name == "pid_raw" {
                    // Fixed slots for raw PIDs — avoid unbounded "pid_XX" String growth.
                    // Map into a few rotating keys by pid nibble.
                    static RAW: [&str; 16] = [
                        "pid_raw_0",
                        "pid_raw_1",
                        "pid_raw_2",
                        "pid_raw_3",
                        "pid_raw_4",
                        "pid_raw_5",
                        "pid_raw_6",
                        "pid_raw_7",
                        "pid_raw_8",
                        "pid_raw_9",
                        "pid_raw_a",
                        "pid_raw_b",
                        "pid_raw_c",
                        "pid_raw_d",
                        "pid_raw_e",
                        "pid_raw_f",
                    ];
                    RAW[(v.pid as usize) & 0x0f]
                } else {
                    v.name
                };
                cap_frame(cap, "rx", &format!("OK:{:02X}", v.pid), Some(key));
                cap_signal(cap, key, v.value, v.unit, v.mode, v.pid);
                hard_fails = 0;
                if let Ok(mut t) = tele.lock() {
                    set_value(&mut t, key, v.value);
                    t.ticks = t.ticks.wrapping_add(1);
                    t.error = None;
                    t.link_phase = "LIVE".into();
                    // Avoid format! every poll — reuse status when stable.
                    if !t.status.starts_with("live ") {
                        t.status.clear();
                        t.status.push_str("live ");
                        t.status.push_str(&link_name);
                    }
                }
            }
            Err(e) => {
                // Only log continuous errors sparsely (sampled frames handle this).
                cap_frame(cap, "rx", &format!("ERR:{e}"), None);
                let msg = e.to_string();
                if is_hard_link_error(&e) {
                    hard_fails = hard_fails.saturating_add(1);
                    if hard_fails >= 8 {
                        return SessionEnd::LinkLost(msg);
                    }
                } else {
                    hard_fails = 0;
                }
                if let Ok(mut t) = tele.lock() {
                    if !msg.contains("NO DATA") {
                        t.error = Some(msg);
                    }
                }
            }
        }

        // Flush at most ~every few seconds (BufWriter + FLUSH_EVERY also apply).
        if keep % 500 == 0 {
            if let Some(c) = cap.as_mut() {
                let _ = c.flush();
            }
        }
        thread::sleep(Duration::from_millis(if crush { 15 } else { 25 }));
    }

    SessionEnd::Stop
}

fn is_hard_link_error(e: &ObdError) -> bool {
    matches!(
        e,
        ObdError::Io(_)
            | ObdError::NotOpen
            | ObdError::Timeout
            | ObdError::Adapter(_)
            | ObdError::Serial(_)
    )
}

fn finish_cap(cap: Option<CaptureWriter>) {
    if let Some(c) = cap {
        match c.finish() {
            Ok(dir) => eprintln!(
                "mfd obd: capture closed → {} (frames.ndjson signals.csv meta.toml session.json)",
                dir.display()
            ),
            Err(e) => eprintln!("mfd obd: capture finish: {e}"),
        }
    }
}

fn assign_str(dst: &mut String, src: &str) {
    if dst != src {
        dst.clear();
        dst.push_str(src);
    }
}

fn apply_telemetry(t: &Telemetry, v: &mut VehicleSnapshot) {
    // Bus block: copy only when changed (avoids heap churn every frame).
    assign_str(
        &mut v.bus_kind,
        if t.link_kind.is_empty() {
            "OBD"
        } else {
            &t.link_kind
        },
    );
    assign_str(&mut v.bus_addr, &t.link_addr);
    assign_str(
        &mut v.bus_channel,
        if t.link_channel.is_empty() {
            "-"
        } else {
            &t.link_channel
        },
    );
    assign_str(
        &mut v.bus_adapter,
        if t.adapter_id.is_empty() {
            "—"
        } else {
            &t.adapter_id
        },
    );
    assign_str(
        &mut v.bus_proto,
        if t.protocol.is_empty() {
            "—"
        } else {
            &t.protocol
        },
    );
    v.bus_ticks = t.ticks;
    let cap = t
        .capture_dir
        .as_ref()
        .map(|p| short_path(p))
        .unwrap_or_default();
    assign_str(&mut v.bus_capture, &cap);
    let state = match t.link_phase.as_str() {
        "SEARCH" => "SEARCH",
        "RECONN" => "RECONN",
        "CONN" => "CONN",
        _ if !t.caps.ready => "BIT",
        _ if t.error.is_some() => "ERR",
        _ => "LIVE",
    };
    assign_str(&mut v.bus_state, state);
    if let Some(e) = &t.error {
        assign_str(&mut v.bus_error, e);
    } else if !v.bus_error.is_empty() {
        v.bus_error.clear();
    }

    if let Some(rpm) = t.values.get("engine_rpm") {
        v.rpm = *rpm as f32;
    }
    if let Some(kmh) = t.values.get("vehicle_speed") {
        v.speed_mph = (*kmh as f32) / 1.60934;
    }
    if let Some(th) = t.values.get("throttle") {
        v.throttle = (*th as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(th) = t.values.get("accel_pedal") {
        v.throttle = (*th as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(load) = t.values.get("engine_load") {
        v.load = (*load as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("coolant_temp") {
        v.coolant_c = *c as f32;
        v.coolant = ((*c as f32 + 40.0) / 160.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("coolant_temp_c") {
        v.coolant_c = *c as f32;
        v.coolant = ((*c as f32 + 40.0) / 160.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("intake_temp") {
        v.iat_c = *c as f32;
        v.iat = ((*c as f32 + 40.0) / 120.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("intake_temp_c") {
        v.iat_c = *c as f32;
        v.iat = ((*c as f32 + 40.0) / 120.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("oil_temp") {
        v.oil_temp_c = *c as f32;
        v.oil_temp = ((*c as f32 + 40.0) / 160.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("oil_temp_c") {
        v.oil_temp_c = *c as f32;
        v.oil_temp = ((*c as f32 + 40.0) / 160.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("trans_temp_c") {
        v.trans_temp_c = *c as f32;
        v.trans_temp = ((*c as f32 + 40.0) / 160.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("ambient_temp_c") {
        v.temp_out_c = *c as f32;
    }
    if let Some(c) = t.values.get("ambient_temp") {
        v.temp_out_c = *c as f32;
    }
    if let Some(f) = t.values.get("fuel_level_pct") {
        v.fuel = (*f as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(f) = t.values.get("fuel_level") {
        v.fuel = (*f as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(fp) = t.values.get("fuel_pressure") {
        v.fuel_pressure_kpa = *fp as f32;
    }
    if let Some(fp) = t.values.get("fuel_rail_pressure") {
        v.fuel_pressure_kpa = *fp as f32;
    }
    if let Some(volt) = t.values.get("battery_v") {
        v.battery_v = *volt as f32;
        v.battery = (((*volt as f32) - 11.0) / 4.0).clamp(0.0, 1.0);
    }
    if let Some(volt) = t.values.get("control_module_voltage") {
        v.battery_v = *volt as f32;
        v.battery = (((*volt as f32) - 11.0) / 4.0).clamp(0.0, 1.0);
    }
    if let Some(maf) = t.values.get("maf") {
        v.maf_gps = *maf as f32;
        v.maf = ((*maf as f32) / 100.0).clamp(0.0, 1.0);
    }
    if t.dtc_loaded {
        // Clone DTC list only when length/content may change (cheap check).
        if v.dtcs.len() != t.dtcs.len()
            || v.dtcs
                .iter()
                .zip(t.dtcs.iter())
                .any(|(a, b)| a.code != b.code)
        {
            v.dtcs = t.dtcs.clone();
        }
        v.dtc_count = t.dtcs.len() as u32;
    }
    if let Some(vin) = &t.vin {
        if !vin.is_empty() {
            assign_str(&mut v.vin, vin);
        }
    }
}
