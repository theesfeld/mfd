//! F-16-class **format (page) calls**.
//!
//! Names follow public flight-manual / open training usage.
//! See `docs/reference/f16-mfd-catalog.md`.

use crate::bezel::BezelState;
use crate::geom::Rect;
use crate::page::Page;
use crate::palette::Palette;
use crate::widget::{
    bearing_pointer, bscope_grid, caution_box, crosshair, horizon_cue, list_menu, numeric_readout,
    osb_chrome, progress_strip, range_rings, round_gauge, station_grid, tape_gauge, track_gate,
    video_frame, RoundGaugeOpts, TapeOpts, TapeOrientation,
};

/// Logical format id for OSB routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Blank,
    Sms,
    Hsd,
    Tgp,
    Fcr,
    FcrGm,
    FcrSea,
    Wpn,
    Had,
    Flir,
    Dte,
    Test,
    Eng,
    Fuel,
    Cni,
    Reset,
    Ecm,
    Tfr,
    HudRpt,
    Ufc,
    Pfl,
    Stores,
}

impl Format {
    pub const ALL: &'static [Format] = &[
        Format::Blank,
        Format::Sms,
        Format::Hsd,
        Format::Tgp,
        Format::Fcr,
        Format::FcrGm,
        Format::FcrSea,
        Format::Wpn,
        Format::Had,
        Format::Flir,
        Format::Dte,
        Format::Test,
        Format::Eng,
        Format::Fuel,
        Format::Cni,
        Format::Reset,
        Format::Ecm,
        Format::Tfr,
        Format::HudRpt,
        Format::Ufc,
        Format::Pfl,
        Format::Stores,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Format::Blank => "BLANK",
            Format::Sms => "SMS",
            Format::Hsd => "HSD",
            Format::Tgp => "TGP",
            Format::Fcr => "FCR",
            Format::FcrGm => "FCR GM",
            Format::FcrSea => "FCR SEA",
            Format::Wpn => "WPN",
            Format::Had => "HAD",
            Format::Flir => "FLIR",
            Format::Dte => "DTE",
            Format::Test => "TEST",
            Format::Eng => "ENG",
            Format::Fuel => "FUEL",
            Format::Cni => "CNI",
            Format::Reset => "RESET",
            Format::Ecm => "ECM",
            Format::Tfr => "TFR",
            Format::HudRpt => "HUD",
            Format::Ufc => "UFC",
            Format::Pfl => "PFL",
            Format::Stores => "STORES",
        }
    }

    /// Top OSB 1–5 cycle primary formats (demo binding).
    pub fn from_top_osb(osb: u8, bank: usize) -> Option<Format> {
        let primary = [
            Format::Sms,
            Format::Hsd,
            Format::Tgp,
            Format::Fcr,
            Format::Wpn,
        ];
        let secondary = [
            Format::Had,
            Format::Flir,
            Format::Dte,
            Format::Eng,
            Format::Fuel,
        ];
        let tertiary = [
            Format::Cni,
            Format::Test,
            Format::Ecm,
            Format::HudRpt,
            Format::Blank,
        ];
        let set = match bank % 3 {
            0 => &primary,
            1 => &secondary,
            _ => &tertiary,
        };
        if (1..=5).contains(&osb) {
            Some(set[(osb - 1) as usize])
        } else {
            None
        }
    }
}

fn chrome(page: &mut Page, pal: &Palette, title: &str, bezel: &BezelState) {
    let b = page.bounds.inset(2);
    let top = ["SMS", "HSD", "TGP", "FCR", "WPN"];
    let right = ["DCLT", "SWAP", "CNTL", "MODE", "GAIN"];
    let bottom = ["DTE", "TEST", "ENG", "FUEL", "CNI"];
    let left = ["HAD", "FLIR", "ECM", "HUD", "BLANK"];
    osb_chrome(
        page.surface,
        b,
        &top,
        &right,
        &bottom,
        &left,
        page.font_px * 0.75,
        pal.dim,
        bezel.last_osb,
    );
    page.label_centered(
        b.center().0 as f32,
        b.y as f32 + page.font_px * 1.6,
        title,
        pal.primary,
    );
    // Corner knob readouts (BRT/CON) — POC for real knobs later.
    page.label_at(
        b.x as f32 + 4.0,
        b.bottom() as f32 - page.font_px * 2.2,
        &format!("BRT {:.0}", bezel.brightness * 100.0),
        pal.dim,
        page.font_px * 0.7,
    );
    page.label_at(
        b.right() as f32 - page.font_px * 6.0,
        b.bottom() as f32 - page.font_px * 2.2,
        &format!("CON {:.0}", bezel.contrast * 100.0),
        pal.dim,
        page.font_px * 0.7,
    );
}

fn content(page: &Page) -> Rect {
    let b = page.bounds.inset(4);
    let m = (page.font_px * 1.8) as i32 + 8;
    Rect::new(b.x + m, b.y + m, b.w - 2 * m, b.h - 2 * m - 10)
}

pub fn draw_format(page: &mut Page, fmt: Format, pal: &Palette, bezel: &BezelState, t: f32) {
    page.clear();
    page.surface.clear(pal.glass);
    page.bezel();
    chrome(page, pal, fmt.name(), bezel);
    let c = content(page);
    match fmt {
        Format::Blank => {}
        Format::Sms | Format::Stores => {
            let labs = [
                "1 AIM", "2 AIM", "3 TANK", "4 GBU", "5 GUN", "6 GBU", "7 TANK", "8 AIM", "9 AIM",
            ];
            station_grid(
                page.surface,
                c,
                &labs,
                3,
                ((t * 0.5) as usize) % 9,
                page.font_px * 0.8,
                pal.primary,
                pal.readout,
            );
            let arm = if t.sin() > 0.0 { "MASTER ARM" } else { "SAFE" };
            numeric_readout(
                page.surface,
                c.x as f32 + 40.0,
                c.bottom() as f32 - 8.0,
                arm,
                if t.sin() > 0.0 { pal.warning } else { pal.dim },
                page.font_px * 0.85,
            );
        }
        Format::Hsd => {
            let (cx, cy) = c.center();
            let r = (c.w.min(c.h) / 2 - 8).max(20);
            range_rings(page.surface, cx, cy, r, 3, pal.nav);
            bearing_pointer(
                page.surface,
                cx,
                cy,
                r as f32 * 0.9,
                (t * 25.0) % 360.0,
                pal.readout,
            );
            page.surface.circle(cx, cy, 3, pal.primary);
            numeric_readout(
                page.surface,
                c.x as f32 + 40.0,
                c.y as f32 + 12.0,
                &format!("HDG {:.0}", (t * 25.0) % 360.0),
                pal.primary,
                page.font_px,
            );
            numeric_readout(
                page.surface,
                c.x as f32 + 40.0,
                c.y as f32 + 12.0 + page.font_px + 4.0,
                "RNG 40 NM",
                pal.dim,
                page.font_px * 0.9,
            );
        }
        Format::Tgp | Format::Flir => {
            video_frame(page.surface, c.inset(c.w / 10), pal.structure);
            let tx = c.center().0 + ((t * 0.8).sin() * c.w as f32 * 0.15) as i32;
            let ty = c.center().1 + ((t * 0.6).cos() * c.h as f32 * 0.12) as i32;
            track_gate(page.surface, tx, ty, 14, pal.readout);
            crosshair(page.surface, c.center().0, c.center().1, 24, 6, pal.dim);
            let lz = if t.sin() > 0.7 {
                "LASER ARM"
            } else {
                "LASER SAFE"
            };
            numeric_readout(
                page.surface,
                c.x as f32 + 50.0,
                c.bottom() as f32 - 6.0,
                lz,
                if t.sin() > 0.7 { pal.warning } else { pal.dim },
                page.font_px * 0.85,
            );
        }
        Format::Fcr | Format::FcrGm | Format::FcrSea => {
            bscope_grid(page.surface, c, 6, pal.structure);
            let px = c.x as f32 + (0.5 + 0.35 * (t * 0.5).sin()) * c.w as f32;
            let py = c.bottom() as f32 - (0.3 + 0.4 * (t * 0.35).cos().abs()) * c.h as f32;
            page.surface.circle(px as i32, py as i32, 5, pal.caution);
            numeric_readout(
                page.surface,
                c.x as f32 + 30.0,
                c.y as f32 + 10.0,
                if matches!(fmt, Format::FcrGm) {
                    "GM"
                } else if matches!(fmt, Format::FcrSea) {
                    "SEA"
                } else {
                    "RWS"
                },
                pal.primary,
                page.font_px,
            );
            numeric_readout(
                page.surface,
                c.right() as f32 - 40.0,
                c.y as f32 + 10.0,
                &format!("G {:.0}", bezel.gain * 100.0),
                pal.dim,
                page.font_px * 0.85,
            );
        }
        Format::Wpn | Format::Had => {
            list_menu(
                page.surface,
                c,
                &[
                    "MODE  CCRP",
                    "PROFILE  1",
                    "TARGET  TGP",
                    "RELEASE  SGL",
                    "FUZE  N/S",
                ],
                Some(((t * 0.4) as usize) % 5),
                page.font_px,
                pal.primary,
                pal.readout,
            );
        }
        Format::Dte | Format::Cni | Format::Ufc | Format::Pfl => {
            let lines: &[&str] = match fmt {
                Format::Dte => &[
                    "LOAD 1 READY",
                    "LOAD 2 READY",
                    "WP LIST 12",
                    "DTC MOUNTED",
                    "COMM OK",
                ],
                Format::Cni => &[
                    "UHF  251.000",
                    "VHF  127.500",
                    "IFF  MODE 3",
                    "TACAN  22Y",
                    "ILS  109.50",
                ],
                Format::Ufc => &[
                    "DED  STPT 12",
                    "LAT  N 36 12.1",
                    "LNG  W 115 08.4",
                    "TOS  12:04:11",
                    "",
                ],
                Format::Pfl => &[
                    "PFL 00  NO FAULTS",
                    "MFDS  OK",
                    "FCR  OK",
                    "SMS  OK",
                    "INS  RDY",
                ],
                _ => &[],
            };
            list_menu(
                page.surface,
                c,
                lines,
                None,
                page.font_px,
                pal.primary,
                pal.readout,
            );
        }
        Format::Test | Format::Reset => {
            caution_box(
                page.surface,
                c.inset(c.w / 6),
                if matches!(fmt, Format::Test) {
                    "BIT GO"
                } else {
                    "RESET RDY"
                },
                page.font_px * 1.2,
                pal.primary,
            );
            progress_strip(
                page.surface,
                Rect::new(c.x + 20, c.bottom() - 24, c.w - 40, 12),
                0.5 + 0.5 * (t * 0.7).sin(),
                pal.nav,
                pal.structure,
            );
        }
        Format::Eng => {
            let half = c.w / 2;
            round_gauge(
                page.surface,
                Rect::new(c.x, c.y, half, c.h / 2),
                RoundGaugeOpts {
                    value: 0.55 + 0.25 * (t * 0.7).sin(),
                    redline: Some(0.85),
                    label: "RPM",
                    color: pal.primary,
                    font_px: page.font_px * 0.8,
                    ..Default::default()
                },
            );
            round_gauge(
                page.surface,
                Rect::new(c.x + half, c.y, half, c.h / 2),
                RoundGaugeOpts {
                    value: 0.4 + 0.2 * (t * 0.5).cos(),
                    redline: None,
                    label: "NOZ",
                    color: pal.nav,
                    font_px: page.font_px * 0.8,
                    ..Default::default()
                },
            );
            let ty = c.y + c.h / 2 + 4;
            let tw = c.w / 2 - 8;
            tape_gauge(
                page.surface,
                Rect::new(c.x + 4, ty, tw, c.h / 2 - 8),
                TapeOpts {
                    orientation: TapeOrientation::Vertical,
                    font_px: page.font_px * 0.75,
                    color: pal.caution,
                    value: 0.45 + 0.1 * (t * 0.3).sin(),
                    label: "OIL",
                },
            );
            tape_gauge(
                page.surface,
                Rect::new(c.x + tw + 8, ty, tw, c.h / 2 - 8),
                TapeOpts {
                    orientation: TapeOrientation::Vertical,
                    font_px: page.font_px * 0.75,
                    color: pal.warning,
                    value: 0.5 + 0.12 * (t * 0.4).cos(),
                    label: "FTIT",
                },
            );
        }
        Format::Fuel => {
            let tw = c.w / 3 - 6;
            for (i, (lab, val, col)) in [
                ("TOT", 0.7 + 0.1 * (t * 0.1).cos(), pal.primary),
                ("INT", 0.55 + 0.08 * (t * 0.12).sin(), pal.nav),
                ("EXT", 0.3 + 0.05 * (t * 0.08).cos(), pal.caution),
            ]
            .iter()
            .enumerate()
            {
                tape_gauge(
                    page.surface,
                    Rect::new(c.x + i as i32 * (tw + 6), c.y, tw, c.h),
                    TapeOpts {
                        orientation: TapeOrientation::Vertical,
                        font_px: page.font_px * 0.8,
                        color: *col,
                        value: *val,
                        label: lab,
                    },
                );
            }
        }
        Format::Ecm => {
            list_menu(
                page.surface,
                c,
                &[
                    "ECM  STBY",
                    "RWR  NORM",
                    "CHAFF  30",
                    "FLARE  15",
                    "JAM  OFF",
                ],
                Some(0),
                page.font_px,
                pal.primary,
                pal.caution,
            );
        }
        Format::Tfr | Format::HudRpt => {
            horizon_cue(
                page.surface,
                c.center().0,
                c.center().1,
                c.w / 4,
                10.0 * (t * 0.4).sin(),
                pal.primary,
            );
            bearing_pointer(
                page.surface,
                c.center().0,
                c.center().1,
                c.h as f32 * 0.2,
                0.0,
                pal.nav,
            );
            numeric_readout(
                page.surface,
                c.center().0 as f32,
                c.y as f32 + 16.0,
                if matches!(fmt, Format::Tfr) {
                    "TFR  SOFT"
                } else {
                    "HUD RPT"
                },
                pal.readout,
                page.font_px,
            );
        }
    }
}

// Thin wrappers for direct calls (stable names).
pub fn blank(page: &mut Page, pal: &Palette, bezel: &BezelState, t: f32) {
    draw_format(page, Format::Blank, pal, bezel, t);
}
pub fn sms(page: &mut Page, pal: &Palette, bezel: &BezelState, t: f32) {
    draw_format(page, Format::Sms, pal, bezel, t);
}
pub fn hsd(page: &mut Page, pal: &Palette, bezel: &BezelState, t: f32) {
    draw_format(page, Format::Hsd, pal, bezel, t);
}
pub fn tgp(page: &mut Page, pal: &Palette, bezel: &BezelState, t: f32) {
    draw_format(page, Format::Tgp, pal, bezel, t);
}
pub fn fcr(page: &mut Page, pal: &Palette, bezel: &BezelState, t: f32) {
    draw_format(page, Format::Fcr, pal, bezel, t);
}
pub fn eng(page: &mut Page, pal: &Palette, bezel: &BezelState, t: f32) {
    draw_format(page, Format::Eng, pal, bezel, t);
}
pub fn fuel(page: &mut Page, pal: &Palette, bezel: &BezelState, t: f32) {
    draw_format(page, Format::Fuel, pal, bezel, t);
}
