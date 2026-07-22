//! Ford F-150 (P552-class) **read-only** DID catalog and decode helpers.
//!
//! ## Data sources
//! - Live DID table: `docs/reference/ford-f150-forscan/live_parameters.csv`
//! - Module list: `docs/reference/ford-f150-forscan/modules_can.csv`
//! - FORScan **As-Built** export (config addresses, not live PIDs), **2019-only**:  
//!   `docs/reference/ford-f150-forscan/*.csv`
//! - Protocol: `docs/reference/ford-f150-uds-readonly.md`
//!
//! DIDs marked for verify must be confirmed on the live truck.  
//! **Display-only:** never write DIDs or As-Built.

use crate::obd::error::{Error, Result};
use crate::obd::session::Session;
use crate::obd::uds;

/// One known or candidate DID for glass.
#[derive(Clone, Copy, Debug)]
pub struct DidDef {
    pub did: u16,
    pub name: &'static str,
    pub header: &'static str,
    pub scale: DidScale,
    pub unit: &'static str,
    /// Poll weight: 0 = rare/discovery, 1 = medium, 2 = high.
    pub priority: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum DidScale {
    /// `value = (b0 as f64 + add) * mul`
    U8AddMul { add: f64, mul: f64 },
    /// Big-endian u16: `(b0<<8|b1) as f64 * mul + add`
    U16Be { mul: f64, add: f64 },
    /// `b0 * mul` (e.g. fuel %)
    U8Mul { mul: f64 },
    /// ASCII string (VIN-class)
    Ascii,
    /// Raw bytes as hex
    Raw,
}

pub const HDR_PCM: &str = "7E0";
pub const HDR_FUNC: &str = "7DF";
pub const HDR_ABS: &str = "760";
pub const HDR_BCM: &str = "726";
pub const HDR_IPC: &str = "720";
pub const HDR_PSCM: &str = "730";

/// F-150 class catalog (community + user table). Verify on truck.
pub const F150_DIDS: &[DidDef] = &[
    // ── Identity ──────────────────────────────────────────────────────────
    DidDef {
        did: 0xF190,
        name: "vin",
        header: HDR_PCM,
        scale: DidScale::Ascii,
        unit: "",
        priority: 0,
    },
    // ── Temps (PCM) ───────────────────────────────────────────────────────
    DidDef {
        did: 0xF405,
        name: "coolant_temp_c",
        header: HDR_PCM,
        scale: DidScale::U8AddMul {
            add: -40.0,
            mul: 1.0,
        },
        unit: "C",
        priority: 2,
    },
    DidDef {
        did: 0xF40F,
        name: "intake_temp_c",
        header: HDR_PCM,
        scale: DidScale::U8AddMul {
            add: -40.0,
            mul: 1.0,
        },
        unit: "C",
        priority: 1,
    },
    // Oil/ambient UDS often NRC 31 on this PCM — Mode 01 0x5C / 0x46 preferred.
    DidDef {
        did: 0xF45C,
        name: "oil_temp_c",
        header: HDR_PCM,
        scale: DidScale::U8AddMul {
            add: -40.0,
            mul: 1.0,
        },
        unit: "C",
        priority: 0,
    },
    DidDef {
        did: 0xF457,
        name: "ambient_temp_c",
        header: HDR_PCM,
        scale: DidScale::U8AddMul {
            add: -40.0,
            mul: 1.0,
        },
        unit: "C",
        priority: 0,
    },
    // Works on truck capture (≈3–60 °C early drive).
    DidDef {
        did: 0x1E1C,
        name: "trans_temp_c",
        header: HDR_PCM,
        scale: DidScale::U16Be {
            mul: 1.0 / 16.0,
            add: 0.0,
        },
        unit: "C",
        priority: 1,
    },
    // ── Fuel / battery ────────────────────────────────────────────────────
    // F41F returned 0–2% on MX+ capture — not trustworthy; use Mode 01 0x2F.
    DidDef {
        did: 0xF41F,
        name: "fuel_level_pct",
        header: HDR_PCM,
        scale: DidScale::U8Mul { mul: 100.0 / 255.0 },
        unit: "%",
        priority: 0,
    },
    // 402C NRC 31 on capture — use Mode 01 0x42 control_module_voltage.
    DidDef {
        did: 0x402C,
        name: "battery_v",
        header: HDR_PCM,
        scale: DidScale::U8Mul { mul: 0.1 },
        unit: "V",
        priority: 0,
    },
    // ── Gear (raw until PRNDL map verified) ───────────────────────────────
    DidDef {
        did: 0x1E12,
        name: "gear_raw",
        header: HDR_PCM,
        scale: DidScale::U8Mul { mul: 1.0 },
        unit: "",
        priority: 1,
    },
    // ── ABS / body — NO DATA on morning capture; demote (verify later) ────
    DidDef {
        did: 0x2B00,
        name: "brake_park_raw",
        header: HDR_ABS,
        scale: DidScale::Raw,
        unit: "",
        priority: 0,
    },
    DidDef {
        did: 0x2B06,
        name: "wheel_speed_seed_raw",
        header: HDR_ABS,
        scale: DidScale::Raw,
        unit: "",
        priority: 0,
    },
    DidDef {
        did: 0x2813,
        name: "steer_or_wheels_raw",
        header: HDR_PSCM,
        scale: DidScale::Raw,
        unit: "",
        priority: 0,
    },
    DidDef {
        did: 0x03DC,
        name: "fuel_pressure_raw",
        header: HDR_PCM,
        scale: DidScale::Raw,
        unit: "",
        priority: 0,
    },
];

/// DIDs to cycle in the live feed (priority ≥ 1). Dead channels stay priority 0.
pub fn feed_poll_dids() -> impl Iterator<Item = &'static DidDef> {
    F150_DIDS.iter().filter(|d| d.priority >= 1)
}

/// All DIDs for capture / discovery.
pub fn probe_dids() -> &'static [DidDef] {
    F150_DIDS
}

/// Decode DID data payload (bytes after `62 DID_H DID_L`).
pub fn decode_data(def: &DidDef, data: &[u8]) -> Result<DecodedDid> {
    match def.scale {
        DidScale::U8AddMul { add, mul } => {
            let b0 = *data
                .first()
                .ok_or_else(|| Error::Decode(format!("{} empty", def.name)))?;
            Ok(DecodedDid::Number {
                name: def.name,
                value: (b0 as f64 + add) * mul,
                unit: def.unit,
            })
        }
        DidScale::U16Be { mul, add } => {
            if data.len() < 2 {
                return Err(Error::Decode(format!("{} short u16", def.name)));
            }
            let raw = u16::from_be_bytes([data[0], data[1]]) as f64;
            Ok(DecodedDid::Number {
                name: def.name,
                value: raw * mul + add,
                unit: def.unit,
            })
        }
        DidScale::U8Mul { mul } => {
            let b0 = *data
                .first()
                .ok_or_else(|| Error::Decode(format!("{} empty", def.name)))?;
            Ok(DecodedDid::Number {
                name: def.name,
                value: b0 as f64 * mul,
                unit: def.unit,
            })
        }
        DidScale::Ascii => {
            let s: String = data
                .iter()
                .filter(|b| b.is_ascii_graphic() || **b == b' ')
                .map(|b| *b as char)
                .collect::<String>()
                .trim()
                .to_string();
            Ok(DecodedDid::Text {
                name: def.name,
                value: s,
            })
        }
        DidScale::Raw => Ok(DecodedDid::Hex {
            name: def.name,
            value: uds::hex_bytes(data),
        }),
    }
}

#[derive(Clone, Debug)]
pub enum DecodedDid {
    Number {
        name: &'static str,
        value: f64,
        unit: &'static str,
    },
    Text {
        name: &'static str,
        value: String,
    },
    Hex {
        name: &'static str,
        value: String,
    },
}

impl DecodedDid {
    pub fn name(&self) -> &str {
        match self {
            DecodedDid::Number { name, .. }
            | DecodedDid::Text { name, .. }
            | DecodedDid::Hex { name, .. } => name,
        }
    }
}

/// Extended session + read one DID on its module header.
pub fn read_did(session: &mut Session, def: &DidDef) -> Result<DecodedDid> {
    let data = session.read_did(def.header, def.did)?;
    decode_data(def, &data)
}

/// Enter extended session on PCM and keep-alive (read path).
pub fn prepare_pcm_read(session: &mut Session) -> Result<()> {
    session.elm_mut().set_header(HDR_PCM)?;
    let _ = session.extended_session();
    let _ = session.tester_present();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ect_decode() {
        let def = F150_DIDS.iter().find(|d| d.did == 0xF405).unwrap();
        let d = decode_data(def, &[0x5A]).unwrap();
        match d {
            // 0x5A=90; (90-40)=50 °C
            DecodedDid::Number { value, unit, .. } => {
                assert!((value - 50.0).abs() < 0.01);
                assert_eq!(unit, "C");
            }
            _ => panic!("expected number"),
        }
    }

    #[test]
    fn catalog_has_core() {
        assert!(F150_DIDS.iter().any(|d| d.did == 0xF190));
        assert!(F150_DIDS.iter().any(|d| d.did == 0xF405));
        assert!(F150_DIDS.iter().any(|d| d.did == 0x1E1C));
        assert!(F150_DIDS.iter().any(|d| d.did == 0x2B00));
    }

    #[test]
    fn live_parameters_csv_exists() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/reference/ford-f150-forscan/live_parameters.csv");
        assert!(p.exists(), "missing {p:?}");
        let text = std::fs::read_to_string(p).unwrap();
        assert!(text.contains("22F405"));
        assert!(text.contains("j1979"));
    }

    #[test]
    fn forscan_index_exists() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/reference/ford-f150-forscan/INDEX.md");
        assert!(p.exists());
    }
}
