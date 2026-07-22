//! Background poller → [`VehicleSnapshot`](crate::auto::VehicleSnapshot).
//!
//! Startup: capability probe (BIT), then DTC + Mode 01 / Ford DID poll.
//! Read-only — never Mode 04 clear.

use crate::auto::caps::VehicleCaps;
use crate::auto::probe;
use crate::auto::{DtcEntry, DtcKind, VehicleSnapshot};
use crate::obd::j1979::{self, PRIORITY_PIDS};
use crate::obd::session::Session;
use std::collections::HashMap;
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
}

/// Background OBD poller (native stack).
pub struct ObdFeed {
    stop: Arc<AtomicBool>,
    tele: Arc<Mutex<Telemetry>>,
    _join: Option<JoinHandle<()>>,
}

impl ObdFeed {
    /// Start if `MFD_OBD_PORT`, `MFD_OBD_BT`, or `MFD_OBD_REPLAY` is set.
    pub fn try_start_from_env() -> Option<Self> {
        match Session::from_env() {
            Ok(Some(session)) => Self::from_session(session).ok(),
            Ok(None) => None,
            Err(e) => {
                eprintln!("mfd obd: {e}");
                None
            }
        }
    }

    pub fn from_session(session: Session) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let tele = Arc::new(Mutex::new(Telemetry {
            status: format!("probe {}", session.name()),
            caps: VehicleCaps {
                phase: "POWER ON".into(),
                link: session.name().into(),
                ..Default::default()
            },
            ..Default::default()
        }));
        let stop_t = Arc::clone(&stop);
        let tele_t = Arc::clone(&tele);

        let join = thread::Builder::new()
            .name("mfd-obd".into())
            .spawn(move || run_loop(session, stop_t, tele_t))
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
                if !t.caps.ready {
                    format!("BIT {:.0}% {}", t.caps.progress * 100.0, t.caps.phase)
                } else if let Some(e) = &t.error {
                    format!("OBD ERR {e}")
                } else {
                    let d = if t.dtc_loaded {
                        format!(" DTC{}", t.dtcs.len())
                    } else {
                        String::new()
                    };
                    format!("{} · t{}{d}", t.status, t.ticks)
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

fn map_kind(k: j1979::DtcKind) -> DtcKind {
    match k {
        j1979::DtcKind::Stored => DtcKind::Stored,
        j1979::DtcKind::Pending => DtcKind::Pending,
        j1979::DtcKind::Permanent => DtcKind::Permanent,
    }
}

fn load_dtcs(session: &mut Session, tele: &Arc<Mutex<Telemetry>>) {
    match session.read_all_dtcs() {
        Ok(list) => {
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

fn run_loop(mut session: Session, stop: Arc<AtomicBool>, tele: Arc<Mutex<Telemetry>>) {
    // ── BIT / capability probe first ──────────────────────────────────────
    let caps = probe::run_live_probe(&mut session);
    if let Ok(mut t) = tele.lock() {
        t.caps = caps;
        t.status = format!("live {}", session.name());
    }
    if stop.load(Ordering::Relaxed) {
        return;
    }

    load_dtcs(&mut session, &tele);

    if let Ok(vin) = session.read_vin_mode09() {
        if let Ok(mut t) = tele.lock() {
            t.vin = Some(vin);
        }
    }
    let _ = crate::obd::ford::prepare_pcm_read(&mut session);

    let ford_dids: Vec<_> = crate::obd::ford::feed_poll_dids().collect();
    let mut i = 0usize;
    let mut fi = 0usize;
    let mut keep = 0u32;
    while !stop.load(Ordering::Relaxed) {
        keep += 1;
        if keep % 50 == 1 {
            load_dtcs(&mut session, &tele);
        }
        if keep % 40 == 0 {
            let _ = session.tester_present();
        }
        if keep % 8 == 0 && !ford_dids.is_empty() {
            let def = ford_dids[fi % ford_dids.len()];
            fi = fi.wrapping_add(1);
            match crate::obd::ford::read_did(&mut session, def) {
                Ok(crate::obd::ford::DecodedDid::Number { name, value, .. }) => {
                    if let Ok(mut t) = tele.lock() {
                        t.values.insert(name.to_string(), value);
                        t.ticks = t.ticks.wrapping_add(1);
                    }
                }
                Ok(crate::obd::ford::DecodedDid::Text { name: "vin", value }) => {
                    if let Ok(mut t) = tele.lock() {
                        if value.len() >= 11 {
                            t.vin = Some(value);
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }
        let pid = PRIORITY_PIDS[i % PRIORITY_PIDS.len()];
        i = i.wrapping_add(1);
        match session.read_pid(pid) {
            Ok(v) => {
                if let Ok(mut t) = tele.lock() {
                    t.values.insert(v.name.to_string(), v.value);
                    t.ticks = t.ticks.wrapping_add(1);
                    t.error = None;
                    t.status = format!("live {}", session.name());
                }
            }
            Err(e) => {
                if let Ok(mut t) = tele.lock() {
                    let msg = e.to_string();
                    if !msg.contains("NO DATA") {
                        t.error = Some(msg);
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn apply_telemetry(t: &Telemetry, v: &mut VehicleSnapshot) {
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
        v.dtcs = t.dtcs.clone();
        v.dtc_count = t.dtcs.len() as u32;
    }
    if let Some(vin) = &t.vin {
        if !vin.is_empty() {
            v.vin = vin.clone();
        }
    }
}
