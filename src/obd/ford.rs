//! Ford F-150 (P552-class) **read-only** DID catalog and decode helpers.
//!
//! DIDs are community / reverse-engineering **hints**. Marked scaling must be
//! confirmed on the live truck. See `docs/reference/ford-f150-uds-readonly.md`.
//!
//! Display-only: never write DIDs.

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
    /// Poll weight: 0 = rare, 1 = medium, 2 = often (with Mode 01).
    pub priority: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum DidScale {
    /// Single byte: `value = b0 as f64 + add` then `* mul` (e.g. temp: add=-40, mul=1).
    U8AddMul { add: f64, mul: f64 },
    /// Big-endian u16: `(b0<<8|b1) as f64 * mul + add`.
    U16Be { mul: f64, add: f64 },
    /// ASCII string (VIN-class).
    Ascii,
    /// Raw bytes — store as hex only.
    Raw,
}

/// PCM physical header (11-bit).
pub const HDR_PCM: &str = "7E0";
/// Functional broadcast.
pub const HDR_FUNC: &str = "7DF";

/// Catalog for F-150 class (verify per vehicle).
pub const F150_DIDS: &[DidDef] = &[
    DidDef {
        did: 0xF190,
        name: "vin",
        header: HDR_PCM,
        scale: DidScale::Ascii,
        priority: 0,
    },
    DidDef {
        did: 0xF405,
        name: "coolant_temp_c",
        header: HDR_PCM,
        scale: DidScale::U8AddMul {
            add: -40.0,
            mul: 1.0,
        },
        priority: 1,
    },
    DidDef {
        did: 0xF40F,
        name: "intake_temp_c",
        header: HDR_PCM,
        scale: DidScale::U8AddMul {
            add: -40.0,
            mul: 1.0,
        },
        priority: 1,
    },
    DidDef {
        did: 0x1E1C,
        name: "trans_temp_c",
        header: HDR_PCM,
        // Community often uses /16 on 16-bit — confirm on truck.
        scale: DidScale::U16Be {
            mul: 1.0 / 16.0,
            add: 0.0,
        },
        priority: 1,
    },
    // Probe-only candidates (raw until scaled with capture)
    DidDef {
        did: 0x2B00,
        name: "brake_park_raw",
        header: HDR_PCM,
        scale: DidScale::Raw,
        priority: 0,
    },
    DidDef {
        did: 0x2813,
        name: "steer_or_wheels_raw",
        header: HDR_PCM,
        scale: DidScale::Raw,
        priority: 0,
    },
];

/// DIDs to cycle in the live feed (medium priority and up).
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
            let v = (b0 as f64 + add) * mul;
            Ok(DecodedDid::Number {
                name: def.name,
                value: v,
                unit: "C",
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
                unit: "C",
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
            // 0x5A = 90 raw; OBD formula (x - 40) = 50 °C
            DecodedDid::Number { value, .. } => assert!((value - 50.0).abs() < 0.01),
            _ => panic!("expected number"),
        }
    }

    #[test]
    fn catalog_has_vin() {
        assert!(F150_DIDS.iter().any(|d| d.did == 0xF190));
    }
}
