//! Live **OBD-II** feed → [`VehicleSnapshot`] (optional `obd` feature).
//!
//! Uses path dependency on `obd-io` (theesfeld/obdtui). Env:
//! - `MFD_OBD_PORT=/dev/ttyUSB0` — ELM327/STN serial
//! - `MFD_OBD_REPLAY=path.jsonl|csv` — capture replay
//! - `MFD_OBD_BAUD=115200`

#![cfg(feature = "obd")]

use crate::auto::VehicleSnapshot;
use obd_io::{
    connect, generic_profile, priority_pids, ConnectOptions, ReplayTransport, VehicleSession,
};
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
}

/// Background OBD poller.
pub struct ObdFeed {
    stop: Arc<AtomicBool>,
    tele: Arc<Mutex<Telemetry>>,
    _join: Option<JoinHandle<()>>,
}

impl ObdFeed {
    /// Start if `MFD_OBD_PORT` or `MFD_OBD_REPLAY` is set.
    pub fn try_start_from_env() -> Option<Self> {
        let replay = std::env::var_os("MFD_OBD_REPLAY").map(PathBuf::from);
        let port = std::env::var("MFD_OBD_PORT").ok().filter(|s| !s.is_empty());
        if replay.is_none() && port.is_none() {
            return None;
        }
        Self::start(replay, port, None).ok()
    }

    pub fn start(
        replay: Option<PathBuf>,
        port: Option<String>,
        baud: Option<u32>,
    ) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let tele = Arc::new(Mutex::new(Telemetry {
            status: "connecting".into(),
            ..Default::default()
        }));
        let stop_t = Arc::clone(&stop);
        let tele_t = Arc::clone(&tele);
        let baud = baud.or_else(|| {
            std::env::var("MFD_OBD_BAUD")
                .ok()
                .and_then(|s| s.parse().ok())
        });

        let join = thread::Builder::new()
            .name("mfd-obd".into())
            .spawn(move || {
                let session = match open_session(replay, port, baud) {
                    Ok(s) => s,
                    Err(e) => {
                        if let Ok(mut t) = tele_t.lock() {
                            t.error = Some(e);
                            t.status = "fail".into();
                        }
                        return;
                    }
                };
                run_loop(session, stop_t, tele_t);
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

    pub fn status_line(&self) -> String {
        self.tele
            .lock()
            .map(|t| {
                if let Some(e) = &t.error {
                    format!("OBD ERR {e}")
                } else {
                    format!("{} · t{}", t.status, t.ticks)
                }
            })
            .unwrap_or_else(|_| "OBD lock".into())
    }
}

impl Drop for ObdFeed {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self._join.take() {
            let _ = j.join();
        }
    }
}

fn open_session(
    replay: Option<PathBuf>,
    port: Option<String>,
    baud: Option<u32>,
) -> Result<VehicleSession, String> {
    let software = format!("mfd {}", env!("CARGO_PKG_VERSION"));
    let profile = generic_profile();
    if let Some(path) = replay {
        let transport = ReplayTransport::from_path(&path).map_err(|e| format!("replay: {e}"))?;
        let mut session = VehicleSession::new(Box::new(transport), profile, software);
        session.init().map_err(|e| e.to_string())?;
        let _ = session.read_vin();
        let _ = session.probe_supported_pids();
        return Ok(session);
    }
    let opts = ConnectOptions {
        path: port,
        baud,
        prefer: obd_io::LinkPrefer::Usb,
        bt_mac: None,
        rfcomm_index: 0,
        rfcomm_channel: 1,
        timeout: Duration::from_millis(800),
        default_bus: obd_io::BusTag::Hs,
        skip_init: false,
    };
    let connected = connect(opts).map_err(|e| e.to_string())?;
    let mut session = VehicleSession::new(connected.transport, profile, software);
    let _ = session.read_vin();
    let _ = session.probe_supported_pids();
    Ok(session)
}

fn run_loop(mut session: VehicleSession, stop: Arc<AtomicBool>, tele: Arc<Mutex<Telemetry>>) {
    {
        if let Ok(mut t) = tele.lock() {
            t.vin = session.vin.clone();
            t.status = format!("live {}", session.transport_name());
        }
    }
    let mut order: Vec<u8> = priority_pids().to_vec();
    for &p in &[0x2F_u8, 0x42, 0x5C, 0x0F, 0x46, 0x10] {
        if !order.contains(&p) {
            order.push(p);
        }
    }
    let mut i = 0usize;
    while !stop.load(Ordering::Relaxed) {
        let pid = order[i % order.len()];
        i = i.wrapping_add(1);
        match session.read_pid(pid) {
            Ok(v) => {
                if let Ok(mut t) = tele.lock() {
                    t.values.insert(v.name.clone(), v.value);
                    t.ticks = t.ticks.wrapping_add(1);
                    t.error = None;
                    t.vin = session.vin.clone();
                    t.status = format!("live {}", session.transport_name());
                }
                let _ = v;
            }
            Err(e) => {
                if let Ok(mut t) = tele.lock() {
                    t.error = Some(e.to_string());
                }
            }
        }
        thread::sleep(Duration::from_millis(15));
    }
}

fn apply_telemetry(t: &Telemetry, v: &mut VehicleSnapshot) {
    // Names from obd-io standard_pid_defs
    if let Some(rpm) = t.values.get("engine_rpm") {
        v.rpm = *rpm as f32;
    }
    if let Some(kmh) = t.values.get("vehicle_speed") {
        v.speed_mph = (*kmh as f32) / 1.60934;
    }
    if let Some(th) = t.values.get("throttle") {
        v.throttle = (*th as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(load) = t.values.get("engine_load") {
        v.load = (*load as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("coolant_temp") {
        // normalize ~ -40..215 C → 0..1 around operating band
        v.coolant = ((*c as f32 + 40.0) / 160.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("intake_temp") {
        v.iat = ((*c as f32 + 40.0) / 120.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("oil_temp") {
        v.oil_temp = ((*c as f32 + 40.0) / 160.0).clamp(0.0, 1.0);
    }
    if let Some(c) = t.values.get("ambient_temp") {
        v.temp_out_c = *c as f32;
    }
    if let Some(f) = t.values.get("fuel_level") {
        v.fuel = (*f as f32 / 100.0).clamp(0.0, 1.0);
    }
    if let Some(volt) = t.values.get("control_module_voltage") {
        // 11..15 V → 0..1
        v.battery = (((*volt as f32) - 11.0) / 4.0).clamp(0.0, 1.0);
    }
    if let Some(maf) = t.values.get("maf") {
        v.maf = ((*maf as f32) / 100.0).clamp(0.0, 1.0);
    }
}
