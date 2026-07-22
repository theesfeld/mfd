//! Startup capability probe (demo timeline or live OBD).
//!
//! Runs until ready, then [`VehicleCaps`] freezes for page layout.

use crate::auto::caps::{BitLine, BitState, FeatureCaps, VehicleCaps};
use std::time::Instant;

/// Host-side probe state machine (demo path without vehicle).
pub struct DemoProbe {
    t0: Instant,
    caps: VehicleCaps,
}

impl DemoProbe {
    pub fn start() -> Self {
        let caps = VehicleCaps {
            phase: "POWER ON".into(),
            link: "DEMO".into(),
            lines: vec![BitLine {
                name: "MFDS".into(),
                state: BitState::Rdy,
            }],
            ..Default::default()
        };
        Self {
            t0: Instant::now(),
            caps,
        }
    }

    pub fn tick(&mut self) -> &VehicleCaps {
        let t = self.t0.elapsed().as_secs_f32();
        // ~2.8 s full BIT (feels like CMFD power-up, not endless)
        let steps: &[(&str, f32, BitState)] = &[
            ("MFDS", 0.15, BitState::Go),
            ("LINK", 0.35, BitState::Go),
            ("PCM", 0.55, BitState::Go),
            ("BCM", 0.75, BitState::Go),
            ("ABS", 0.95, BitState::Go),
            ("IPC", 1.15, BitState::Go),
            ("APIM", 1.35, BitState::Go),
            ("HSWM", 1.55, BitState::Go),
            ("J1979", 1.8, BitState::Go),
            ("UDS22", 2.05, BitState::Go),
            ("FOG", 2.2, BitState::Go),
            ("HSEAT", 2.35, BitState::Go),
            ("HSTR", 2.5, BitState::Go),
            ("TPMS", 2.65, BitState::Go),
            ("BIT", 2.8, BitState::Go),
        ];
        self.caps.progress = (t / 2.8).clamp(0.0, 1.0);
        self.caps.phase = if t < 0.2 {
            "POWER ON".into()
        } else if t < 2.8 {
            "BIT IN PROGRESS".into()
        } else {
            "BIT COMPLETE".into()
        };

        let mut lines = Vec::new();
        for &(name, at, st) in steps {
            if t >= at {
                lines.push(BitLine {
                    name: name.into(),
                    state: st,
                });
            } else if t >= at - 0.2 {
                lines.push(BitLine {
                    name: name.into(),
                    state: BitState::Rdy,
                });
            }
        }
        if lines.is_empty() {
            lines.push(BitLine {
                name: "MFDS".into(),
                state: BitState::Rdy,
            });
        }
        self.caps.lines = lines;

        if t >= 2.85 {
            self.caps = VehicleCaps::demo_complete();
        }
        &self.caps
    }

    pub fn caps(&self) -> &VehicleCaps {
        &self.caps
    }
}

/// Live OBD probe steps (read-only). Call from feed thread.
#[cfg(feature = "obd")]
pub fn run_live_probe(session: &mut crate::obd::session::Session) -> VehicleCaps {
    use crate::obd::ford::{self, DidDef, F150_DIDS, HDR_ABS, HDR_BCM, HDR_PCM};
    use crate::obd::j1979;

    let mut caps = VehicleCaps {
        link: session.name().into(),
        phase: "BIT IN PROGRESS".into(),
        progress: 0.1,
        ..Default::default()
    };
    let mut push = |name: &str, st: BitState| {
        caps.lines.push(BitLine {
            name: name.into(),
            state: st,
        });
    };
    push("MFDS", BitState::Go);
    push("LINK", BitState::Go);

    // Modules via extended session on headers
    for (name, hdr) in [("PCM", HDR_PCM), ("BCM", HDR_BCM), ("ABS", HDR_ABS)] {
        let ok = session.elm_mut().set_header(hdr).is_ok() && session.extended_session().is_ok();
        push(name, if ok { BitState::Go } else { BitState::Nogo });
        if ok {
            caps.modules.insert(name);
        }
        caps.progress += 0.08;
    }
    let _ = ford::prepare_pcm_read(session);
    push("APIM", BitState::Rdy); // not always UDS-reachable via ELM
    push("HSWM", BitState::Rdy);

    // Mode 01 PID support (bitmap 0100)
    caps.progress = 0.45;
    match session.elm_mut().request_hex("0100") {
        Ok(bytes) if bytes.len() >= 6 && bytes[0] == 0x41 && bytes[1] == 0x00 => {
            // bytes[2..] are support bitmap for PIDs 01-20
            let map = &bytes[2..];
            for (i, &b) in map.iter().enumerate().take(4) {
                for bit in 0..8 {
                    if b & (0x80 >> bit) != 0 {
                        let pid = (i as u8) * 8 + bit as u8 + 1;
                        caps.pids.insert(pid);
                    }
                }
            }
            push("J1979", BitState::Go);
        }
        _ => {
            // Fall back: try priority PIDs individually
            for &pid in j1979::PRIORITY_PIDS {
                if session.read_pid(pid).is_ok() {
                    caps.pids.insert(pid);
                }
            }
            push(
                "J1979",
                if caps.pids.is_empty() {
                    BitState::Nogo
                } else {
                    BitState::Go
                },
            );
        }
    }
    caps.progress = 0.6;

    // Feature DIDs
    let mut uds_ok = 0u32;
    for def in F150_DIDS {
        if ford::read_did(session, def).is_ok() {
            caps.dids.insert(def.did);
            uds_ok += 1;
        }
    }
    let _comfort: &[DidDef] = &[];
    push(
        "UDS22",
        if uds_ok > 0 {
            BitState::Go
        } else {
            BitState::Nogo
        },
    );

    // Map features from what we found
    caps.features = FeatureCaps {
        fog_lights: false, // only if DID/status later GO
        heated_seats: false,
        heated_steering: caps.modules.contains("HSWM"),
        tpms: caps.modules.contains("ABS"),
        abs: caps.modules.contains("ABS"),
        camera: std::env::var_os("MFD_CAMERA").is_some()
            || std::env::var_os("MFD_FLIR_PATH").is_some(),
        park_sensors: std::env::var_os("MFD_RANGE").is_some(),
        four_wd: true, // SuperCrew 4x4 install
        hvac: true,
        ambient_temp: caps.pids.contains(&0x46) || caps.dids.contains(&0xF457),
        oil_temp: caps.pids.contains(&0x5C) || caps.dids.contains(&0xF45C),
        trans_temp: caps.dids.contains(&0x1E1C),
        fuel_level: caps.pids.contains(&0x2F) || caps.dids.contains(&0xF41F),
        fuel_pressure: caps.dids.contains(&0x03DC),
        attitude: true, // glass always; live data may be synthetic until IMU/DID
        map: true,
    };
    // If BCM GO, allow fog/body options as "probe later" present for XLT-class
    if caps.modules.contains("BCM") {
        caps.features.fog_lights = true;
        caps.features.heated_seats = true;
    }
    if caps.modules.contains("HSWM") {
        caps.features.heated_steering = true;
        push("HSTR", BitState::Go);
    } else {
        push("HSTR", BitState::Nogo);
    }
    push(
        "FOG",
        if caps.features.fog_lights {
            BitState::Go
        } else {
            BitState::Nogo
        },
    );
    push(
        "HSEAT",
        if caps.features.heated_seats {
            BitState::Go
        } else {
            BitState::Nogo
        },
    );
    push(
        "TPMS",
        if caps.features.tpms {
            BitState::Go
        } else {
            BitState::Nogo
        },
    );

    push("BIT", BitState::Go);
    caps.progress = 1.0;
    caps.phase = "BIT COMPLETE".into();
    caps.ready = true;
    caps
}
