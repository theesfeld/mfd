//! Caution / warning logic for CMFD glass + speaker.
//!
//! **Visual:** flash (red field) on discrete alerts.  
//! **Aural:** BINGO (low fuel), ALERT (master-class), caution chirps.
//!
//! Hardware: same tone PCM can feed a device speaker (ALSA / I2S later).

use crate::audio::{self, Callout};
use crate::auto::{AutoPage, VehicleSnapshot};
use std::time::Instant;

/// Fuel fraction that triggers **BINGO** (classic low-fuel callout).
pub const BINGO_FUEL: f32 = 0.15;
/// Speed above which park brake ON is a serious alert (mph).
pub const PARK_BRAKE_SPEED_MPH: f32 = 2.0;
/// Battery below this (V) → caution.
pub const LOW_BATT_V: f32 = 12.0;

/// One active warning for glass + audio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveWarn {
    pub id: WarnId,
    pub label: &'static str,
    pub level: WarnLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WarnId {
    Bingo,
    ParkBrake,
    TireAlert,
    DoorAjar,
    LowBattery,
    DtcPresent,
    MasterCaution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarnLevel {
    /// Amber-class
    Caution,
    /// Red / aural priority
    Warning,
}

/// Evaluate snapshot → active list (order = priority).
pub fn evaluate(v: &VehicleSnapshot) -> Vec<ActiveWarn> {
    let mut out = Vec::new();
    if v.fuel <= BINGO_FUEL {
        out.push(ActiveWarn {
            id: WarnId::Bingo,
            label: "BINGO FUEL",
            level: WarnLevel::Warning,
        });
    }
    if v.park_brake && v.speed_mph > PARK_BRAKE_SPEED_MPH {
        out.push(ActiveWarn {
            id: WarnId::ParkBrake,
            label: "PARK BRAKE",
            level: WarnLevel::Warning,
        });
    } else if v.park_brake && v.speed_mph <= PARK_BRAKE_SPEED_MPH {
        // Still show flash when parked with brake on (reminder, caution)
        out.push(ActiveWarn {
            id: WarnId::ParkBrake,
            label: "PARK BRAKE",
            level: WarnLevel::Caution,
        });
    }
    if v.tire_fl.alert || v.tire_fr.alert || v.tire_rl.alert || v.tire_rr.alert {
        out.push(ActiveWarn {
            id: WarnId::TireAlert,
            label: "TIRE PRESS",
            level: WarnLevel::Warning,
        });
    }
    if (!v.door_fl || !v.door_fr || !v.door_rl || !v.door_rr || !v.door_hatch) && v.speed_mph > 1.0
    {
        out.push(ActiveWarn {
            id: WarnId::DoorAjar,
            label: "DOOR AJAR",
            level: WarnLevel::Warning,
        });
    }
    if v.battery_v > 0.1 && v.battery_v < LOW_BATT_V {
        out.push(ActiveWarn {
            id: WarnId::LowBattery,
            label: "LOW BATT",
            level: WarnLevel::Caution,
        });
    }
    if v.dtc_count > 0 {
        out.push(ActiveWarn {
            id: WarnId::DtcPresent,
            label: "FAULT",
            level: WarnLevel::Caution,
        });
    }
    out
}

/// Format that **owns** this warn for slot-flash / local detail (Lockheed PDR).
///
/// `None` = no format-slot flash (use master strip + DTC path only).
pub fn owning_format(id: WarnId) -> Option<AutoPage> {
    match id {
        WarnId::Bingo => Some(AutoPage::Fuel),
        WarnId::ParkBrake => Some(AutoPage::Drive),
        WarnId::TireAlert => Some(AutoPage::Chas),
        WarnId::DoorAjar => Some(AutoPage::Body),
        WarnId::LowBattery => Some(AutoPage::Elec),
        WarnId::DtcPresent | WarnId::MasterCaution => None,
    }
}

/// Highest-priority warning that may flash a **format slot** when off-glass.
/// Warning-class only; caution (DTC, low batt) never slot-flash.
pub fn slot_flash_owner(warns: &[ActiveWarn]) -> Option<AutoPage> {
    let mut best: Option<(i32, AutoPage)> = None;
    for w in warns {
        if w.level != WarnLevel::Warning {
            continue;
        }
        let Some(page) = owning_format(w.id) else {
            continue;
        };
        let pri = match w.id {
            WarnId::ParkBrake => 50,
            WarnId::DoorAjar => 40,
            WarnId::TireAlert => 30,
            WarnId::Bingo => 20,
            _ => 10,
        };
        if best.map(|(p, _)| pri > p).unwrap_or(true) {
            best = Some((pri, page));
        }
    }
    best.map(|(_, p)| p)
}

/// Flash phase: true = “on” (show red field). ~2 Hz.
pub fn flash_on(t_secs: f32) -> bool {
    (t_secs * 2.0).fract() < 0.5
}

/// Faster flash for warning-level (~3.5 Hz).
pub fn flash_warn_on(t_secs: f32) -> bool {
    (t_secs * 3.5).fract() < 0.5
}

/// Runtime engine: rate-limit aural callouts.
pub struct WarningEngine {
    last_bingo: Option<Instant>,
    last_alert: Option<Instant>,
    last_caution: Option<Instant>,
    bingo_period: std::time::Duration,
    alert_period: std::time::Duration,
    caution_period: std::time::Duration,
    audio_enabled: bool,
}

impl Default for WarningEngine {
    fn default() -> Self {
        Self {
            last_bingo: None,
            last_alert: None,
            last_caution: None,
            bingo_period: std::time::Duration::from_secs(8),
            alert_period: std::time::Duration::from_secs(4),
            caution_period: std::time::Duration::from_secs(12),
            audio_enabled: std::env::var("MFD_AUDIO")
                .map(|v| v != "0" && v != "off")
                .unwrap_or(true),
        }
    }
}

impl WarningEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable speaker (lab without audio).
    pub fn set_audio(&mut self, on: bool) {
        self.audio_enabled = on;
    }

    /// Tick: return active warns; fire aural as needed.
    pub fn tick(&mut self, v: &VehicleSnapshot) -> Vec<ActiveWarn> {
        let active = evaluate(v);
        if !self.audio_enabled || active.is_empty() {
            return active;
        }
        let now = Instant::now();
        let has_bingo = active.iter().any(|w| w.id == WarnId::Bingo);
        let has_warn = active
            .iter()
            .any(|w| w.level == WarnLevel::Warning && w.id != WarnId::Bingo);
        let has_caut = active.iter().any(|w| w.level == WarnLevel::Caution);

        if has_bingo {
            let due = self
                .last_bingo
                .map(|t| now.duration_since(t) >= self.bingo_period)
                .unwrap_or(true);
            if due {
                audio::play(Callout::Bingo);
                self.last_bingo = Some(now);
            }
        }
        if has_warn {
            let due = self
                .last_alert
                .map(|t| now.duration_since(t) >= self.alert_period)
                .unwrap_or(true);
            if due {
                audio::play(Callout::Alert);
                self.last_alert = Some(now);
            }
        } else if has_caut {
            let due = self
                .last_caution
                .map(|t| now.duration_since(t) >= self.caution_period)
                .unwrap_or(true);
            if due {
                audio::play(Callout::Caution);
                self.last_caution = Some(now);
            }
        }
        active
    }
}

/// True if this label should use red flash field right now.
pub fn label_should_flash(label: &str, active: &[ActiveWarn], t: f32) -> bool {
    let flash = flash_warn_on(t);
    if !flash {
        return false;
    }
    for w in active {
        match w.id {
            WarnId::ParkBrake if label.contains("PARK") || label == "PARK" => return true,
            WarnId::Bingo if label.contains("FUEL") || label.contains("BINGO") => return true,
            WarnId::TireAlert
                if label.contains("TIRE") || label.starts_with("FL") || label.starts_with("FR") =>
            {
                return true
            }
            WarnId::DoorAjar if label.starts_with("DR") || label == "HATCH" => return true,
            WarnId::LowBattery if label.contains("BATT") => return true,
            WarnId::DtcPresent if label.contains("FAULT") || label.contains("DTC") => return true,
            _ => {}
        }
    }
    false
}
