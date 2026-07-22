//! Automotive **page layouts** — same bezel / OSB model as jet, vehicle data.
//!
//! Optional live OBD: feature `obd` + native [`crate::obd`] stack
//! (`MFD_OBD_BT` / `MFD_OBD_PORT` / `MFD_OBD_REPLAY`).
//!
//! Vehicle under test: see [`vehicle_profile`] and `docs/vehicle.md`
//! (2019 SuperCrew 2.7 EcoBoost 4×4 · Sync 3 · display only).
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

pub mod boot;
pub mod caps;
pub mod channels;
pub mod format_select;
pub mod probe;
pub mod vehicle_profile;

pub use boot::draw_bit_screen;
pub use caps::{BitLine, BitState, FeatureCaps, VehicleCaps};
pub use format_select::{AutoFormatSelect, FormatSelectAction, FormatSlot};

use crate::bezel::BezelState;
use crate::color::WHITE;
use crate::color::rgb;
use crate::Color;
use crate::geom::Rect;
use crate::page::Page;
use crate::palette::Palette;
use crate::video::{blit_grey_flir, GreyFrame};
use crate::warn::slot_flash_owner;
use crate::warn::{self, ActiveWarn, WarnId, WarnLevel};
use crate::widget::{
    attitude_ball, content_after_osb, crosshair, heading_display, label, master_warn_strip,
    osb_chrome_ex, progress_strip, range_display, round_gauge, schematic_topo_map, status_grid,
    status_grid_flash, tape_gauge, tire_grid, track_gate, value_readout, RangeSnapshot,
    RoundGaugeOpts, StatusItem, TapeOpts, TapeOrientation, TireReading,
};
use crate::Surface;

// ─── Data model ──────────────────────────────────────────────────────────────

/// DTC class for fault glass (read-only inventory).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DtcKind {
    Stored,
    Pending,
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

/// One trouble code line for the FAULT page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DtcEntry {
    pub code: String,
    pub kind: DtcKind,
}

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

/// Full vehicle snapshot for auto pages (live OBD/CAN only — never synthetic).
///
/// Prefer **engineering units** on glass. Normalized 0..1 fields remain for tape widgets.
#[derive(Clone, Debug)]
pub struct VehicleSnapshot {
    pub rpm: f32,
    pub rpm_redline: f32,
    pub speed_mph: f32,
    pub throttle: f32,
    /// Fuel level 0..1
    pub fuel: f32,
    /// Battery as 0..1 (legacy tapes); use [`Self::battery_v`] for display.
    pub battery: f32,
    pub battery_v: f32,
    pub load: f32,
    /// Normalized temps 0..1 (tapes)
    pub oil_temp: f32,
    pub coolant: f32,
    pub trans_temp: f32,
    pub iat: f32,
    pub maf: f32,
    pub exhaust_temp: f32,
    /// Absolute °C for numeric glass
    pub oil_temp_c: f32,
    pub coolant_c: f32,
    pub trans_temp_c: f32,
    pub iat_c: f32,
    pub exhaust_temp_c: f32,
    pub maf_gps: f32,
    pub fuel_pressure_kpa: f32,
    pub gear: GearSelect,
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
    pub wheel_fl_kph: f32,
    pub wheel_fr_kph: f32,
    pub wheel_rl_kph: f32,
    pub wheel_rr_kph: f32,
    pub brake_pedal: bool,
    pub park_brake: bool,
    pub steer_deg: f32,
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
    pub dtcs: Vec<DtcEntry>,
    pub speed_unit: SpeedUnit,
    pub pitch_deg: f32,
    pub roll_deg: f32,
    pub heading_deg: f32,
    pub vin: String,
    // ── OBD / Bluetooth link (OWN · SETUP · BUS · status strip) ───────────
    /// `BT` · `SERIAL` · `REPLAY` · `OFF`
    pub bus_kind: String,
    /// MAC, serial path, or replay path.
    pub bus_addr: String,
    /// RFCOMM channel (BT) or `-`.
    pub bus_channel: String,
    /// ELM `ATI` identity string.
    pub bus_adapter: String,
    /// ELM `ATDP` protocol string.
    pub bus_proto: String,
    /// `LIVE` · `BIT` · `ERR` · `SEARCH` · `RECONN` · `OFF`
    pub bus_state: String,
    /// Last bus error (empty if none).
    pub bus_error: String,
    /// Poll tick count from feed.
    pub bus_ticks: u64,
    /// Short capture directory name when logging.
    pub bus_capture: String,
}

impl VehicleSnapshot {
    /// Dense lines for OWN / SETUP bus block.
    pub fn bus_link_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("LINK  {}  {}", self.bus_state, self.bus_kind),
            format!(
                "ADDR  {}",
                if self.bus_addr.is_empty() {
                    "—"
                } else {
                    &self.bus_addr
                }
            ),
        ];
        if self.bus_kind == "BT" || !self.bus_channel.is_empty() && self.bus_channel != "-" {
            lines.push(format!("CH    {}", self.bus_channel));
        }
        lines.push(format!(
            "ADPT  {}",
            if self.bus_adapter.is_empty() {
                "—"
            } else {
                &self.bus_adapter
            }
        ));
        lines.push(format!(
            "PROT  {}",
            if self.bus_proto.is_empty() {
                "—"
            } else {
                &self.bus_proto
            }
        ));
        lines.push(format!("TICK  {}", self.bus_ticks));
        if !self.bus_capture.is_empty() {
            lines.push(format!("CAP   {}", self.bus_capture));
        }
        if !self.bus_error.is_empty() {
            let err = if self.bus_error.len() > 36 {
                format!("{}…", &self.bus_error[..35])
            } else {
                self.bus_error.clone()
            };
            lines.push(format!("ERR   {err}"));
        }
        lines.push("MODE  DISPLAY ONLY".into());
        lines
    }

    /// One-line strip for bottom glass status.
    pub fn bus_status_short(&self) -> String {
        let addr = if self.bus_addr.len() > 17 {
            // show last 8 of MAC-like strings when long
            let s = &self.bus_addr;
            if s.contains(':') && s.len() >= 8 {
                s[s.len().saturating_sub(8)..].to_string()
            } else {
                format!("{}…", &s[..14.min(s.len())])
            }
        } else if self.bus_addr.is_empty() {
            "—".into()
        } else {
            self.bus_addr.clone()
        };
        if !self.bus_error.is_empty() {
            format!("BT ERR · {addr}")
        } else {
            format!("{} {} · {addr}", self.bus_kind, self.bus_state)
        }
    }
}

impl Default for VehicleSnapshot {
    /// Empty / no-data defaults. Glass stays blank until live OBD fills fields.
    fn default() -> Self {
        Self {
            rpm: 0.0,
            rpm_redline: 6500.0,
            speed_mph: 0.0,
            throttle: 0.0,
            fuel: 0.0,
            battery: 0.0,
            battery_v: 0.0,
            load: 0.0,
            oil_temp: 0.0,
            coolant: 0.0,
            trans_temp: 0.0,
            iat: 0.0,
            maf: 0.0,
            exhaust_temp: 0.0,
            oil_temp_c: 0.0,
            coolant_c: 0.0,
            trans_temp_c: 0.0,
            iat_c: 0.0,
            exhaust_temp_c: 0.0,
            maf_gps: 0.0,
            fuel_pressure_kpa: 0.0,
            gear: GearSelect::Park,
            gear_num: 0,
            drive: DriveMode::TwoHigh,
            // Lights off until body/BCM data exists (never invent ON).
            light_low: false,
            light_high: false,
            light_drive: false,
            light_fog: false,
            light_brake: false,
            light_turn_l: false,
            light_turn_r: false,
            light_interior: false,
            tire_fl: TireReading {
                pressure: 0.0,
                temp_c: 0.0,
                alert: false,
            },
            tire_fr: TireReading {
                pressure: 0.0,
                temp_c: 0.0,
                alert: false,
            },
            tire_rl: TireReading {
                pressure: 0.0,
                temp_c: 0.0,
                alert: false,
            },
            tire_rr: TireReading {
                pressure: 0.0,
                temp_c: 0.0,
                alert: false,
            },
            wheel_fl_kph: 0.0,
            wheel_fr_kph: 0.0,
            wheel_rl_kph: 0.0,
            wheel_rr_kph: 0.0,
            brake_pedal: false,
            park_brake: false,
            steer_deg: 0.0,
            // Doors/belts: **true = closed/latched**. Unknown defaults closed so
            // empty OBD does not fire DOOR AJAR at speed (no body DID yet).
            door_fl: true,
            door_fr: true,
            door_rl: true,
            door_rr: true,
            door_hatch: true,
            belt_fl: true,
            belt_fr: true,
            belt_rl: true,
            belt_rr: true,
            temp_out_c: 0.0,
            temp_in_c: 0.0,
            hvac_fan: 0.0,
            hvac_set_c: 0.0,
            hvac_ac: false,
            hvac_defrost: false,
            dtc_count: 0,
            dtcs: Vec::new(),
            speed_unit: SpeedUnit::Mph,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            heading_deg: 0.0,
            vin: String::new(),
            bus_kind: "OFF".into(),
            bus_addr: String::new(),
            bus_channel: "—".into(),
            bus_adapter: "—".into(),
            bus_proto: "—".into(),
            bus_state: "OFF".into(),
            bus_error: String::new(),
            bus_ticks: 0,
            bus_capture: String::new(),
        }
    }
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

/// Systems pages (fighter-style banks). Vehicle CMFD product path only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoPage {
    /// Powerplant — ENG
    Eng,
    Fuel,
    /// Fluids / temperatures — FLUID
    Fluid,
    /// Electrical — ELEC
    Elec,
    Drive,
    /// Chassis: TPM + wheels + brake — CHAS
    Chas,
    Body,
    Lights,
    Clim,
    /// Camera / FLIR — CAM
    Cam,
    /// Park / collision range — RNG
    Range,
    Attitude,
    Map,
    Faults,
    /// All channels numeric dump — BUS
    Bus,
    /// Ownship identity — OWN
    Own,
    Setup,
}

impl AutoPage {
    pub const ALL: &'static [AutoPage] = &[
        AutoPage::Eng,
        AutoPage::Fuel,
        AutoPage::Fluid,
        AutoPage::Elec,
        AutoPage::Drive,
        AutoPage::Chas,
        AutoPage::Body,
        AutoPage::Lights,
        AutoPage::Clim,
        AutoPage::Cam,
        AutoPage::Range,
        AutoPage::Attitude,
        AutoPage::Map,
        AutoPage::Faults,
        AutoPage::Bus,
        AutoPage::Own,
        AutoPage::Setup,
    ];

    pub fn name(self) -> &'static str {
        match self {
            AutoPage::Eng => "ENG",
            AutoPage::Fuel => "FUEL",
            AutoPage::Fluid => "FLUD",
            AutoPage::Elec => "ELEC",
            AutoPage::Drive => "DRV",
            AutoPage::Chas => "CHAS",
            AutoPage::Body => "BODY",
            AutoPage::Lights => "LITE",
            AutoPage::Clim => "CLIM",
            AutoPage::Cam => "CAM",
            AutoPage::Range => "RNG",
            AutoPage::Attitude => "ATT",
            AutoPage::Map => "MAP",
            AutoPage::Faults => "DTC",
            AutoPage::Bus => "BUS",
            AutoPage::Own => "OWN",
            AutoPage::Setup => "SET",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            AutoPage::Eng => "ENGINE",
            AutoPage::Fuel => "FUEL / ENERGY",
            AutoPage::Fluid => "FLUIDS / TEMP",
            AutoPage::Elec => "ELECTRICAL",
            AutoPage::Drive => "DRIVE",
            AutoPage::Chas => "CHASSIS",
            AutoPage::Body => "BODY",
            AutoPage::Lights => "LIGHTS",
            AutoPage::Clim => "CLIMATE",
            AutoPage::Cam => "CAMERA / FLIR",
            AutoPage::Range => "RANGE",
            AutoPage::Attitude => "ATTITUDE",
            AutoPage::Map => "MAP",
            AutoPage::Faults => "FAULT CODES",
            AutoPage::Bus => "BUS / CHANNELS",
            AutoPage::Own => "OWNSHIP",
            AutoPage::Setup => "SETUP",
        }
    }

    /// Left support OSB → format jump (BUS / SET / DTC).
    pub fn from_left_support_osb(osb: u8) -> Option<AutoPage> {
        match osb {
            20 => Some(AutoPage::Bus),
            19 => Some(AutoPage::Setup),
            16 => Some(AutoPage::Faults),
            _ => None,
        }
    }
}

type Osb5 = [&'static str; 5];

/// Format-local options (top / right / left). Bottom is format-select chrome.
fn option_legends(
    page: AutoPage,
    v: &VehicleSnapshot,
    feat: Option<&crate::auto::caps::FeatureCaps>,
) -> (Osb5, Osb5, Osb5) {
    let fog_ok = feat.map(|f| f.fog_lights).unwrap_or(true);
    let left: Osb5 = ["BUS", "SET", "", "", "DTC"];
    match page {
        // Display-only: gear is status on glass, not OSB "control" toggles.
        AutoPage::Drive => (
            ["UNIT", v.speed_unit.name(), "", "", ""],
            ["", "", "", "", ""],
            left,
        ),
        AutoPage::Fuel | AutoPage::Fluid | AutoPage::Elec | AutoPage::Eng => (
            ["UNIT", v.speed_unit.name(), "", "", ""],
            ["", "", "", "", ""],
            left,
        ),
        AutoPage::Lights => (
            ["LO", "HI", if fog_ok { "FOG" } else { "" }, "DRL", "INT"],
            ["", "", "", "", ""],
            left,
        ),
        AutoPage::Cam => (["", "", "", "", ""], ["", "", "", "", ""], left),
        AutoPage::Range => (["", "", "", "", ""], ["", "", "", "", ""], left),
        AutoPage::Own => (["", "", "", "", ""], ["", "", "", "", ""], left),
        // SET: UNIT + PAL (color mode) — production hardware path for prefs.
        AutoPage::Setup => (
            ["UNIT", v.speed_unit.name(), "PAL", "MODE", ""],
            ["", "", "", "", ""],
            left,
        ),
        AutoPage::Clim => (["", "", "", "", ""], ["", "", "", "", ""], left),
        _ => (["", "", "", "", ""], ["", "", "", "", ""], left),
    }
}

/// Bottom L→R = OSB 15..11: OWN · slotA · slotB · slotC · DCLT
///
/// SOI is a green box on the active slot (not a `*` prefix). DCLT keeps one mnemonic.
fn format_select_bottom(fmt: &AutoFormatSelect) -> Osb5 {
    let [a, b, c] = fmt.slot_labels();
    ["OWN", a, b, c, "DCLT"]
}

/// Dense numeric matrix (preferred glass style).
fn numeric_matrix(
    s: &mut Surface,
    rect: Rect,
    lines: &[String],
    font_px: f32,
    color: crate::Color,
    cols: i32,
) {
    let cols = cols.clamp(1, 3);
    let n = lines.len() as i32;
    let rows = (n + cols - 1) / cols;
    let cell_w = rect.w / cols;
    let cell_h = ((rect.h as f32) / rows.max(1) as f32).max(font_px + 2.0) as i32;
    for (i, line) in lines.iter().enumerate() {
        let col = i as i32 % cols;
        let row = i as i32 / cols;
        let x = rect.x + col * cell_w + 2;
        let y = rect.y + row * cell_h;
        if y + font_px as i32 > rect.bottom() {
            break;
        }
        label(s, x as f32, y as f32, line, color, font_px);
    }
}

#[allow(clippy::too_many_arguments)]
fn chrome(
    page: &mut Page,
    pal: &Palette,
    which: AutoPage,
    bezel: &BezelState,
    v: &VehicleSnapshot,
    allowed: Option<&[AutoPage]>,
    feat: Option<&crate::auto::caps::FeatureCaps>,
    fmt: Option<&AutoFormatSelect>,
    warns: Option<&[ActiveWarn]>,
    t: f32,
) {
    let b = page.bounds.inset(2);
    let allow = allowed.unwrap_or(AutoPage::ALL);
    let menu = fmt.map(|f| f.menu_open).unwrap_or(false);
    let (top, right, bottom, left) = if let Some(f) = fmt {
        if f.menu_open {
            format_select::master_menu_legends(allow)
        } else {
            let (t, r, l) = option_legends(which, v, feat);
            (t, r, format_select_bottom(f), l)
        }
    } else {
        let (t, r, l) = option_legends(which, v, feat);
        (t, r, ["OWN", "", "", "", "DCLT"], l)
    };

    // SOI = OSB that owns **displayed** page (slot or support), not last press.
    let soi = if menu {
        fmt.map(|f| f.menu_target.osb())
    } else {
        fmt.and_then(|f| f.soi_osb_for_page(which)).or(match which {
            AutoPage::Own => Some(15),
            AutoPage::Faults => Some(16),
            AutoPage::Setup => Some(19),
            AutoPage::Bus => Some(20),
            _ => None,
        })
    };
    // Press flash only while held (does not replace SOI after release).
    let active = (1..=20u8).find(|&id| bezel.is_down(id)).or(soi);

    // Off-glass owning format slot flashes (warning-class only).
    let flash_page = if menu {
        None
    } else {
        warns.and_then(slot_flash_owner).filter(|&p| p != which)
    };
    let flash_osb = flash_page.and_then(|p| fmt.and_then(|f| f.slot_osb_for_page(p)));
    let flash_lit = warn::flash_warn_on(t);

    // SYM rocker: dim softkey ink (structure intensity).
    let sym = bezel.symbology.clamp(0.2, 1.0);
    let soft_dim = scale_color_rgb(pal.dim, sym);
    let soft_soi = scale_color_rgb(pal.primary, sym.max(0.55));

    osb_chrome_ex(
        page.surface,
        b,
        &top,
        &right,
        &bottom,
        &left,
        page.font_px * 0.7,
        soft_dim,
        soft_soi,
        active,
        flash_osb,
        flash_lit,
        pal.warning,
    );

    let c = content_after_osb(b, page.font_px * 0.7);
    // Sparse format ID (jet-style); full title only when not menu.
    let head = if menu {
        "MASTER MENU".to_string()
    } else {
        which.name().to_string()
    };
    page.label_centered(
        c.center().0 as f32,
        c.y as f32 + page.font_px * 0.45,
        &head,
        WHITE,
    );
    // Lab-only nav cue (not MLU glass). Enable with MFD_LAB_CHROME=1.
    let lab_chrome = matches!(
        std::env::var("MFD_LAB_CHROME").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes")
    );
    if lab_chrome {
        let nav = if menu {
            "PICK FORMAT OSB 1-20  ·  SLOT KEY CLOSES MENU"
        } else {
            "SOI BOX = GLASS OWNER  ·  [ ] PREV/NEXT  ·  1234567890QWERTYUIOP = OSB 1-20"
        };
        page.label_centered(
            c.center().0 as f32,
            c.y as f32 + page.font_px * 1.2,
            nav,
            pal.dim,
        );
    }
    // VIN only on OWN (identity page) — not every format header.
    if !v.vin.is_empty() && !menu && matches!(which, AutoPage::Own) {
        let os = format!("OS  {}", short_vin(&v.vin));
        page.label_centered(
            c.center().0 as f32,
            c.y as f32 + page.font_px * 1.25,
            &os,
            pal.readout,
        );
    }
}

fn scale_color_rgb(c: Color, f: f32) -> Color {
    let a = (c >> 24) & 0xFF;
    let r = ((((c >> 16) & 0xFF) as f32) * f).clamp(0.0, 255.0) as u32;
    let g = ((((c >> 8) & 0xFF) as f32) * f).clamp(0.0, 255.0) as u32;
    let b = (((c & 0xFF) as f32) * f).clamp(0.0, 255.0) as u32;
    (a << 24) | (r << 16) | (g << 8) | b
}

/// Last 8 of VIN for tight chrome; full string if shorter.
pub fn short_vin(vin: &str) -> &str {
    let v = vin.trim();
    if v.len() > 8 {
        &v[v.len() - 8..]
    } else {
        v
    }
}

fn content(page: &Page) -> Rect {
    let b = page.bounds.inset(2);
    let c = content_after_osb(b, page.font_px * 0.65);
    // Short format ID band (more glass for gauges / ATT).
    let title_band = (page.font_px as i32) + 8;
    Rect::new(
        c.x,
        c.y + title_band,
        c.w,
        (c.h - title_band - (page.font_px as i32) - 4).max(40),
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
    draw_auto_with_video(page, which, pal, bezel, v, t, None, None, None, None);
}

/// Draw auto page; optional live greyscale camera frame for FLIR.
/// `caps` omits equipment that did not probe GO (fog, HSWM, …).
/// `fmt` drives MLU-class format select chrome + DCLT density.
/// `warns` drives master strip + flash fields.
#[allow(clippy::too_many_arguments)]
pub fn draw_auto_with_video(
    page: &mut Page,
    which: AutoPage,
    pal: &Palette,
    bezel: &BezelState,
    v: &VehicleSnapshot,
    t: f32,
    cam_frame: Option<&GreyFrame>,
    caps: Option<&VehicleCaps>,
    warns: Option<&[ActiveWarn]>,
    fmt: Option<&AutoFormatSelect>,
) {
    page.clear();
    page.surface.clear(pal.glass);
    page.bezel();
    // Use cached page list — never rebuild Vec every frame.
    let allowed_slice = caps.and_then(|c| {
        if c.ready && !c.page_list.is_empty() {
            Some(c.pages_cached())
        } else {
            None
        }
    });
    let feat = caps.map(|c| &c.features);
    let dclt = fmt.map(|f| f.dclt).unwrap_or(0);
    chrome(
        page,
        pal,
        which,
        bezel,
        v,
        allowed_slice,
        feat,
        fmt,
        warns,
        t,
    );
    let mut c = content(page);
    let fh = page.font_px;
    let flash = warn::flash_warn_on(t);

    // Global master strip on **every** format (incl. Master Menu) — Lockheed (c).
    if let Some(ws) = warns {
        if !ws.is_empty() {
            let has_w = ws.iter().any(|w| w.level == WarnLevel::Warning);
            let text = if ws.len() == 1 {
                ws[0].label.to_string()
            } else {
                format!("{} +{}", ws[0].label, ws.len() - 1)
            };
            let strip_h = (fh * 1.2) as i32 + 4;
            master_warn_strip(
                page.surface,
                Rect::new(c.x, c.y, c.w, strip_h),
                &text,
                if has_w { flash } else { warn::flash_on(t) },
                fh * 0.85,
            );
            c = Rect::new(c.x, c.y + strip_h + 2, c.w, (c.h - strip_h - 2).max(20));
        }
    }

    // Master Menu: empty glass — formats live on OSB legends only (MLU Fig 1-15).
    if fmt.map(|f| f.menu_open).unwrap_or(false) {
        let _ = (allowed_slice, fh, c);
        return;
    }

    match which {
        AutoPage::Eng => {
            // Primary: RPM gauge. Secondary: sparse ENG numerics (DCLT drops matrix).
            let tach_w = if dclt >= 2 {
                c.w - 8
            } else {
                (c.w as f32 * 0.48) as i32
            };
            let rpm_n = (v.rpm / v.rpm_redline).clamp(0.0, 1.1);
            round_gauge(
                page.surface,
                Rect::new(c.x, c.y, tach_w, c.h - 8),
                RoundGaugeOpts {
                    value: rpm_n.min(1.0),
                    redline: Some(0.9),
                    label: "RPM",
                    color: pal.primary,
                    font_px: fh * 0.8,
                    ..Default::default()
                },
            );
            // Digital tach readout under needle (always).
            value_readout(
                page.surface,
                c.x as f32 + tach_w as f32 * 0.5,
                c.y as f32 + c.h as f32 * 0.78,
                "rpm",
                &format!("{:.0}", v.rpm),
                "",
                pal.primary,
                fh * 0.55,
                fh * 1.3,
            );
            if dclt < 2 {
                let mut lines = vec![
                    format!("LOAD  {:.0} %", v.load * 100.0),
                    format!("TPS   {:.0} %", v.throttle * 100.0),
                    format!("MAF   {:.1} g/s", v.maf_gps),
                    format!("ECT   {:.0} C", v.coolant_c),
                ];
                if dclt == 0 {
                    lines.push(format!("OIL   {:.0} C", v.oil_temp_c));
                    lines.push(format!("IAT   {:.0} C", v.iat_c));
                }
                numeric_matrix(
                    page.surface,
                    Rect::new(c.x + tach_w + 4, c.y, c.w - tach_w - 6, c.h),
                    &lines,
                    fh * 0.85,
                    pal.readout,
                    1,
                );
            }
        }
        AutoPage::Fuel => {
            let bingo = warns
                .map(|w| w.iter().any(|x| x.id == WarnId::Bingo))
                .unwrap_or(false);
            let fuel_col = if bingo && flash {
                pal.warning
            } else {
                pal.primary
            };
            // Level tape + big % — useful widgets only.
            value_readout(
                page.surface,
                c.center().0 as f32,
                c.y as f32 + c.h as f32 * 0.28,
                if bingo { "BINGO" } else { "FUEL" },
                &format!("{:.0}", v.fuel * 100.0),
                "%",
                fuel_col,
                fh * 0.75,
                fh * 2.2,
            );
            tape(
                page.surface,
                Rect::new(c.x + 8, c.bottom() - 52, c.w - 16, 44),
                "FUEL",
                v.fuel,
                fuel_col,
                fh * 0.7,
                true,
            );
            if dclt == 0 {
                let lines = vec![
                    format!("FP    {:.0} kPa", v.fuel_pressure_kpa),
                    format!("BATT  {:.1} V", v.battery_v),
                ];
                numeric_matrix(
                    page.surface,
                    Rect::new(
                        c.x,
                        c.y + (c.h as f32 * 0.48) as i32,
                        c.w,
                        (c.h as f32 * 0.2) as i32,
                    ),
                    &lines,
                    fh * 0.8,
                    pal.readout,
                    2,
                );
            }
        }
        AutoPage::Fluid => {
            // Key temps only (ECT · OIL · TFT) — not a four-gauge wall.
            let n = if dclt >= 2 { 2 } else { 3 };
            let gw = (c.w - 12) / n;
            let gh = (c.h as f32 * 0.55) as i32;
            let temps: &[(&str, f32, f32, crate::Color)] = &[
                ("ECT", v.coolant, v.coolant_c, pal.primary),
                ("OIL", v.oil_temp, v.oil_temp_c, pal.caution),
                ("TFT", v.trans_temp, v.trans_temp_c, pal.nav),
            ];
            for (i, (lab, norm, deg, col)) in temps.iter().take(n as usize).enumerate() {
                let gx = c.x + 2 + i as i32 * (gw + 2);
                round_gauge(
                    page.surface,
                    Rect::new(gx, c.y, gw, gh),
                    RoundGaugeOpts {
                        value: (*norm).clamp(0.0, 1.0),
                        redline: Some(0.85),
                        label: lab,
                        color: *col,
                        font_px: fh * 0.6,
                        ..Default::default()
                    },
                );
                label(
                    page.surface,
                    gx as f32 + 4.0,
                    (c.y + gh - (fh * 0.95) as i32) as f32,
                    &format!("{:.0}C", deg),
                    pal.readout,
                    fh * 0.65,
                );
            }
            if dclt == 0 {
                let lines = vec![
                    format!("IAT   {:.0} C", v.iat_c),
                    format!("AAT   {:.0} C", v.temp_out_c),
                ];
                numeric_matrix(
                    page.surface,
                    Rect::new(c.x, c.y + gh + 4, c.w, (c.h - gh - 6).max(20)),
                    &lines,
                    fh * 0.8,
                    pal.readout,
                    2,
                );
            }
        }
        AutoPage::Elec => {
            let batt_w = (c.w as f32 * 0.48) as i32;
            // Battery as round gauge (11–15 V mapped ~0..1 via v.battery).
            round_gauge(
                page.surface,
                Rect::new(c.x, c.y, batt_w, (c.h as f32 * 0.7) as i32),
                RoundGaugeOpts {
                    value: v.battery.clamp(0.0, 1.0),
                    redline: Some(0.92),
                    label: "BATT",
                    color: pal.nav,
                    font_px: fh * 0.75,
                    ..Default::default()
                },
            );
            value_readout(
                page.surface,
                c.x as f32 + batt_w as f32 * 0.5,
                c.y as f32 + c.h as f32 * 0.78,
                "V",
                &format!("{:.1}", v.battery_v),
                "V",
                pal.nav,
                fh * 0.65,
                fh * 1.4,
            );
            value_readout(
                page.surface,
                c.x as f32 + batt_w as f32 + (c.w - batt_w) as f32 * 0.5,
                c.y as f32 + c.h as f32 * 0.22,
                "LOAD",
                &format!("{:.0}", v.load * 100.0),
                "%",
                pal.caution,
                fh * 0.7,
                fh * 1.6,
            );
            tape(
                page.surface,
                Rect::new(
                    c.x + batt_w + 6,
                    c.y + (c.h as f32 * 0.4) as i32,
                    (c.w - batt_w - 12).max(24),
                    (c.h as f32 * 0.35) as i32,
                ),
                "LOAD",
                v.load,
                pal.caution,
                fh * 0.65,
                false,
            );
            progress_strip(
                page.surface,
                Rect::new(c.x + 12, c.bottom() - 18, c.w - 24, 12),
                v.load,
                pal.caution,
                pal.structure,
            );
        }
        AutoPage::Drive => {
            // Hard: large **speedo** + proper **tach** gauges (not a thin tape).
            let half = (c.w - 12) / 2;
            let g_h = if dclt >= 2 {
                c.h - 8
            } else {
                (c.h as f32 * 0.62) as i32
            };
            let spd_n = (v.speed_mph / 120.0).clamp(0.0, 1.0);
            let rpm_n = (v.rpm / v.rpm_redline).clamp(0.0, 1.0);
            round_gauge(
                page.surface,
                Rect::new(c.x, c.y, half, g_h),
                RoundGaugeOpts {
                    value: spd_n,
                    redline: Some(0.85),
                    label: "SPD",
                    color: pal.readout,
                    font_px: fh * 0.75,
                    ..Default::default()
                },
            );
            value_readout(
                page.surface,
                c.x as f32 + half as f32 * 0.5,
                c.y as f32 + g_h as f32 - fh * 1.2,
                v.speed_unit.name(),
                &format!("{:.0}", v.speed_unit.from_mph(v.speed_mph)),
                "",
                pal.readout,
                fh * 0.55,
                fh * 1.2,
            );
            round_gauge(
                page.surface,
                Rect::new(c.x + half + 8, c.y, half, g_h),
                RoundGaugeOpts {
                    value: rpm_n,
                    redline: Some(0.9),
                    label: "RPM",
                    color: pal.primary,
                    font_px: fh * 0.75,
                    ..Default::default()
                },
            );
            value_readout(
                page.surface,
                c.x as f32 + half as f32 + 8.0 + half as f32 * 0.5,
                c.y as f32 + g_h as f32 - fh * 1.2,
                "rpm",
                &format!("{:.0}", v.rpm),
                "",
                pal.primary,
                fh * 0.55,
                fh * 1.2,
            );
            if dclt < 2 {
                let park_items = [StatusItem {
                    label: "PARK",
                    on: v.park_brake,
                }];
                status_grid_flash(
                    page.surface,
                    Rect::new(c.x + 4, c.y + g_h + 2, c.w / 3, 28),
                    &park_items,
                    1,
                    fh * 0.85,
                    pal.warning,
                    pal.dim,
                    Some(&["PARK"]),
                    v.park_brake && flash,
                );
                value_readout(
                    page.surface,
                    c.x as f32 + c.w as f32 * 0.72,
                    c.y as f32 + g_h as f32 + fh * 1.2,
                    "GEAR",
                    v.gear.label(),
                    "",
                    pal.nav,
                    fh * 0.65,
                    fh * 1.4,
                );
            }
            if dclt == 0 {
                let lines = vec![
                    format!("4WD   {}", v.drive.label()),
                    format!("TPS   {:.0} %", v.throttle * 100.0),
                ];
                numeric_matrix(
                    page.surface,
                    Rect::new(c.x, c.bottom() - (fh * 2.5) as i32, c.w, (fh * 2.4) as i32),
                    &lines,
                    fh * 0.75,
                    pal.readout,
                    2,
                );
            }
        }
        AutoPage::Chas => {
            tire_grid(
                page.surface,
                Rect::new(c.x, c.y, c.w, (c.h as f32 * 0.55) as i32),
                v.tire_fl,
                v.tire_fr,
                v.tire_rl,
                v.tire_rr,
                fh * 0.75,
                pal.primary,
                pal.warning,
                pal.structure,
            );
            let lines = channels::channels_in_group(v, "CHAS")
                .into_iter()
                .map(|ch| ch.line())
                .collect::<Vec<_>>();
            numeric_matrix(
                page.surface,
                Rect::new(
                    c.x,
                    c.y + (c.h as f32 * 0.55) as i32,
                    c.w,
                    (c.h as f32 * 0.45) as i32,
                ),
                &lines,
                fh * 0.7,
                pal.readout,
                2,
            );
        }
        AutoPage::Body => {
            let mut items = vec![
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
            if feat.map(|f| f.heated_seats).unwrap_or(false) {
                items.push(StatusItem {
                    label: "HSEAT",
                    on: true,
                });
            }
            if feat.map(|f| f.heated_steering).unwrap_or(false) {
                items.push(StatusItem {
                    label: "HSTR",
                    on: true,
                });
            }
            status_grid(
                page.surface,
                Rect::new(c.x, c.y, c.w, (c.h as f32 * 0.55) as i32),
                &items,
                3,
                fh * 0.75,
                pal.primary,
                pal.warning,
            );
            let lines = channels::channels_in_group(v, "BODY")
                .into_iter()
                .map(|ch| ch.line())
                .collect::<Vec<_>>();
            numeric_matrix(
                page.surface,
                Rect::new(
                    c.x,
                    c.y + (c.h as f32 * 0.58) as i32,
                    c.w,
                    (c.h as f32 * 0.4) as i32,
                ),
                &lines,
                fh * 0.75,
                pal.readout,
                2,
            );
        }
        AutoPage::Lights => {
            let mut items = vec![
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
            ];
            if feat.map(|f| f.fog_lights).unwrap_or(true) {
                items.push(StatusItem {
                    label: "FOG",
                    on: v.light_fog,
                });
            }
            items.push(StatusItem {
                label: "BRAKE",
                on: v.light_brake,
            });
            items.push(StatusItem {
                label: "TURN L",
                on: v.light_turn_l,
            });
            items.push(StatusItem {
                label: "TURN R",
                on: v.light_turn_r,
            });
            items.push(StatusItem {
                label: "INT",
                on: v.light_interior,
            });
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
        AutoPage::Clim => {
            let lines = channels::channels_in_group(v, "CLIM")
                .into_iter()
                .chain(std::iter::once(channels::Channel {
                    group: "CLIM",
                    label: "AC",
                    value: if v.hvac_ac { "ON".into() } else { "OFF".into() },
                    unit: "",
                }))
                .map(|ch| ch.line())
                .collect::<Vec<_>>();
            numeric_matrix(page.surface, c.inset(4), &lines, fh * 1.0, pal.readout, 1);
        }
        AutoPage::Cam => {
            // Live camera only; empty = NO VIDEO (never invents imagery).
            if let Some(fr) = cam_frame {
                blit_grey_flir(page.surface, c.inset(4), fr, pal.primary, pal.structure);
                crosshair(page.surface, c.center().0, c.center().1, 18, 4, pal.caution);
                track_gate(page.surface, c.center().0, c.center().1, 28, pal.readout);
                label(
                    page.surface,
                    c.x as f32 + 4.0,
                    c.bottom() as f32 - fh,
                    "CAM LIVE",
                    pal.dim,
                    fh * 0.7,
                );
            } else {
                value_readout(
                    page.surface,
                    c.center().0 as f32,
                    c.y as f32 + c.h as f32 * 0.45,
                    "CAM",
                    "NO VIDEO",
                    "",
                    pal.dim,
                    fh * 0.8,
                    fh * 1.6,
                );
            }
        }
        AutoPage::Range => {
            // No synthetic motion — empty range until real sensor GO.
            let rng = RangeSnapshot::from_env_or_empty();
            if rng.has_any() {
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
            } else {
                value_readout(
                    page.surface,
                    c.center().0 as f32,
                    c.y as f32 + c.h as f32 * 0.45,
                    "RNG",
                    "NO SENSOR",
                    "",
                    pal.dim,
                    fh * 0.8,
                    fh * 1.6,
                );
            }
        }
        AutoPage::Attitude => {
            // Dedicated ATT: horizon ball + compass/heading (hard requirement).
            // DCLT 2 → ball fills glass; lower DCLT keeps heading block.
            let ball_w = if dclt >= 2 {
                c.w - 4
            } else {
                (c.w as f32 * 0.62) as i32
            };
            // Sky blue / ground brown (cyan reserved for ownship-class marks).
            attitude_ball(
                page.surface,
                Rect::new(c.x, c.y, ball_w, c.h - 8),
                v.pitch_deg,
                v.roll_deg,
                v.heading_deg,
                rgb(35, 90, 175),
                rgb(110, 75, 35),
                pal.readout,
                pal.dim,
            );
            if dclt < 2 {
                let hx = c.x + ball_w + 4;
                let hw = (c.w - ball_w - 8).max(40);
                heading_display(
                    page.surface,
                    Rect::new(hx, c.y + 8, hw, c.h / 3),
                    v.heading_deg,
                    pal.readout,
                    pal.dim,
                    fh,
                );
                if dclt == 0 {
                    let lines = vec![
                        format!("PITCH {:.1}", v.pitch_deg),
                        format!("ROLL  {:.1}", v.roll_deg),
                        format!("HDG   {:05.1}", ((v.heading_deg % 360.0) + 360.0) % 360.0),
                    ];
                    numeric_matrix(
                        page.surface,
                        Rect::new(hx, c.y + c.h / 2, hw, c.h / 2 - 4),
                        &lines,
                        fh * 0.8,
                        pal.readout,
                        1,
                    );
                }
            }
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
                    "HDG {:05.1}  {:.0} {}",
                    ((v.heading_deg % 360.0) + 360.0) % 360.0,
                    v.speed_unit.from_mph(v.speed_mph),
                    v.speed_unit.name()
                ),
                pal.readout,
                fh * 0.75,
            );
        }
        AutoPage::Faults => {
            // Dedicated DTC format (hard requirement). Empty list is honest NONE.
            label(
                page.surface,
                c.x as f32 + 4.0,
                c.y as f32 + 2.0,
                &format!("COUNT  {}   ·  READ ONLY · NO CLEAR", v.dtc_count),
                if v.dtc_count > 0 {
                    pal.warning
                } else {
                    pal.primary
                },
                fh * 0.7,
            );
            if v.dtcs.is_empty() {
                value_readout(
                    page.surface,
                    c.center().0 as f32,
                    c.y as f32 + c.h as f32 * 0.45,
                    "DTC",
                    "NONE",
                    "",
                    pal.primary,
                    fh,
                    fh * 2.0,
                );
            } else {
                let lines: Vec<String> = v
                    .dtcs
                    .iter()
                    .map(|d| format!("{}  {}", d.code, d.kind.label()))
                    .collect();
                numeric_matrix(
                    page.surface,
                    Rect::new(c.x, c.y + 20, c.w, c.h - 28),
                    &lines,
                    fh * 0.95,
                    pal.warning,
                    1,
                );
            }
        }
        AutoPage::Bus => {
            // Link header + full channel dump.
            let mut lines: Vec<String> = v
                .bus_link_lines()
                .into_iter()
                .map(|l| format!("LINK {l}"))
                .collect();
            lines.push("── CHANNELS ──".into());
            lines.extend(
                channels::all_channels(v)
                    .into_iter()
                    .map(|ch| format!("{} {}", ch.group, ch.line())),
            );
            let cols = if lines.len() > 28 { 3 } else { 2 };
            let fsz = if lines.len() > 40 {
                fh * 0.52
            } else {
                fh * 0.62
            };
            numeric_matrix(page.surface, c.inset(2), &lines, fsz, pal.readout, cols);
        }
        AutoPage::Own => {
            let id = vehicle_profile::identity_line();
            // Link state as hero status (LIVE / ERR / SEARCH / RECONN / OFF).
            let link_col = match v.bus_state.as_str() {
                "LIVE" => pal.primary,
                "ERR" => pal.warning,
                "BIT" | "SEARCH" | "CONN" | "RECONN" => pal.caution,
                _ => pal.dim,
            };
            value_readout(
                page.surface,
                c.x as f32 + c.w as f32 * 0.28,
                c.y as f32 + fh * 1.6,
                "LINK",
                &v.bus_state,
                "",
                link_col,
                fh * 0.65,
                fh * 1.5,
            );
            value_readout(
                page.surface,
                c.x as f32 + c.w as f32 * 0.72,
                c.y as f32 + fh * 1.6,
                "KIND",
                &v.bus_kind,
                "",
                pal.nav,
                fh * 0.65,
                fh * 1.3,
            );
            let mut lines = vec![
                id,
                format!("VIN  {}", if v.vin.is_empty() { "—" } else { &v.vin }),
            ];
            lines.extend(v.bus_link_lines());
            lines.push("STACK J1979+UDS+FORD".into());
            numeric_matrix(
                page.surface,
                Rect::new(
                    c.x,
                    c.y + (fh * 3.4) as i32,
                    c.w,
                    (c.h - (fh * 3.4) as i32).max(40),
                ),
                &lines,
                fh * 0.65,
                pal.primary,
                1,
            );
        }
        AutoPage::Setup => {
            let vin_s = if v.vin.is_empty() {
                "VIN  (none)".to_string()
            } else {
                format!("VIN  {}", v.vin)
            };
            let mut lines = Vec::new();
            lines.push(vin_s);
            lines.push(format!("SPD {}", v.speed_unit.name()));
            lines.extend(v.bus_link_lines());
            lines.push("── FEATURES (ref) ──".into());
            for lab in vehicle_profile::asbuilt_feature_labels().iter().take(10) {
                let s = if lab.len() > 28 {
                    format!("{}…", &lab[..27])
                } else {
                    lab.clone()
                };
                lines.push(s);
            }
            numeric_matrix(page.surface, c.inset(2), &lines, fh * 0.6, pal.readout, 1);
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
    draw_auto_with_video(page, which, pal, bezel, &v, t, None, None, None, None);
}

pub fn rpm_norm(rpm: f32, redline: f32) -> f32 {
    if redline <= 0.0 {
        0.0
    } else {
        (rpm / redline).clamp(0.0, 1.0)
    }
}
