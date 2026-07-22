//! Automotive **page layouts** — same bezel / OSB model as jet, vehicle data.
//!
//! Optional live OBD: feature `obd` + `MFD_OBD_PORT` / `MFD_OBD_REPLAY` (see `obd_feed`).
//!
//! # Widget mapping (MFD equivalents)
//! | Vehicle data | MFD widget |
//! |--------------|------------|
//! | Fuel, oil, temps, battery, flow | tape_gauge |
//! | Engine RPM | round_gauge |
//! | Speed | value_readout |
//! | Throttle % | progress_strip |
//! | Gear / 4WD / lights / doors | status_grid |
//! | TPM | tire_grid |
//! | Forward camera / FLIR | greyscale blit + TGP overlays |
//! | Collision / park range | range_display |

#[cfg(feature = "obd")]
pub mod obd_feed;

use crate::bezel::BezelState;
use crate::color::{rgb, CYAN};
use crate::geom::Rect;
use crate::page::Page;
use crate::palette::Palette;
use crate::video::{blit_grey_flir, GreyFrame};
use crate::widget::{
    attitude_ball, caution_box, content_after_osb, crosshair, heading_display, heading_rose, label,
    list_menu, numeric_readout, osb_chrome, progress_strip, range_display, round_gauge,
    schematic_topo_map, status_grid, tape_gauge, tire_grid, track_gate, value_readout,
    RangeSnapshot, RoundGaugeOpts, StatusItem, TapeOpts, TapeOrientation, TireReading,
};
use crate::Surface;

// ─── Data model ──────────────────────────────────────────────────────────────

/// Speed display unit (OSB cycles on CLUSTER / SETUP).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpeedUnit {
    #[default]
    Mph,
    Kmh,
    Knots,
}

impl SpeedUnit {
    pub fn name(self) -> &'static str {
        match self {
            SpeedUnit::Mph => "MPH",
            SpeedUnit::Kmh => "KM/H",
            SpeedUnit::Knots => "KT",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            SpeedUnit::Mph => SpeedUnit::Kmh,
            SpeedUnit::Kmh => SpeedUnit::Knots,
            SpeedUnit::Knots => SpeedUnit::Mph,
        }
    }

    /// Convert from stored mph.
    pub fn from_mph(self, mph: f32) -> f32 {
        match self {
            SpeedUnit::Mph => mph,
            SpeedUnit::Kmh => mph * 1.60934,
            SpeedUnit::Knots => mph * 0.868976,
        }
    }
}

/// Transmission range selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GearSelect {
    Park,
    Reverse,
    Neutral,
    #[default]
    Drive,
    Manual,
}

impl GearSelect {
    pub fn name(self) -> &'static str {
        match self {
            GearSelect::Park => "P",
            GearSelect::Reverse => "R",
            GearSelect::Neutral => "N",
            GearSelect::Drive => "D",
            GearSelect::Manual => "M",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GearSelect::Park => "PARK",
            GearSelect::Reverse => "REV",
            GearSelect::Neutral => "NEUT",
            GearSelect::Drive => "DRIVE",
            GearSelect::Manual => "MAN",
        }
    }
}

/// Transfer case / drive mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DriveMode {
    /// 2WD rear (or FWD — host labels).
    #[default]
    TwoHigh,
    FourHigh,
    FourLow,
}

impl DriveMode {
    pub fn name(self) -> &'static str {
        match self {
            DriveMode::TwoHigh => "2H",
            DriveMode::FourHigh => "4H",
            DriveMode::FourLow => "4L",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DriveMode::TwoHigh => "2WD HIGH",
            DriveMode::FourHigh => "4WD HIGH",
            DriveMode::FourLow => "4WD LOW",
        }
    }
}

/// Full vehicle snapshot for auto pages (demo or OBD/CAN host).
#[derive(Clone, Debug)]
pub struct VehicleSnapshot {
    /// Engine RPM absolute (e.g. 0..7000).
    pub rpm: f32,
    pub rpm_redline: f32,
    /// Speed in **mph** (convert with [`SpeedUnit`]).
    pub speed_mph: f32,
    pub throttle: f32,
    pub fuel: f32,
    pub battery: f32,
    pub load: f32,
    pub oil_temp: f32,
    pub coolant: f32,
    pub trans_temp: f32,
    pub iat: f32,
    pub maf: f32,
    pub exhaust_temp: f32,
    pub gear: GearSelect,
    /// Manual gear 1..6 when Manual.
    pub gear_num: u8,
    pub drive: DriveMode,
    pub light_low: bool,
    pub light_high: bool,
    pub light_drive: bool,
    pub light_fog: bool,
    pub light_brake: bool,
    pub light_turn_l: bool,
    pub light_turn_r: bool,
    pub light_interior: bool,
    pub tire_fl: TireReading,
    pub tire_fr: TireReading,
    pub tire_rl: TireReading,
    pub tire_rr: TireReading,
    pub door_fl: bool,
    pub door_fr: bool,
    pub door_rl: bool,
    pub door_rr: bool,
    pub door_hatch: bool,
    pub belt_fl: bool,
    pub belt_fr: bool,
    pub belt_rl: bool,
    pub belt_rr: bool,
    pub temp_out_c: f32,
    pub temp_in_c: f32,
    pub hvac_fan: f32,
    pub hvac_set_c: f32,
    pub hvac_ac: bool,
    pub hvac_defrost: bool,
    pub dtc_count: u32,
    pub speed_unit: SpeedUnit,
    /// Pitch degrees (nose up +).
    pub pitch_deg: f32,
    /// Roll degrees (right wing down +).
    pub roll_deg: f32,
    /// Heading degrees magnetic/true (0–360, 0 = north).
    pub heading_deg: f32,
}

impl Default for VehicleSnapshot {
    fn default() -> Self {
        Self {
            rpm: 900.0,
            rpm_redline: 6500.0,
            speed_mph: 0.0,
            throttle: 0.0,
            fuel: 0.62,
            battery: 0.72,
            load: 0.2,
            oil_temp: 0.45,
            coolant: 0.5,
            trans_temp: 0.4,
            iat: 0.35,
            maf: 0.3,
            exhaust_temp: 0.4,
            gear: GearSelect::Park,
            gear_num: 1,
            drive: DriveMode::TwoHigh,
            light_low: true,
            light_high: false,
            light_drive: false,
            light_fog: false,
            light_brake: false,
            light_turn_l: false,
            light_turn_r: false,
            light_interior: false,
            tire_fl: TireReading {
                pressure: 35.0,
                temp_c: 28.0,
                alert: false,
            },
            tire_fr: TireReading {
                pressure: 35.0,
                temp_c: 28.0,
                alert: false,
            },
            tire_rl: TireReading {
                pressure: 34.0,
                temp_c: 30.0,
                alert: false,
            },
            tire_rr: TireReading {
                pressure: 34.0,
                temp_c: 30.0,
                alert: false,
            },
            door_fl: true,
            door_fr: true,
            door_rl: true,
            door_rr: true,
            door_hatch: true,
            belt_fl: true,
            belt_fr: true,
            belt_rl: false,
            belt_rr: false,
            temp_out_c: 18.0,
            temp_in_c: 22.0,
            hvac_fan: 0.4,
            hvac_set_c: 21.0,
            hvac_ac: true,
            hvac_defrost: false,
            dtc_count: 0,
            speed_unit: SpeedUnit::Mph,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            heading_deg: 0.0,
        }
    }
}

/// Animated demo vehicle (sinusoids) — host replaces with OBD/CAN.
pub fn demo_vehicle(t: f32) -> VehicleSnapshot {
    let mut v = VehicleSnapshot::default();
    v.rpm = 900.0 + 2800.0 * (0.5 + 0.5 * (t * 0.55).sin());
    v.speed_mph = 25.0 + 40.0 * (0.5 + 0.5 * (t * 0.35).sin());
    v.throttle = 0.2 + 0.5 * (0.5 + 0.5 * (t * 0.8).sin());
    v.fuel = 0.55 + 0.1 * (t * 0.08).cos();
    v.battery = 0.65 + 0.08 * (t * 0.2).sin();
    v.load = 0.25 + 0.35 * (0.5 + 0.5 * (t * 0.5).cos());
    v.oil_temp = 0.4 + 0.15 * (t * 0.12).sin();
    v.coolant = 0.48 + 0.1 * (t * 0.1).sin();
    v.trans_temp = 0.38 + 0.12 * (t * 0.15).cos();
    v.iat = 0.3 + 0.1 * (t * 0.2).sin();
    v.maf = 0.25 + 0.3 * v.throttle;
    v.exhaust_temp = 0.35 + 0.25 * v.throttle;
    v.gear = if v.speed_mph < 2.0 {
        GearSelect::Park
    } else if (t * 0.15).sin() > 0.7 {
        GearSelect::Manual
    } else {
        GearSelect::Drive
    };
    v.gear_num = 1 + ((v.speed_mph / 15.0) as u8).min(5);
    v.drive = if (t * 0.07).sin() > 0.5 {
        DriveMode::FourHigh
    } else {
        DriveMode::TwoHigh
    };
    v.light_low = true;
    v.light_turn_l = (t * 2.0).sin() > 0.3 && (t as i32 % 4 < 2);
    v.light_brake = v.throttle < 0.15 && v.speed_mph > 10.0;
    v.tire_fl.pressure = 34.0 + (t * 0.3).sin();
    v.tire_fr.pressure = 35.0 + (t * 0.25).cos();
    v.tire_rl.alert = (t * 0.05).sin() > 0.92;
    if v.tire_rl.alert {
        v.tire_rl.pressure = 22.0;
    }
    v.door_fr = (t * 0.04).sin() < 0.95;
    v.temp_out_c = 12.0 + 8.0 * (t * 0.03).sin();
    v.temp_in_c = 20.0 + 2.0 * (t * 0.1).cos();
    v.hvac_fan = 0.3 + 0.4 * (0.5 + 0.5 * (t * 0.2).sin());
    // Stronger attitude so the sphere reads as a ball in demo.
    v.pitch_deg = 18.0 * (t * 0.45).sin();
    v.roll_deg = 32.0 * (t * 0.35).cos();
    v.heading_deg = (t * 18.0) % 360.0;
    v
}

/// Back-compat thin OBD view (older API).
#[derive(Clone, Debug, Default)]
pub struct ObdSnapshot {
    pub rpm: f32,
    pub speed: f32,
    pub fuel: f32,
    pub coolant: f32,
    pub trans_temp: f32,
    pub battery: f32,
    pub throttle: f32,
    pub load: f32,
    pub dtc_count: u32,
}

impl From<&VehicleSnapshot> for ObdSnapshot {
    fn from(v: &VehicleSnapshot) -> Self {
        Self {
            rpm: (v.rpm / v.rpm_redline).clamp(0.0, 1.0),
            speed: (v.speed_mph / 120.0).clamp(0.0, 1.0),
            fuel: v.fuel,
            coolant: v.coolant,
            trans_temp: v.trans_temp,
            battery: v.battery,
            throttle: v.throttle,
            load: v.load,
            dtc_count: v.dtc_count,
        }
    }
}

// ─── Pages ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoPage {
    Cluster,
    Fuel,
    Temps,
    Drive,
    Lights,
    Tpm,
    Body,
    Clim,
    Flir,
    /// Forward/rear parking & collision ranges (sensor arcs).
    Collision,
    /// Attitude ball + heading (cardinals + degrees).
    Attitude,
    /// Schematic line/topo map (not full DEM).
    Map,
    Obd,
    Setup,
}

impl AutoPage {
    pub const ALL: &'static [AutoPage] = &[
        AutoPage::Cluster,
        AutoPage::Fuel,
        AutoPage::Temps,
        AutoPage::Drive,
        AutoPage::Lights,
        AutoPage::Tpm,
        AutoPage::Body,
        AutoPage::Clim,
        AutoPage::Flir,
        AutoPage::Collision,
        AutoPage::Attitude,
        AutoPage::Map,
        AutoPage::Obd,
        AutoPage::Setup,
    ];

    pub fn name(self) -> &'static str {
        match self {
            AutoPage::Cluster => "CLST",
            AutoPage::Fuel => "FUEL",
            AutoPage::Temps => "TEMP",
            AutoPage::Drive => "DRV",
            AutoPage::Lights => "LITE",
            AutoPage::Tpm => "TPM",
            AutoPage::Body => "BODY",
            AutoPage::Clim => "CLIM",
            AutoPage::Flir => "FLIR",
            AutoPage::Collision => "RNG",
            AutoPage::Attitude => "ATT",
            AutoPage::Map => "MAP",
            AutoPage::Obd => "OBD",
            AutoPage::Setup => "SET",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            AutoPage::Cluster => "CLUSTER",
            AutoPage::Fuel => "FUEL / BAT",
            AutoPage::Temps => "TEMPS",
            AutoPage::Drive => "DRIVE",
            AutoPage::Lights => "LIGHTS",
            AutoPage::Tpm => "TPM",
            AutoPage::Body => "BODY",
            AutoPage::Clim => "CLIMATE",
            AutoPage::Flir => "FLIR / CAM",
            AutoPage::Collision => "RANGE",
            AutoPage::Attitude => "ATTITUDE",
            AutoPage::Map => "MAP",
            AutoPage::Obd => "OBD",
            AutoPage::Setup => "SETUP",
        }
    }

    /// Top OSB 1–5 primary bank.
    pub fn from_top_osb(osb: u8) -> Option<AutoPage> {
        match osb {
            1 => Some(AutoPage::Cluster),
            2 => Some(AutoPage::Fuel),
            3 => Some(AutoPage::Temps),
            4 => Some(AutoPage::Drive),
            5 => Some(AutoPage::Lights),
            _ => None,
        }
    }

    /// Right OSB 6–10.
    pub fn from_right_osb(osb: u8) -> Option<AutoPage> {
        match osb {
            6 => Some(AutoPage::Tpm),
            7 => Some(AutoPage::Body),
            8 => Some(AutoPage::Clim),
            9 => Some(AutoPage::Flir),
            10 => Some(AutoPage::Collision),
            _ => None,
        }
    }

    /// Left OSB 16–20 (chrome left[0]=OSB20).
    pub fn from_left_osb(osb: u8) -> Option<AutoPage> {
        match osb {
            20 => Some(AutoPage::Obd),
            19 => Some(AutoPage::Setup),
            18 => Some(AutoPage::Attitude),
            17 => Some(AutoPage::Map),
            _ => None,
        }
    }
}

type Osb5 = [&'static str; 5];

fn legends(page: AutoPage, v: &VehicleSnapshot) -> (Osb5, Osb5, Osb5, Osb5) {
    let top = ["CLST", "FUEL", "TEMP", "DRV", "LITE"];
    let right = ["TPM", "BODY", "CLIM", "FLIR", "RNG"];
    // Bottom: context for page
    let bottom: Osb5 = match page {
        AutoPage::Cluster => ["UNIT", v.speed_unit.name(), "", "SET", "HOME"],
        AutoPage::Drive => [
            GearSelect::Park.name(),
            GearSelect::Reverse.name(),
            GearSelect::Neutral.name(),
            GearSelect::Drive.name(),
            GearSelect::Manual.name(),
        ],
        AutoPage::Lights => ["LO", "HI", "FOG", "DRL", "INT"],
        AutoPage::Flir => ["CAM", "WHOT", "GHOT", "GATE", "SET"],
        AutoPage::Collision => ["F", "FL", "FR", "R", "RST"],
        AutoPage::Setup => ["UNIT", "OBD", "CAN", "BRT", "HOME"],
        _ => ["P", "R", "N", "D", "M"],
    };
    // left[0]=OSB20 … left[4]=OSB16
    let left: Osb5 = match page {
        AutoPage::Drive => ["OBD", "SET", "4L", "4H", "2H"],
        AutoPage::Tpm => ["OBD", "SET", "BAR", "kPa", "PSI"],
        AutoPage::Clim => ["OBD", "SET", "DEF", "FAN+", "AC"],
        AutoPage::Attitude => ["OBD", "SET", "ATT", "MAP", "CLST"],
        AutoPage::Map => ["OBD", "SET", "ATT", "MAP", "N-UP"],
        _ => ["OBD", "SET", "ATT", "MAP", "2H"],
    };
    (top, right, bottom, left)
}

fn chrome(
    page: &mut Page,
    pal: &Palette,
    which: AutoPage,
    bezel: &BezelState,
    v: &VehicleSnapshot,
) {
    let b = page.bounds.inset(2);
    let (top, right, bottom, left) = legends(which, v);
    osb_chrome(
        page.surface,
        b,
        &top,
        &right,
        &bottom,
        &left,
        page.font_px * 0.65,
        pal.dim,
        bezel.last_osb,
    );
    let c = content_after_osb(b, page.font_px * 0.65);
    page.label_centered(
        c.center().0 as f32,
        c.y as f32 + page.font_px * 0.45,
        which.title(),
        pal.primary,
    );
}

fn content(page: &Page) -> Rect {
    let b = page.bounds.inset(2);
    let c = content_after_osb(b, page.font_px * 0.65);
    Rect::new(
        c.x,
        c.y + (page.font_px as i32) + 4,
        c.w,
        (c.h - (page.font_px as i32) * 2 - 8).max(40),
    )
}

fn tape(
    s: &mut Surface,
    r: Rect,
    label: &'static str,
    value: f32,
    color: crate::Color,
    font_px: f32,
    horiz: bool,
) {
    tape_gauge(
        s,
        r,
        TapeOpts {
            orientation: if horiz {
                TapeOrientation::Horizontal
            } else {
                TapeOrientation::Vertical
            },
            font_px,
            color,
            value: value.clamp(0.0, 1.0),
            label,
        },
    );
}

pub fn draw_auto(
    page: &mut Page,
    which: AutoPage,
    pal: &Palette,
    bezel: &BezelState,
    v: &VehicleSnapshot,
    t: f32,
) {
    draw_auto_with_video(page, which, pal, bezel, v, t, None);
}

/// Draw auto page; optional live greyscale camera frame for FLIR.
pub fn draw_auto_with_video(
    page: &mut Page,
    which: AutoPage,
    pal: &Palette,
    bezel: &BezelState,
    v: &VehicleSnapshot,
    t: f32,
    cam_frame: Option<&GreyFrame>,
) {
    page.clear();
    page.surface.clear(pal.glass);
    page.bezel();
    chrome(page, pal, which, bezel, v);
    let c = content(page);
    let fh = page.font_px;

    match which {
        AutoPage::Cluster => {
            // Tach (round) + speed (value) + throttle strip + gear
            let tach_w = (c.w as f32 * 0.5) as i32;
            let rpm_n = (v.rpm / v.rpm_redline).clamp(0.0, 1.1);
            round_gauge(
                page.surface,
                Rect::new(c.x, c.y, tach_w, c.h - 28),
                RoundGaugeOpts {
                    value: rpm_n.min(1.0),
                    redline: Some(0.9),
                    label: "RPM",
                    color: pal.primary,
                    font_px: fh * 0.85,
                    ..Default::default()
                },
            );
            let sx = c.x + tach_w + 4;
            let spd = v.speed_unit.from_mph(v.speed_mph);
            value_readout(
                page.surface,
                sx as f32 + (c.w - tach_w) as f32 * 0.45,
                c.y as f32 + c.h as f32 * 0.28,
                "SPD",
                &format!("{:.0}", spd),
                v.speed_unit.name(),
                pal.readout,
                fh * 0.75,
                fh * 2.2,
            );
            let gear_s = if matches!(v.gear, GearSelect::Manual) {
                format!("{} {}", v.gear.label(), v.gear_num)
            } else {
                v.gear.label().to_string()
            };
            numeric_readout(
                page.surface,
                sx as f32 + (c.w - tach_w) as f32 * 0.45,
                c.y as f32 + c.h as f32 * 0.55,
                &gear_s,
                pal.nav,
                fh * 1.1,
            );
            label(
                page.surface,
                sx as f32 + 4.0,
                c.bottom() as f32 - fh * 2.2,
                "TPS",
                pal.dim,
                fh * 0.7,
            );
            progress_strip(
                page.surface,
                Rect::new(sx, c.bottom() - 20, c.w - tach_w - 8, 12),
                v.throttle,
                pal.caution,
                pal.structure,
            );
            numeric_readout(
                page.surface,
                c.x as f32 + 40.0,
                c.bottom() as f32 - 8.0,
                &format!("{:.0} RPM", v.rpm),
                pal.dim,
                fh * 0.75,
            );
        }
        AutoPage::Fuel => {
            let cols = 3;
            let tw = (c.w - 8) / cols;
            for (i, (lab, val, col)) in [
                ("FUEL", v.fuel, pal.primary),
                ("BATT", v.battery, pal.nav),
                ("LOAD", v.load, pal.caution),
            ]
            .iter()
            .enumerate()
            {
                tape(
                    page.surface,
                    Rect::new(c.x + i as i32 * (tw + 2), c.y, tw, c.h),
                    lab,
                    *val,
                    *col,
                    fh * 0.8,
                    false,
                );
            }
        }
        AutoPage::Temps => {
            // Horizontal total + vertical stack of engine fluids (jet FUEL page language).
            tape(
                page.surface,
                Rect::new(c.x, c.y, c.w, (c.h as f32 * 0.2) as i32),
                "EGT",
                v.exhaust_temp,
                pal.warning,
                fh * 0.75,
                true,
            );
            let ty = c.y + (c.h as f32 * 0.22) as i32;
            let th = c.h - (ty - c.y);
            let n = 5;
            let tw = (c.w - 4) / n;
            for (i, (lab, val, col)) in [
                ("OIL", v.oil_temp, pal.caution),
                ("COOL", v.coolant, pal.nav),
                ("TRNS", v.trans_temp, pal.caution),
                ("IAT", v.iat, pal.primary),
                ("MAF", v.maf, pal.readout),
            ]
            .iter()
            .enumerate()
            {
                tape(
                    page.surface,
                    Rect::new(c.x + i as i32 * tw, ty, tw - 2, th),
                    lab,
                    *val,
                    *col,
                    fh * 0.7,
                    false,
                );
            }
        }
        AutoPage::Drive => {
            value_readout(
                page.surface,
                c.center().0 as f32,
                c.y as f32 + c.h as f32 * 0.28,
                "GEAR",
                v.gear.name(),
                v.gear.label(),
                pal.readout,
                fh,
                fh * 3.0,
            );
            if matches!(v.gear, GearSelect::Manual) {
                numeric_readout(
                    page.surface,
                    c.center().0 as f32,
                    c.y as f32 + c.h as f32 * 0.48,
                    &format!("GEAR {}", v.gear_num),
                    pal.primary,
                    fh * 1.2,
                );
            }
            numeric_readout(
                page.surface,
                c.center().0 as f32,
                c.y as f32 + c.h as f32 * 0.62,
                v.drive.label(),
                pal.nav,
                fh * 1.1,
            );
            let items = [
                StatusItem {
                    label: "2H",
                    on: matches!(v.drive, DriveMode::TwoHigh),
                },
                StatusItem {
                    label: "4H",
                    on: matches!(v.drive, DriveMode::FourHigh),
                },
                StatusItem {
                    label: "4L",
                    on: matches!(v.drive, DriveMode::FourLow),
                },
            ];
            status_grid(
                page.surface,
                Rect::new(c.x + 8, c.bottom() - 50, c.w - 16, 44),
                &items,
                3,
                fh * 0.9,
                pal.primary,
                pal.dim,
            );
        }
        AutoPage::Lights => {
            let items = [
                StatusItem {
                    label: "LO BEAM",
                    on: v.light_low,
                },
                StatusItem {
                    label: "HI BEAM",
                    on: v.light_high,
                },
                StatusItem {
                    label: "DRL",
                    on: v.light_drive,
                },
                StatusItem {
                    label: "FOG",
                    on: v.light_fog,
                },
                StatusItem {
                    label: "BRAKE",
                    on: v.light_brake,
                },
                StatusItem {
                    label: "TURN L",
                    on: v.light_turn_l,
                },
                StatusItem {
                    label: "TURN R",
                    on: v.light_turn_r,
                },
                StatusItem {
                    label: "INT",
                    on: v.light_interior,
                },
            ];
            status_grid(
                page.surface,
                c.inset(4),
                &items,
                2,
                fh * 0.85,
                pal.caution,
                pal.dim,
            );
        }
        AutoPage::Tpm => {
            tire_grid(
                page.surface,
                c.inset(6),
                v.tire_fl,
                v.tire_fr,
                v.tire_rl,
                v.tire_rr,
                fh * 0.85,
                pal.primary,
                pal.warning,
                pal.structure,
            );
            numeric_readout(
                page.surface,
                c.center().0 as f32,
                c.bottom() as f32 - 6.0,
                "PSI  /  °C",
                pal.dim,
                fh * 0.7,
            );
        }
        AutoPage::Body => {
            let items = [
                StatusItem {
                    label: "DR FL",
                    on: v.door_fl,
                },
                StatusItem {
                    label: "DR FR",
                    on: v.door_fr,
                },
                StatusItem {
                    label: "DR RL",
                    on: v.door_rl,
                },
                StatusItem {
                    label: "DR RR",
                    on: v.door_rr,
                },
                StatusItem {
                    label: "HATCH",
                    on: v.door_hatch,
                },
                StatusItem {
                    label: "BELT FL",
                    on: v.belt_fl,
                },
                StatusItem {
                    label: "BELT FR",
                    on: v.belt_fr,
                },
                StatusItem {
                    label: "BELT RL",
                    on: v.belt_rl,
                },
                StatusItem {
                    label: "BELT RR",
                    on: v.belt_rr,
                },
            ];
            // ON = closed door / buckled belt (green); open/unbuckled = dim
            status_grid(
                page.surface,
                c.inset(4),
                &items,
                3,
                fh * 0.8,
                pal.primary,
                pal.warning,
            );
            label(
                page.surface,
                c.x as f32 + 4.0,
                c.bottom() as f32 - fh,
                "ON=CLOSED/BUCKLED",
                pal.dim,
                fh * 0.65,
            );
        }
        AutoPage::Clim => {
            value_readout(
                page.surface,
                c.x as f32 + c.w as f32 * 0.28,
                c.y as f32 + c.h as f32 * 0.25,
                "OUT",
                &format!("{:.0}", v.temp_out_c),
                "C",
                pal.nav,
                fh * 0.8,
                fh * 1.8,
            );
            value_readout(
                page.surface,
                c.x as f32 + c.w as f32 * 0.72,
                c.y as f32 + c.h as f32 * 0.25,
                "IN",
                &format!("{:.0}", v.temp_in_c),
                "C",
                pal.readout,
                fh * 0.8,
                fh * 1.8,
            );
            tape(
                page.surface,
                Rect::new(c.x + 4, c.y + c.h / 2, c.w / 2 - 8, c.h / 2 - 4),
                "FAN",
                v.hvac_fan,
                pal.primary,
                fh * 0.75,
                false,
            );
            let items = [
                StatusItem {
                    label: "A/C",
                    on: v.hvac_ac,
                },
                StatusItem {
                    label: "DEFROST",
                    on: v.hvac_defrost,
                },
            ];
            status_grid(
                page.surface,
                Rect::new(c.x + c.w / 2, c.y + c.h / 2, c.w / 2 - 4, c.h / 2 - 4),
                &items,
                1,
                fh * 0.9,
                pal.nav,
                pal.dim,
            );
            numeric_readout(
                page.surface,
                c.center().0 as f32,
                c.y as f32 + c.h as f32 * 0.42,
                &format!("SET {:.0}C", v.hvac_set_c),
                pal.caution,
                fh,
            );
        }
        AutoPage::Flir => {
            let frame = c.inset(c.w / 14);
            let fw = (frame.w as u32).clamp(80, 320);
            let fh_px = (frame.h as u32).clamp(60, 240);
            let owned;
            let grey = if let Some(f) = cam_frame {
                f
            } else {
                owned = GreyFrame::resolve(t, fw, fh_px);
                &owned
            };
            blit_grey_flir(page.surface, frame, grey, pal.primary, pal.structure);
            let (cx, cy) = frame.center();
            crosshair(page.surface, cx, cy, 22, 6, pal.dim);
            track_gate(
                page.surface,
                cx + ((t * 0.6).sin() * 20.0) as i32,
                cy + ((t * 0.45).cos() * 12.0) as i32,
                12,
                pal.readout,
            );
            label(
                page.surface,
                frame.x as f32 + 4.0,
                frame.y as f32 + 4.0,
                "FLIR  G-HOT",
                pal.readout,
                fh * 0.75,
            );
            let src = if cam_frame.is_some() {
                "SRC  CAM"
            } else if std::env::var_os("MFD_FLIR_PATH").is_some() {
                "SRC  FILE"
            } else {
                "SRC  SYN"
            };
            label(
                page.surface,
                frame.x as f32 + 4.0,
                frame.bottom() as f32 - fh,
                src,
                pal.dim,
                fh * 0.7,
            );
        }
        AutoPage::Collision => {
            let rng = RangeSnapshot::from_env_or_synthetic(t);
            range_display(
                page.surface,
                c.inset(4),
                &rng,
                pal.structure,
                pal.primary,
                pal.caution,
                pal.warning,
                pal.readout,
            );
            label(
                page.surface,
                c.x as f32 + 4.0,
                c.bottom() as f32 - fh,
                "PARK/COLLISION  ·  MFD_RANGE=m",
                pal.dim,
                fh * 0.65,
            );
        }
        AutoPage::Attitude => {
            // Attitude ball (left) + heading (right) — pitch/roll/heading.
            let ball_w = (c.w as f32 * 0.58) as i32;
            let sky = CYAN;
            let ground = rgb(120, 90, 40);
            attitude_ball(
                page.surface,
                Rect::new(c.x, c.y, ball_w, c.h - 8),
                v.pitch_deg,
                v.roll_deg,
                v.heading_deg,
                sky,
                ground,
                pal.readout,
                pal.dim,
            );
            let hx = c.x + ball_w + 4;
            let hw = c.w - ball_w - 8;
            heading_display(
                page.surface,
                Rect::new(hx, c.y + 8, hw, c.h / 3),
                v.heading_deg,
                pal.readout,
                pal.dim,
                fh,
            );
            heading_rose(
                page.surface,
                hx + hw / 2,
                c.y + c.h * 2 / 3,
                (hw.min(c.h / 2) / 2 - 4).max(28),
                v.heading_deg,
                pal.primary,
                pal.dim,
                fh * 0.85,
            );
            label(
                page.surface,
                hx as f32,
                c.bottom() as f32 - fh * 2.0,
                &format!("P {:+.0}°  R {:+.0}°", v.pitch_deg, v.roll_deg),
                pal.dim,
                fh * 0.7,
            );
        }
        AutoPage::Map => {
            schematic_topo_map(
                page.surface,
                c.inset(4),
                v.heading_deg,
                v.speed_mph,
                t,
                pal.structure,
                pal.nav,
                pal.caution,
                pal.readout,
                pal.primary,
            );
            label(
                page.surface,
                c.x as f32 + 4.0,
                c.y as f32 + fh + 2.0,
                &format!(
                    "HDG {:05.1}°  {:.0} {}",
                    ((v.heading_deg % 360.0) + 360.0) % 360.0,
                    v.speed_unit.from_mph(v.speed_mph),
                    v.speed_unit.name()
                ),
                pal.readout,
                fh * 0.75,
            );
        }
        AutoPage::Obd => {
            let lines = [
                format!("RPM   {:.0}", v.rpm),
                format!(
                    "VSS   {:.0} {}",
                    v.speed_unit.from_mph(v.speed_mph),
                    v.speed_unit.name()
                ),
                format!("TPS   {:.0}%", v.throttle * 100.0),
                format!("FUEL  {:.0}%", v.fuel * 100.0),
                format!("ECT   {:.0}%", v.coolant * 100.0),
                format!("TFT   {:.0}%", v.trans_temp * 100.0),
                format!("IAT   {:.0}%", v.iat * 100.0),
                format!("DTC   {}", v.dtc_count),
            ];
            let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            list_menu(
                page.surface,
                c,
                &refs,
                None,
                fh * 0.9,
                pal.primary,
                pal.readout,
            );
        }
        AutoPage::Setup => {
            list_menu(
                page.surface,
                Rect::new(c.x, c.y, c.w, c.h / 2),
                &[
                    "SPD UNIT  → bottom OSB",
                    "FLIR  MFD_FLIR_PATH=file.pgm",
                    "OBD  host inject VehicleSnapshot",
                    "CAN  future BezelSource",
                    "JET  Tab domain",
                ],
                Some(0),
                fh * 0.8,
                pal.primary,
                pal.readout,
            );
            caution_box(
                page.surface,
                Rect::new(c.x + 12, c.y + c.h / 2 + 4, c.w - 24, c.h / 3),
                &format!("UNIT {}", v.speed_unit.name()),
                fh,
                pal.nav,
            );
        }
    }
}

/// Legacy entry used by older demos (normalized OBD only).
pub fn draw_auto_obd(
    page: &mut Page,
    which: AutoPage,
    pal: &Palette,
    bezel: &BezelState,
    obd: &ObdSnapshot,
    t: f32,
) {
    let mut v = VehicleSnapshot::default();
    v.rpm = obd.rpm * v.rpm_redline;
    v.speed_mph = obd.speed * 120.0;
    v.fuel = obd.fuel;
    v.coolant = obd.coolant;
    v.trans_temp = obd.trans_temp;
    v.battery = obd.battery;
    v.throttle = obd.throttle;
    v.load = obd.load;
    v.dtc_count = obd.dtc_count;
    draw_auto_with_video(page, which, pal, bezel, &v, t, None);
}

pub fn rpm_norm(rpm: f32, redline: f32) -> f32 {
    if redline <= 0.0 {
        0.0
    } else {
        (rpm / redline).clamp(0.0, 1.0)
    }
}
