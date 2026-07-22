//! SAE J1979 Mode 01 PID decode (standard powertrain).

use crate::obd::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct LiveValue {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
    pub mode: u8,
    pub pid: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct PidDef {
    pub pid: u8,
    pub name: &'static str,
    pub unit: &'static str,
    pub bytes: u8,
}

/// Priority poll order for live dashboards (high rate first — drive glass).
pub const PRIORITY_PIDS: &[u8] = &[
    0x0C, // RPM
    0x0D, // Speed
    0x0C, // RPM again (weight)
    0x0D, // Speed again
    0x11, // TPS
    0x2F, // Fuel level
    0x42, // Control module voltage
    0x04, // Load
    0x05, // Coolant
    0x0F, // IAT
    0x5C, // Oil temp
    0x10, // MAF
    0x46, // Ambient
    0x0B, // MAP
    0x49, // Accel D
    0x4A, // Accel E
    0x4C, // Throttle cmd
];

/// Mode 01 support PIDs used to discover available channels (J1979).
pub const SUPPORT_PIDS: &[u8] = &[0x00, 0x20, 0x40, 0x60, 0x80, 0xA0, 0xC0];

/// Parse Mode 01 support response `41 XX <4 bytes bitmap>` → list of supported PIDs.
///
/// For support PID `base` (0x00, 0x20, …), bit 7 of first data byte = PID `base+1`.
pub fn parse_support_bitmap(support_pid: u8, payload: &[u8]) -> Vec<u8> {
    // Expect 41 <support_pid> b0 b1 b2 b3
    if payload.len() < 6 || payload[0] != 0x41 {
        return Vec::new();
    }
    let base = payload[1];
    if base != support_pid {
        // still try using payload[1] as base
    }
    let base = support_pid;
    let map = &payload[2..payload.len().min(6)];
    let mut out = Vec::new();
    for (i, &b) in map.iter().enumerate() {
        for bit in 0..8 {
            if b & (0x80u8 >> bit) != 0 {
                let pid = base
                    .saturating_add(1)
                    .saturating_add((i as u8) * 8 + bit as u8);
                if pid != 0 {
                    out.push(pid);
                }
            }
        }
    }
    out
}

pub fn pid_def(pid: u8) -> Option<PidDef> {
    Some(match pid {
        0x03 => PidDef {
            pid,
            name: "fuel_system_status",
            unit: "",
            bytes: 2,
        },
        0x04 => PidDef {
            pid,
            name: "engine_load",
            unit: "%",
            bytes: 1,
        },
        0x05 => PidDef {
            pid,
            name: "coolant_temp",
            unit: "C",
            bytes: 1,
        },
        0x06 => PidDef {
            pid,
            name: "stft_b1",
            unit: "%",
            bytes: 1,
        },
        0x07 => PidDef {
            pid,
            name: "ltft_b1",
            unit: "%",
            bytes: 1,
        },
        0x0A => PidDef {
            pid,
            name: "fuel_pressure",
            unit: "kPa",
            bytes: 1,
        },
        0x0B => PidDef {
            pid,
            name: "map",
            unit: "kPa",
            bytes: 1,
        },
        0x0C => PidDef {
            pid,
            name: "engine_rpm",
            unit: "rpm",
            bytes: 2,
        },
        0x0D => PidDef {
            pid,
            name: "vehicle_speed",
            unit: "km/h",
            bytes: 1,
        },
        0x0E => PidDef {
            pid,
            name: "timing_advance",
            unit: "deg",
            bytes: 1,
        },
        0x0F => PidDef {
            pid,
            name: "intake_temp",
            unit: "C",
            bytes: 1,
        },
        0x10 => PidDef {
            pid,
            name: "maf",
            unit: "g/s",
            bytes: 2,
        },
        0x11 => PidDef {
            pid,
            name: "throttle",
            unit: "%",
            bytes: 1,
        },
        0x1F => PidDef {
            pid,
            name: "run_time",
            unit: "s",
            bytes: 2,
        },
        0x21 => PidDef {
            pid,
            name: "distance_mil",
            unit: "km",
            bytes: 2,
        },
        0x23 => PidDef {
            pid,
            name: "fuel_rail_pressure",
            unit: "kPa",
            bytes: 2,
        },
        0x2F => PidDef {
            pid,
            name: "fuel_level",
            unit: "%",
            bytes: 1,
        },
        0x33 => PidDef {
            pid,
            name: "baro",
            unit: "kPa",
            bytes: 1,
        },
        0x42 => PidDef {
            pid,
            name: "control_module_voltage",
            unit: "V",
            bytes: 2,
        },
        0x43 => PidDef {
            pid,
            name: "abs_load",
            unit: "%",
            bytes: 2,
        },
        0x45 => PidDef {
            pid,
            name: "throttle_rel",
            unit: "%",
            bytes: 1,
        },
        0x46 => PidDef {
            pid,
            name: "ambient_temp",
            unit: "C",
            bytes: 1,
        },
        0x47 => PidDef {
            pid,
            name: "throttle_abs_b",
            unit: "%",
            bytes: 1,
        },
        0x49 => PidDef {
            pid,
            name: "accel_pedal_d",
            unit: "%",
            bytes: 1,
        },
        0x4A => PidDef {
            pid,
            name: "accel_pedal_e",
            unit: "%",
            bytes: 1,
        },
        0x4C => PidDef {
            pid,
            name: "throttle_cmd",
            unit: "%",
            bytes: 1,
        },
        0x5A => PidDef {
            pid,
            name: "accel_pedal",
            unit: "%",
            bytes: 1,
        },
        0x5C => PidDef {
            pid,
            name: "oil_temp",
            unit: "C",
            bytes: 1,
        },
        0x5E => PidDef {
            pid,
            name: "fuel_rate",
            unit: "L/h",
            bytes: 2,
        },
        // Additional PIDs seen live on 2019 2.7 F-150 capture
        0x2E => PidDef {
            pid,
            name: "evap_purge",
            unit: "%",
            bytes: 1,
        },
        0x30 => PidDef {
            pid,
            name: "warmups_cleared",
            unit: "",
            bytes: 1,
        },
        0x31 => PidDef {
            pid,
            name: "distance_cleared",
            unit: "km",
            bytes: 2,
        },
        0x3C => PidDef {
            pid,
            name: "catalyst_temp_b1s1",
            unit: "C",
            bytes: 2,
        },
        0x3D => PidDef {
            pid,
            name: "catalyst_temp_b2s1",
            unit: "C",
            bytes: 2,
        },
        0x44 => PidDef {
            pid,
            name: "cmd_equiv_ratio",
            unit: "",
            bytes: 2,
        },
        0x51 => PidDef {
            pid,
            name: "fuel_type",
            unit: "",
            bytes: 1,
        },
        0x1C => PidDef {
            pid,
            name: "obd_standard",
            unit: "",
            bytes: 1,
        },
        _ => return None,
    })
}

/// Decode Mode 01 response bytes (`41 <pid> <data…>`).
pub fn decode_mode01(payload: &[u8]) -> Result<LiveValue> {
    if payload.len() < 3 {
        return Err(Error::Decode(format!("short Mode01: {payload:02X?}")));
    }
    if payload[0] != 0x41 {
        return Err(Error::Decode(format!(
            "not Mode01 positive: {payload:02X?}"
        )));
    }
    let pid = payload[1];
    let data = &payload[2..];
    if let Some(def) = pid_def(pid) {
        if data.len() < def.bytes as usize {
            return Err(Error::Decode(format!(
                "PID {pid:02X} needs {} bytes, got {}",
                def.bytes,
                data.len()
            )));
        }
        let value = match pid {
            0x03 | 0x1C | 0x30 | 0x51 => data[0] as f64,
            0x04 | 0x11 | 0x2E | 0x2F | 0x45 | 0x47 | 0x49 | 0x4A | 0x4C | 0x5A => {
                data[0] as f64 * 100.0 / 255.0
            }
            0x05 | 0x0F | 0x46 | 0x5C => data[0] as f64 - 40.0,
            0x06 | 0x07 => data[0] as f64 * 100.0 / 128.0 - 100.0,
            0x0A => data[0] as f64 * 3.0,
            0x0B | 0x0D | 0x33 => data[0] as f64,
            0x0C => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 4.0,
            0x0E => data[0] as f64 / 2.0 - 64.0,
            0x10 => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 100.0,
            0x1F | 0x21 | 0x31 => ((data[0] as u16) << 8 | data[1] as u16) as f64,
            0x23 => ((data[0] as u16) << 8 | data[1] as u16) as f64 * 10.0,
            // Catalyst temp: (A*256+B)/10 - 40
            0x3C | 0x3D => {
                ((data[0] as u16) << 8 | data[1] as u16) as f64 / 10.0 - 40.0
            }
            // Equiv ratio: (A*256+B)/32768
            0x44 => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 32768.0,
            0x42 => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 1000.0,
            0x43 => ((data[0] as u16) << 8 | data[1] as u16) as f64 * 100.0 / 255.0,
            0x5E => ((data[0] as u16) << 8 | data[1] as u16) as f64 / 20.0,
            _ => return Err(Error::Decode(format!("no formula for PID {pid:02X}"))),
        };
        return Ok(LiveValue {
            name: def.name,
            value,
            unit: def.unit,
            mode: 1,
            pid,
        });
    }
    // Unknown PID: still emit raw so crush capture keeps every answer.
    let value = if data.len() >= 2 {
        ((data[0] as u16) << 8 | data[1] as u16) as f64
    } else if !data.is_empty() {
        data[0] as f64
    } else {
        0.0
    };
    Ok(LiveValue {
        name: "pid_raw",
        value,
        unit: "raw",
        mode: 1,
        pid,
    })
}

pub fn mode01_command(pid: u8) -> String {
    format!("01{pid:02X}")
}

// ─── Diagnostic trouble codes (read-only; never Mode 04 clear) ───────────────

/// DTC class from SAE J2012 encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DtcKind {
    /// Stored confirmed (Mode 03).
    Stored,
    /// Pending (Mode 07).
    Pending,
    /// Permanent (Mode 0A).
    Permanent,
}

impl DtcKind {
    pub fn label(self) -> &'static str {
        match self {
            DtcKind::Stored => "STORED",
            DtcKind::Pending => "PEND",
            DtcKind::Permanent => "PERM",
        }
    }
}

/// One fault code for glass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dtc {
    /// e.g. `P0420`
    pub code: String,
    pub kind: DtcKind,
}

/// Format two DTC bytes as SAE string (`P0420`, `C1234`, …).
pub fn format_dtc_bytes(b0: u8, b1: u8) -> String {
    let sys = (b0 >> 6) & 0x03;
    let d1 = (b0 >> 4) & 0x03;
    let d2 = b0 & 0x0F;
    let d3 = (b1 >> 4) & 0x0F;
    let d4 = b1 & 0x0F;
    let letter = match sys {
        0 => 'P',
        1 => 'C',
        2 => 'B',
        _ => 'U',
    };
    format!("{letter}{d1:X}{d2:X}{d3:X}{d4:X}")
}

/// Decode Mode 03/07/0A payload (`43…` / `47…` / `4A…`) into codes.
///
/// Accepts ELM multi-line concatenation. Skips null `00 00` pairs.
pub fn decode_dtc_response(payload: &[u8], kind: DtcKind) -> Result<Vec<Dtc>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    let expect = match kind {
        DtcKind::Stored => 0x43u8,
        DtcKind::Pending => 0x47,
        DtcKind::Permanent => 0x4A,
    };
    // Find service response byte (may be multi-frame with count prefixes)
    let mut i = 0usize;
    while i < payload.len() {
        if payload[i] == expect {
            i += 1;
            break;
        }
        // Some ECUs return 43 NN then pairs; also handle leading count after SID
        i += 1;
    }
    if i == 0 && payload[0] != expect {
        // No SID match — try raw pairs if even length and looks empty/no data
        if payload.len() >= 2 && payload.iter().all(|&b| b == 0) {
            return Ok(Vec::new());
        }
        return Err(Error::Decode(format!(
            "not DTC response for {:?}: {payload:02X?}",
            kind
        )));
    }
    // Optional count byte (some ISO-TP: 43 <n> <dtcs…>)
    if i < payload.len() && payload[i] <= 0x10 && (payload.len() - i - 1) >= payload[i] as usize * 2
    {
        // Heuristic: small first byte as count when remaining fits
        let n = payload[i] as usize;
        if n * 2 < payload.len() - i {
            i += 1;
        }
    }
    let mut out = Vec::new();
    while i + 1 < payload.len() {
        let b0 = payload[i];
        let b1 = payload[i + 1];
        i += 2;
        if b0 == 0 && b1 == 0 {
            continue;
        }
        out.push(Dtc {
            code: format_dtc_bytes(b0, b1),
            kind,
        });
    }
    Ok(out)
}

/// Merge Mode 03+07+0A lists; de-dupe by code keeping stronger kind (PERM > STORED > PEND).
pub fn merge_dtcs(lists: &[Vec<Dtc>]) -> Vec<Dtc> {
    use std::collections::HashMap;
    let rank = |k: DtcKind| match k {
        DtcKind::Permanent => 3,
        DtcKind::Stored => 2,
        DtcKind::Pending => 1,
    };
    let mut map: HashMap<String, DtcKind> = HashMap::new();
    for list in lists {
        for d in list {
            map.entry(d.code.clone())
                .and_modify(|k| {
                    if rank(d.kind) > rank(*k) {
                        *k = d.kind;
                    }
                })
                .or_insert(d.kind);
        }
    }
    let mut v: Vec<Dtc> = map
        .into_iter()
        .map(|(code, kind)| Dtc { code, kind })
        .collect();
    v.sort_by(|a, b| a.code.cmp(&b.code));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_from_capture() {
        // 410C0A8E → (0x0A8E)/4 = 675.5
        let v = decode_mode01(&[0x41, 0x0C, 0x0A, 0x8E]).unwrap();
        assert_eq!(v.name, "engine_rpm");
        assert!((v.value - 675.5).abs() < 0.01);
    }

    #[test]
    fn speed_zero() {
        let v = decode_mode01(&[0x41, 0x0D, 0x00]).unwrap();
        assert_eq!(v.value, 0.0);
    }

    #[test]
    fn format_p0420() {
        // P0420 = 0x04 0x20
        assert_eq!(format_dtc_bytes(0x04, 0x20), "P0420");
    }

    #[test]
    fn decode_mode03_two_codes() {
        // 43 04 20 01 00 → P0420, P0100
        let d = decode_dtc_response(&[0x43, 0x04, 0x20, 0x01, 0x00], DtcKind::Stored).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].code, "P0420");
        assert_eq!(d[1].code, "P0100");
    }

    #[test]
    fn decode_empty_pairs() {
        let d = decode_dtc_response(&[0x43, 0x00, 0x00], DtcKind::Stored).unwrap();
        assert!(d.is_empty());
    }
}
