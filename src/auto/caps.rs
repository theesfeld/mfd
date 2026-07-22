//! Vehicle **capability map** from startup probe.
//!
//! Only features that probe **GO** are shown on glass. Unknown / NOGO = omitted.

use std::collections::HashSet;

/// BIT line status (F-16 TEST language).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitState {
    /// In progress
    Rdy,
    /// Present / OK
    Go,
    /// Not present / failed probe
    Nogo,
}

impl BitState {
    pub fn label(self) -> &'static str {
        match self {
            BitState::Rdy => "RDY",
            BitState::Go => "GO",
            BitState::Nogo => "NOGO",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BitLine {
    pub name: String,
    pub state: BitState,
}

/// Optional vehicle equipment (omit from UI when not GO).
#[derive(Clone, Debug, Default)]
pub struct FeatureCaps {
    pub fog_lights: bool,
    pub heated_seats: bool,
    pub heated_steering: bool,
    pub tpms: bool,
    pub abs: bool,
    pub camera: bool,
    pub park_sensors: bool,
    pub four_wd: bool,
    pub hvac: bool,
    pub ambient_temp: bool,
    pub oil_temp: bool,
    pub trans_temp: bool,
    pub fuel_level: bool,
    pub fuel_pressure: bool,
    pub attitude: bool,
    pub map: bool,
}

/// Result of startup probe (drives pages + BIT screen).
#[derive(Clone, Debug)]
pub struct VehicleCaps {
    pub ready: bool,
    /// 0.0 .. 1.0
    pub progress: f32,
    pub phase: String,
    pub lines: Vec<BitLine>,
    pub features: FeatureCaps,
    pub pids: HashSet<u8>,
    pub dids: HashSet<u16>,
    pub modules: HashSet<&'static str>,
    pub link: String,
}

impl Default for VehicleCaps {
    fn default() -> Self {
        Self {
            ready: false,
            progress: 0.0,
            phase: "POWER ON".into(),
            lines: Vec::new(),
            features: FeatureCaps::default(),
            pids: HashSet::new(),
            dids: HashSet::new(),
            modules: HashSet::new(),
            link: "NONE".into(),
        }
    }
}

impl VehicleCaps {
    /// Demo / offline probe result for SuperCrew 2.7 XLT-class truck.
    pub fn demo_complete() -> Self {
        let mut c = Self {
            ready: true,
            progress: 1.0,
            phase: "BIT COMPLETE".into(),
            link: "DEMO".into(),
            ..Default::default()
        };
        c.features = FeatureCaps {
            fog_lights: true,
            heated_seats: true,
            heated_steering: true,
            tpms: true,
            abs: true,
            camera: true,
            park_sensors: true,
            four_wd: true,
            hvac: true,
            ambient_temp: true,
            oil_temp: true,
            trans_temp: true,
            fuel_level: true,
            fuel_pressure: true,
            attitude: true,
            map: true,
        };
        for p in [
            0x04u8, 0x05, 0x0C, 0x0D, 0x0F, 0x10, 0x11, 0x2F, 0x42, 0x46, 0x5C,
        ] {
            c.pids.insert(p);
        }
        for d in [
            0xF405u16, 0xF40F, 0xF45C, 0x1E1C, 0xF41F, 0x402C, 0x1E12, 0x2B00,
        ] {
            c.dids.insert(d);
        }
        for m in ["PCM", "BCM", "ABS", "IPC", "APIM", "HSWM", "PSCM"] {
            c.modules.insert(m);
        }
        c.lines = vec![
            BitLine {
                name: "MFDS".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "PCM".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "BCM".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "ABS".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "IPC".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "APIM".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "HSWM".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "J1979".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "UDS22".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "FOG".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "HSEAT".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "HSTR".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "TPMS".into(),
                state: BitState::Go,
            },
            BitLine {
                name: "BIT".into(),
                state: BitState::Go,
            },
        ];
        c
    }

    /// Formats allowed after probe (Lockheed decision table + product hard pages).
    ///
    /// **Always (link ready):** ENG · DRV · FUEL · FLUD · ELEC · **ATT** · **DTC** ·
    /// OWN · SET · BUS (shop).  
    /// **Probe-gated:** CHAS · BODY · LITE · CLIM · CAM · RNG · MAP.  
    /// Blank Master Menu labels for omitted formats — never repack OSB slots.
    pub fn pages(&self) -> Vec<crate::auto::AutoPage> {
        use crate::auto::AutoPage;
        if !self.ready {
            return Vec::new();
        }
        // Core propulsion / drive / energy — always when link is up.
        let mut p = vec![
            AutoPage::Eng,   // tach (hard requirement)
            AutoPage::Drive, // speedo (hard requirement)
            AutoPage::Fuel,
            AutoPage::Fluid,
            AutoPage::Elec,
            AutoPage::Attitude, // dedicated ATT (hard requirement)
            AutoPage::Faults,   // dedicated DTC (hard requirement)
        ];
        // Comfort / chassis / sensors — GO only.
        if self.features.tpms || self.features.abs {
            p.push(AutoPage::Chas);
        }
        // Body/lights: show if we have any body-class data path (always on live truck).
        p.push(AutoPage::Body);
        p.push(AutoPage::Lights);
        if self.features.hvac {
            p.push(AutoPage::Clim);
        }
        if self.features.camera {
            p.push(AutoPage::Cam);
        }
        if self.features.park_sensors {
            p.push(AutoPage::Range);
        }
        if self.features.map {
            p.push(AutoPage::Map);
        }
        // Shop / identity / setup — always available (not hollow).
        p.push(AutoPage::Bus);
        p.push(AutoPage::Own);
        p.push(AutoPage::Setup);
        p
    }
}
