//! F-16-class **MFD format** calls.
//!
//! Layout and OSB legends follow **public** training material (DCS F-16C Early
//! Access Guide, Chuck’s F-16C guide, Hoggit MFD notes). The HAF basic flight
//! manual `docs/HAF-F16.pdf` (T.O. GR1F16CJ-1) does **not** contain MFD page art;
//! it defers to T.O. GR1F16CJ-34-1-1 (avionics). See `docs/reference/`.

use crate::bezel::BezelState;
use crate::font::{draw_text, draw_text_centered, text_width};
use crate::geom::Rect;
use crate::page::Page;
use crate::palette::Palette;
use crate::widget::{
    content_after_osb, crosshair, label, list_menu, numeric_readout, osb_chrome, progress_strip,
    range_rings, round_gauge, station_grid, tape_gauge, track_gate, video_frame, RoundGaugeOpts,
    TapeOpts, TapeOrientation,
};
use crate::{Color, Surface};

/// Logical format id for OSB routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Blank,
    /// Master format-select menu (BLANK/HAD/FCR/TGP/…).
    Menu,
    /// All public widgets (integration test face).
    Gallery,
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
        Format::Menu,
        Format::Gallery,
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
            Format::Menu => "MENU",
            Format::Gallery => "WIDG",
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

    /// Top OSB 1–5 bank for demo format select.
    pub fn from_top_osb(osb: u8, bank: usize) -> Option<Format> {
        let primary = [
            Format::Fcr,
            Format::Hsd,
            Format::Sms,
            Format::Tgp,
            Format::Menu,
        ];
        let secondary = [
            Format::Had,
            Format::Wpn,
            Format::Dte,
            Format::Test,
            Format::Eng,
        ];
        let tertiary = [
            Format::Fuel,
            Format::Cni,
            Format::Ecm,
            Format::HudRpt,
            Format::Gallery,
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

// ─── OSB legends (public Master Menu / format pages) ─────────────────────────
// Array order matches `osb_chrome`:
//   top[0..4]    = OSB 1..5 L→R
//   right[0..4]  = OSB 6..10 top→bot
//   bottom[0..4] = OSB 15..11 L→R  (so bottom[1]=OSB14 primary, bottom[2]=OSB13, bottom[3]=OSB12)
//   left[0..4]   = OSB 20..16 top→bot

type Osb5 = [&'static str; 5];

/// Master menu bottom row must be OSB15..11 L→R = SWAP, OSB14 pri, OSB13 sec, OSB12 ter, DCLT.
fn osb_menu_fixed() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["BLANK", "HAD", "", "RCCE", "RSET"],
        ["SMS", "HSD", "DTE", "TEST", "FLCS"],
        // L→R: OSB15 SWAP, OSB14 (pri label), OSB13 (sec), OSB12 (ter), OSB11 DCLT
        ["SWAP", "FCR", "HSD", "SMS", "DCLT"],
        // top→bot OSB20..16: FCR TGP WPN TFR FLIR
        ["FCR", "TGP", "WPN", "TFR", "FLIR"],
    )
}

fn osb_fcr(mode: &'static str) -> (Osb5, Osb5, Osb5, Osb5) {
    (
        [mode, "CRM", "RWS", "NORM", "OVRD"],
        ["CNTL", "DCLT", "SWAP", mode, ""],
        ["SWAP", "FCR", "HSD", "SMS", "DCLT"],
        ["80", "40", "20", "10", "A6"],
    )
}

fn osb_hsd() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["CEN", "DCPL", "NORM", "MSG", "CNTL"],
        ["▲", "40", "▼", "FZ", ""],
        ["SWAP", "FCR", "HSD", "SMS", "DCLT"],
        ["CNTL", "CZ", "DEP", "CPL", ""],
    )
}

fn osb_sms() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["INV", "A-A", "A-G", "GUN", "S-J"],
        ["PROF", "FUSE", "REL", "CNTL", ""],
        ["SWAP", "FCR", "HSD", "SMS", "DCLT"],
        ["STEP", "WPN", "INV", "ARM", "SAFE"],
    )
}

fn osb_tgp() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["FLIR", "CCD", "WHOT", "BHOT", ""],
        ["CNTL", "LSS", "LT", "FOV", ""],
        ["SWAP", "FCR", "HSD", "TGP", "DCLT"],
        ["NARO", "WIDE", "GRAY", "MGC", "AGC"],
    )
}

fn osb_dte() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["DTE", "CLAS", "LOAD", "FCR", ""],
        ["MSN", "COMM", "INV", "PROF", ""],
        ["SWAP", "FCR", "HSD", "DTE", "DCLT"],
        ["ELINT", "SMDL", "TNDL", "NCTR", ""],
    )
}

fn osb_had() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["HAD", "THRT", "PRI", "CNTL", ""],
        ["TBL1", "TBL2", "TBL3", "RWR", ""],
        ["SWAP", "FCR", "HSD", "HAD", "DCLT"],
        ["ALL", "FRND", "UNK", "HOS", ""],
    )
}

fn osb_wpn() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["WPN", "CCRP", "CCIP", "DTOS", "MAN"],
        ["PROF", "FUZE", "RP", "INTV", ""],
        ["SWAP", "FCR", "HSD", "WPN", "DCLT"],
        ["SGL", "PAIR", "RIPPLE", "STEP", ""],
    )
}

fn osb_test() -> (Osb5, Osb5, Osb5, Osb5) {
    (
        ["TEST", "BIT", "MFL", "CLR", ""],
        ["MFDS", "FCR", "SMS", "INS", ""],
        ["SWAP", "FCR", "HSD", "TEST", "DCLT"],
        ["GO", "NOGO", "RUN", "STOP", ""],
    )
}

fn osb_named(title: &'static str) -> (Osb5, Osb5, Osb5, Osb5) {
    (
        [title, "", "", "", ""],
        ["CNTL", "DCLT", "SWAP", "", ""],
        ["SWAP", "FCR", "HSD", "SMS", "DCLT"],
        ["", "", "", "", ""],
    )
}

fn legends_for(fmt: Format) -> (Osb5, Osb5, Osb5, Osb5) {
    match fmt {
        Format::Menu | Format::Reset => osb_menu_fixed(),
        Format::Fcr => osb_fcr("RWS"),
        Format::FcrGm => osb_fcr("GM"),
        Format::FcrSea => osb_fcr("SEA"),
        Format::Hsd => osb_hsd(),
        Format::Sms | Format::Stores => osb_sms(),
        Format::Tgp | Format::Flir => osb_tgp(),
        Format::Dte => osb_dte(),
        Format::Had => osb_had(),
        Format::Wpn => osb_wpn(),
        Format::Test => osb_test(),
        Format::Blank => osb_named("BLANK"),
        Format::Gallery => osb_named("WIDG"),
        Format::Eng => osb_named("ENG"),
        Format::Fuel => osb_named("FUEL"),
        Format::Cni => osb_named("CNI"),
        Format::Ecm => osb_named("ECM"),
        Format::Tfr => osb_named("TFR"),
        Format::HudRpt => osb_named("HUD"),
        Format::Ufc => osb_named("UFC"),
        Format::Pfl => osb_named("PFL"),
    }
}

fn chrome(page: &mut Page, pal: &Palette, fmt: Format, bezel: &BezelState) {
    let b = page.bounds.inset(2);
    let (top, right, bottom, left) = legends_for(fmt);
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
    // Title strip top-center under OSB1–5
    let c = content_after_osb(b, page.font_px * 0.65);
    page.label_centered(
        c.center().0 as f32,
        c.y as f32 + page.font_px * 0.45,
        fmt.name(),
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
        (c.h - (page.font_px as i32) - 6).max(40),
    )
}

// ─── Drawing helpers (F-16-class symbology) ──────────────────────────────────

fn ownship(s: &mut Surface, cx: i32, cy: i32, color: Color) {
    // Aircraft reference: chevron pointing up (nose).
    let pts = [
        (cx, cy - 8),
        (cx - 6, cy + 6),
        (cx, cy + 2),
        (cx + 6, cy + 6),
        (cx, cy - 8),
    ];
    s.polyline(&pts, color);
}

fn tick_cardinal(s: &mut Surface, cx: i32, cy: i32, r: i32, color: Color) {
    // N arrow out, S long tick, E/W short — HSD style on innermost ring.
    s.line_aa(cx, cy - r, cx, cy - r - 8, color); // N
    s.line_aa(cx - 3, cy - r - 5, cx, cy - r - 8, color);
    s.line_aa(cx + 3, cy - r - 5, cx, cy - r - 8, color);
    s.line_aa(cx, cy + r - 4, cx, cy + r + 4, color); // S
    s.line_aa(cx + r - 3, cy, cx + r, cy, color); // E
    s.line_aa(cx - r, cy, cx - r + 3, cy, color); // W
}

fn box_label(s: &mut Surface, x: f32, y: f32, text: &str, color: Color, px: f32) {
    draw_text(s, x, y, text, color, px);
}

/// FCR RWS B-scope: range vertical (near bottom), azimuth horizontal.
fn draw_fcr_bscope(s: &mut Surface, c: Rect, pal: &Palette, t: f32, mode: &str) {
    let x0 = c.x + 8;
    let y0 = c.y + 4;
    let x1 = c.right() - 8;
    let y1 = c.bottom() - 20;
    // Outer frame
    s.line_aa(x0, y0, x1, y0, pal.structure);
    s.line_aa(x1, y0, x1, y1, pal.structure);
    s.line_aa(x1, y1, x0, y1, pal.structure);
    s.line_aa(x0, y1, x0, y0, pal.structure);

    let w = (x1 - x0).max(1) as f32;
    let h = (y1 - y0).max(1) as f32;

    // Range ticks (left) + labels 0 / mid / max NM
    let max_nm = 40.0_f32;
    for i in 0..=4 {
        let fr = i as f32 / 4.0;
        let yy = y1 - (h * fr) as i32;
        s.line_aa(x0, yy, x0 + 6, yy, pal.dim);
        if i % 2 == 0 {
            let nm = (max_nm * fr) as i32;
            draw_text(
                s,
                x0 as f32 + 8.0,
                yy as f32 - 4.0,
                &format!("{nm}"),
                pal.dim,
                10.0,
            );
        }
    }
    // Azimuth ticks bottom −60..+60
    for i in 0..=6 {
        let fa = i as f32 / 6.0;
        let xx = x0 + (w * fa) as i32;
        s.line_aa(xx, y1, xx, y1 - 5, pal.dim);
    }
    draw_text(s, x0 as f32, y1 as f32 + 4.0, "L60", pal.dim, 9.0);
    draw_text_centered(
        s,
        (x0 + x1) as f32 * 0.5,
        y1 as f32 + 8.0,
        "0",
        pal.dim,
        9.0,
    );
    draw_text(s, x1 as f32 - 22.0, y1 as f32 + 4.0, "R60", pal.dim, 9.0);

    // Azimuth scan gates (±30 example)
    let a30 = w * 0.25;
    let mid = (x0 + x1) as f32 * 0.5;
    s.line_aa(
        (mid - a30) as i32,
        y0,
        (mid - a30) as i32,
        y1,
        pal.structure,
    );
    s.line_aa(
        (mid + a30) as i32,
        y0,
        (mid + a30) as i32,
        y1,
        pal.structure,
    );

    // Horizon / scan bar (moving)
    let bar_y = y1 - (h * (0.55 + 0.35 * (t * 0.4).sin().abs())) as i32;
    s.line_aa(x0 + 2, bar_y, x1 - 2, bar_y, pal.nav);

    // Contacts (synthetic)
    let contacts = [
        (0.35 + 0.05 * t.sin(), 0.62, pal.caution),
        (0.55, 0.40 + 0.08 * (t * 0.7).cos(), pal.warning),
        (0.70, 0.75, pal.primary),
    ];
    for (az, rng, col) in contacts {
        let px = x0 as f32 + az * w;
        let py = y1 as f32 - rng * h;
        // Brick target symbol
        s.rect_fill(
            px as i32 - 3,
            py as i32 - 2,
            px as i32 + 3,
            py as i32 + 2,
            col,
        );
    }

    // Acquisition cursor (diamond)
    let acx = mid + (0.15 * w) * (t * 0.35).sin();
    let acy = y1 as f32 - h * (0.45 + 0.1 * (t * 0.25).cos());
    let acxi = acx as i32;
    let acyi = acy as i32;
    s.line_aa(acxi, acyi - 8, acxi + 6, acyi, pal.readout);
    s.line_aa(acxi + 6, acyi, acxi, acyi + 8, pal.readout);
    s.line_aa(acxi, acyi + 8, acxi - 6, acyi, pal.readout);
    s.line_aa(acxi - 6, acyi, acxi, acyi - 8, pal.readout);

    // Mode / status strip
    box_label(s, x0 as f32 + 4.0, y0 as f32 + 2.0, mode, pal.primary, 11.0);
    box_label(s, mid - 20.0, y0 as f32 + 2.0, "CRM", pal.readout, 11.0);
    box_label(
        s,
        x1 as f32 - 50.0,
        y0 as f32 + 2.0,
        "A6  4B",
        pal.dim,
        10.0,
    );
    box_label(s, x0 as f32 + 4.0, y1 as f32 - 14.0, "G80", pal.dim, 10.0);
}

fn draw_hsd_page(s: &mut Surface, c: Rect, pal: &Palette, t: f32) {
    // Depressed format: ownship lower third
    let cx = c.center().0;
    let cy = c.y + (c.h as f32 * 0.72) as i32;
    let r_out = (c.w.min(c.h) as f32 * 0.42) as i32;
    let r_mid = (r_out as f32 * 0.66) as i32;
    let r_in = (r_out as f32 * 0.33) as i32;

    for rr in [r_out, r_mid, r_in] {
        s.circle(cx, cy, rr, pal.nav);
    }
    tick_cardinal(s, cx, cy, r_in, pal.readout);
    ownship(s, cx, cy, pal.primary);

    // FCR search volume wedge (±30°, out to outer ring)
    let hdg = t * 8.0;
    for &a in &[-30.0_f32, 30.0] {
        let rad = (hdg + a).to_radians();
        let x = cx as f32 + rad.sin() * r_out as f32;
        let y = cy as f32 - rad.cos() * r_out as f32;
        s.line_aa(cx, cy, x as i32, y as i32, pal.structure);
    }

    // Steerpoints (circles) + route
    let stpts = [
        (-0.35_f32, -0.55),
        (0.10, -0.80),
        (0.45, -0.40),
        (0.20, -0.15),
    ];
    let mut prev: Option<(i32, i32)> = None;
    for (i, (nx, ny)) in stpts.iter().enumerate() {
        let px = cx as f32 + nx * r_out as f32;
        let py = cy as f32 + ny * r_out as f32;
        let pxi = px as i32;
        let pyi = py as i32;
        if let Some((ox, oy)) = prev {
            s.line_aa(ox, oy, pxi, pyi, pal.readout);
        }
        prev = Some((pxi, pyi));
        let solid = i == 1; // selected STPT
        if solid {
            s.circle(pxi, pyi, 5, pal.primary);
            s.circle(pxi, pyi, 3, pal.primary);
        } else {
            s.circle(pxi, pyi, 4, pal.dim);
        }
        draw_text(s, px + 6.0, py - 4.0, &format!("{}", i + 1), pal.dim, 9.0);
    }

    // Bullseye (cross + circle)
    let bx = cx as f32 - 0.55 * r_out as f32;
    let by = cy as f32 - 0.25 * r_out as f32;
    s.circle(bx as i32, by as i32, 6, pal.caution);
    s.line_aa(
        bx as i32 - 8,
        by as i32,
        bx as i32 + 8,
        by as i32,
        pal.caution,
    );
    s.line_aa(
        bx as i32,
        by as i32 - 8,
        bx as i32,
        by as i32 + 8,
        pal.caution,
    );
    draw_text(s, bx + 10.0, by - 4.0, "BULL", pal.caution, 9.0);

    // Threat ring (yellow WEZ)
    let tx = cx as f32 + 0.55 * r_out as f32;
    let ty = cy as f32 - 0.55 * r_out as f32;
    s.circle(
        tx as i32,
        ty as i32,
        (r_out as f32 * 0.18) as i32,
        pal.caution,
    );
    draw_text(s, tx - 8.0, ty - 4.0, "SA", pal.caution, 9.0);

    // Hostile track
    let hx = cx as f32 + 0.15 * r_out as f32;
    let hy = cy as f32 - 0.65 * r_out as f32;
    s.rect_fill(
        hx as i32 - 4,
        hy as i32 - 4,
        hx as i32 + 4,
        hy as i32 + 4,
        pal.warning,
    );

    // Data block
    box_label(
        s,
        c.x as f32 + 4.0,
        c.y as f32 + 2.0,
        "DEP  40NM  DCPL",
        pal.primary,
        11.0,
    );
    box_label(
        s,
        c.x as f32 + 4.0,
        c.y as f32 + 16.0,
        &format!("STPT 2  HDG {:03.0}", (hdg % 360.0 + 360.0) % 360.0),
        pal.readout,
        11.0,
    );
}

fn draw_sms_inv(s: &mut Surface, c: Rect, pal: &Palette, t: f32) {
    // F-16 station map (simplified plan view): tips 1/9, wings 2-4 / 6-8, center 5.
    // Layout on glass as station boxes.
    let arm = if (t * 0.3).sin() > 0.0 {
        ("MASTER ARM", pal.warning)
    } else {
        ("MASTER SAFE", pal.dim)
    };
    draw_text_centered(
        s,
        c.center().0 as f32,
        c.y as f32 + 10.0,
        arm.0,
        arm.1,
        12.0,
    );

    // Station rows:  [1] [2][3][4]  [5]  [6][7][8] [9]
    let stations: [(&str, &str); 9] = [
        ("1", "AIM9"),
        ("2", "AIM120"),
        ("3", "TANK"),
        ("4", "GBU31"),
        ("5", "GUN"),
        ("6", "GBU31"),
        ("7", "TANK"),
        ("8", "AIM120"),
        ("9", "AIM9"),
    ];
    let sel = ((t * 0.4) as usize) % 9;
    let box_w = (c.w - 20) / 9;
    let box_h = 48;
    let y = c.center().1 - 10;
    for (i, (num, wpn)) in stations.iter().enumerate() {
        let x = c.x + 10 + i as i32 * box_w;
        let col = if i == sel { pal.readout } else { pal.primary };
        s.line_aa(x, y, x + box_w - 4, y, col);
        s.line_aa(x + box_w - 4, y, x + box_w - 4, y + box_h, col);
        s.line_aa(x + box_w - 4, y + box_h, x, y + box_h, col);
        s.line_aa(x, y + box_h, x, y, col);
        draw_text_centered(
            s,
            x as f32 + (box_w - 4) as f32 * 0.5,
            y as f32 + 12.0,
            num,
            col,
            11.0,
        );
        draw_text_centered(
            s,
            x as f32 + (box_w - 4) as f32 * 0.5,
            y as f32 + 28.0,
            wpn,
            if i == sel { pal.caution } else { pal.dim },
            9.0,
        );
    }

    // Selected weapon profile block
    let py = y + box_h + 16;
    draw_text(
        s,
        c.x as f32 + 12.0,
        py as f32,
        "WPN  AIM-120C   QTY 2",
        pal.primary,
        12.0,
    );
    draw_text(
        s,
        c.x as f32 + 12.0,
        py as f32 + 16.0,
        "PROF 1   REL SGL   FUZE N/A",
        pal.dim,
        11.0,
    );
    draw_text(
        s,
        c.x as f32 + 12.0,
        py as f32 + 32.0,
        "INV  A-A",
        pal.readout,
        11.0,
    );

    // Gun rounds
    draw_text(
        s,
        c.right() as f32 - 90.0,
        py as f32,
        "GUN 510",
        pal.caution,
        12.0,
    );
}

fn draw_tgp_page(s: &mut Surface, c: Rect, pal: &Palette, t: f32) {
    let frame = c.inset(c.w / 12);
    video_frame(s, frame, pal.structure);
    // Fake IR noise field — sparse dots
    for i in 0..40 {
        let u = ((i as f32 * 17.3 + t * 3.0).sin() * 0.5 + 0.5) * frame.w as f32;
        let v = ((i as f32 * 9.1 + t * 2.1).cos() * 0.5 + 0.5) * frame.h as f32;
        s.plot(frame.x + u as i32, frame.y + v as i32, pal.structure);
    }
    let (cx, cy) = frame.center();
    crosshair(s, cx, cy, 28, 8, pal.dim);
    let tx = cx + ((t * 0.7).sin() * frame.w as f32 * 0.18) as i32;
    let ty = cy + ((t * 0.55).cos() * frame.h as f32 * 0.14) as i32;
    track_gate(s, tx, ty, 16, pal.readout);

    // FOV brackets
    let half = frame.w / 5;
    s.line_aa(cx - half, cy - half, cx - half + 10, cy - half, pal.primary);
    s.line_aa(cx - half, cy - half, cx - half, cy - half + 10, pal.primary);
    s.line_aa(cx + half, cy - half, cx + half - 10, cy - half, pal.primary);
    s.line_aa(cx + half, cy - half, cx + half, cy - half + 10, pal.primary);

    draw_text(
        s,
        frame.x as f32 + 6.0,
        frame.y as f32 + 6.0,
        "FLIR  WHOT  WIDE",
        pal.primary,
        11.0,
    );
    let laser = if (t * 0.5).sin() > 0.6 {
        ("L ARM", pal.warning)
    } else {
        ("L SAFE", pal.dim)
    };
    draw_text(
        s,
        frame.x as f32 + 6.0,
        frame.bottom() as f32 - 14.0,
        laser.0,
        laser.1,
        11.0,
    );
    draw_text(
        s,
        frame.right() as f32 - 70.0,
        frame.bottom() as f32 - 14.0,
        "TGP SOI",
        pal.readout,
        11.0,
    );
}

fn draw_menu_page(s: &mut Surface, c: Rect, pal: &Palette) {
    // Master menu is mostly OSB labels; center shows assignment help.
    draw_text_centered(
        s,
        c.center().0 as f32,
        c.y as f32 + 20.0,
        "FORMAT SELECT",
        pal.primary,
        14.0,
    );
    draw_text_centered(
        s,
        c.center().0 as f32,
        c.y as f32 + 40.0,
        "OSB12/13/14 = TER/SEC/PRI",
        pal.dim,
        11.0,
    );
    let lines = [
        "TOP   BLANK HAD  RCCE RSET",
        "RIGHT SMS  HSD  DTE  TEST FLCS",
        "LEFT  FCR  TGP  WPN  TFR  FLIR",
        "BOT   SWAP PRI  SEC  TER  DCLT",
        "",
        "Public ref: DCS F-16C EA Guide MFD ch.",
        "HAF GR1F16CJ-1 defers to 34-1-1",
    ];
    for (i, line) in lines.iter().enumerate() {
        draw_text(
            s,
            c.x as f32 + 16.0,
            c.y as f32 + 70.0 + i as f32 * 16.0,
            line,
            if i < 4 { pal.readout } else { pal.dim },
            11.0,
        );
    }
}

fn draw_gallery(page: &mut Page, pal: &Palette, c: Rect, t: f32) {
    // Compact grid remains for widget QA — not a jet format.
    let fh = (page.font_px * 0.65).max(9.0);
    label(
        page.surface,
        c.x as f32 + 2.0,
        c.y as f32,
        "WIDGET QA",
        pal.dim,
        fh,
    );
    let gap = 3;
    let body = Rect::new(c.x, c.y + 14, c.w, c.h - 14);
    let cw = (body.w - gap * 2) / 3;
    let ch = (body.h - gap * 2) / 3;
    let cell = |col: i32, row: i32| {
        Rect::new(body.x + col * (cw + gap), body.y + row * (ch + gap), cw, ch)
    };
    round_gauge(
        page.surface,
        cell(0, 0).inset(2),
        RoundGaugeOpts {
            value: 0.55 + 0.3 * (t * 0.8).sin(),
            redline: Some(0.85),
            label: "RPM",
            color: pal.primary,
            font_px: fh,
            ..Default::default()
        },
    );
    {
        let r = cell(1, 0);
        let (cx, cy) = r.center();
        range_rings(page.surface, cx, cy, r.w.min(r.h) / 2 - 4, 3, pal.nav);
        ownship(page.surface, cx, cy, pal.primary);
    }
    tape_gauge(
        page.surface,
        cell(2, 0).inset(2),
        TapeOpts {
            orientation: TapeOrientation::Vertical,
            font_px: fh,
            color: pal.caution,
            value: 0.5 + 0.3 * (t * 0.4).sin(),
            label: "FUEL",
        },
    );
    draw_fcr_bscope(page.surface, cell(0, 1).inset(2), pal, t, "RWS");
    draw_tgp_page(page.surface, cell(1, 1).inset(2), pal, t);
    list_menu(
        page.surface,
        cell(2, 1).inset(4),
        &["CCRP", "CCIP", "DTOS", "MAN"],
        Some(((t * 0.3) as usize) % 4),
        fh,
        pal.primary,
        pal.readout,
    );
    station_grid(
        page.surface,
        cell(0, 2).inset(4),
        &["1", "2", "3", "4", "5", "6"],
        3,
        ((t * 0.5) as usize) % 6,
        fh,
        pal.primary,
        pal.readout,
    );
    {
        let r = cell(1, 2);
        progress_strip(
            page.surface,
            Rect::new(r.x + 4, r.y + 8, r.w - 8, 10),
            0.5 + 0.5 * (t * 0.6).sin(),
            pal.nav,
            pal.structure,
        );
        numeric_readout(
            page.surface,
            r.center().0 as f32,
            r.center().1 as f32,
            "BIT GO",
            pal.primary,
            fh + 2.0,
        );
    }
    {
        let r = cell(2, 2);
        let (cx, cy) = r.center();
        // horizon
        let bank = 10.0 * (t * 0.4).sin();
        let rad = bank.to_radians();
        let half = (r.w / 3) as f32;
        let dx = rad.cos() * half;
        let dy = rad.sin() * half;
        page.surface.line_aa(
            (cx as f32 - dx) as i32,
            (cy as f32 - dy) as i32,
            (cx as f32 + dx) as i32,
            (cy as f32 + dy) as i32,
            pal.primary,
        );
        ownship(page.surface, cx, cy, pal.readout);
    }
}

// ─── Public draw ─────────────────────────────────────────────────────────────

pub fn draw_format(page: &mut Page, fmt: Format, pal: &Palette, bezel: &BezelState, t: f32) {
    page.clear();
    page.surface.clear(pal.glass);
    page.bezel();
    chrome(page, pal, fmt, bezel);
    let c = content(page);
    match fmt {
        Format::Blank => {
            numeric_readout(
                page.surface,
                c.center().0 as f32,
                c.center().1 as f32,
                "BLANK",
                pal.dim,
                page.font_px,
            );
        }
        Format::Menu | Format::Reset => draw_menu_page(page.surface, c, pal),
        Format::Gallery => draw_gallery(page, pal, c, t),
        Format::Fcr => draw_fcr_bscope(page.surface, c, pal, t, "RWS"),
        Format::FcrGm => draw_fcr_bscope(page.surface, c, pal, t, "GM"),
        Format::FcrSea => draw_fcr_bscope(page.surface, c, pal, t, "SEA"),
        Format::Hsd => draw_hsd_page(page.surface, c, pal, t),
        Format::Sms | Format::Stores => draw_sms_inv(page.surface, c, pal, t),
        Format::Tgp | Format::Flir => draw_tgp_page(page.surface, c, pal, t),
        Format::Wpn => {
            list_menu(
                page.surface,
                c,
                &[
                    "MODE   CCRP",
                    "PROF   1",
                    "TGT    TGP",
                    "REL    SGL",
                    "FUZE   N/S",
                    "RP     1",
                    "INTV   0.00",
                ],
                Some(((t * 0.35) as usize) % 5),
                page.font_px,
                pal.primary,
                pal.readout,
            );
        }
        Format::Had => {
            let (cx, cy) = c.center();
            let r = (c.w.min(c.h) / 2 - 12).max(20);
            range_rings(page.surface, cx, cy, r, 2, pal.structure);
            ownship(page.surface, cx, cy, pal.primary);
            // Threat symbols
            for (i, lab) in ["SA6", "SA8", "AAA"].iter().enumerate() {
                let a = (i as f32 * 2.1 + t * 0.1).sin();
                let b = (i as f32 * 1.7 + t * 0.08).cos();
                let px = cx as f32 + a * r as f32 * 0.55;
                let py = cy as f32 + b * r as f32 * 0.45;
                s_circle_text(page.surface, px as i32, py as i32, lab, pal.caution, 10.0);
            }
            draw_text(
                page.surface,
                c.x as f32 + 6.0,
                c.y as f32 + 4.0,
                "HAD  THRT PRI",
                pal.primary,
                11.0,
            );
        }
        Format::Dte => {
            list_menu(
                page.surface,
                c,
                &[
                    "DTE POWER   ON",
                    "LOAD ALL    RDY",
                    "MSN PLAN    RDY",
                    "COMM        RDY",
                    "INV         RDY",
                    "PROF        RDY",
                    "FCR SET     RDY",
                    "TNDL        RDY",
                ],
                Some(1),
                page.font_px,
                pal.primary,
                pal.readout,
            );
        }
        Format::Test => {
            list_menu(
                page.surface,
                Rect::new(c.x, c.y, c.w, c.h - 36),
                &[
                    "MFDS   GO",
                    "FCR    GO",
                    "SMS    GO",
                    "INS    RDY",
                    "TGP    GO",
                    "HUD    GO",
                    "BIT    COMPLETE",
                ],
                None,
                page.font_px,
                pal.primary,
                pal.readout,
            );
            progress_strip(
                page.surface,
                Rect::new(c.x + 16, c.bottom() - 24, c.w - 32, 12),
                1.0,
                pal.nav,
                pal.structure,
            );
        }
        Format::Eng => {
            // Systems-style page (dedicated ENG MFD is library extension; real jet uses panel gauges).
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
            let bar_h = (c.h as f32 * 0.22) as i32;
            tape_gauge(
                page.surface,
                Rect::new(c.x, c.y, c.w, bar_h),
                TapeOpts {
                    orientation: TapeOrientation::Horizontal,
                    font_px: page.font_px * 0.8,
                    color: pal.primary,
                    value: 0.65 + 0.1 * (t * 0.1).cos(),
                    label: "TOTAL",
                },
            );
            let ty = c.y + bar_h + 6;
            let th = c.h - bar_h - 6;
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
                    Rect::new(c.x + i as i32 * (tw + 6), ty, tw, th),
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
        Format::Cni | Format::Ufc | Format::Pfl => {
            let lines: &[&str] = match fmt {
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
        Format::Ecm => {
            list_menu(
                page.surface,
                Rect::new(c.x, c.y, c.w, c.h - 40),
                &[
                    "ECM   STBY",
                    "RWR   NORM",
                    "CHAFF 30",
                    "FLARE 15",
                    "JAM   OFF",
                ],
                Some(0),
                page.font_px,
                pal.primary,
                pal.caution,
            );
            progress_strip(
                page.surface,
                Rect::new(c.x + 16, c.bottom() - 28, c.w - 32, 14),
                0.35 + 0.25 * (t * 0.3).sin().abs(),
                pal.caution,
                pal.structure,
            );
        }
        Format::Tfr | Format::HudRpt => {
            let (cx, cy) = c.center();
            let bank = 10.0 * (t * 0.4).sin();
            let rad = bank.to_radians();
            let half = c.w as f32 * 0.25;
            let dx = rad.cos() * half;
            let dy = rad.sin() * half;
            page.surface.line_aa(
                (cx as f32 - dx) as i32,
                (cy as f32 - dy) as i32,
                (cx as f32 + dx) as i32,
                (cy as f32 + dy) as i32,
                pal.primary,
            );
            ownship(page.surface, cx, cy, pal.readout);
            numeric_readout(
                page.surface,
                cx as f32,
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

fn s_circle_text(s: &mut Surface, x: i32, y: i32, text: &str, color: Color, px: f32) {
    s.circle(x, y, 10, color);
    let tw = text_width(text, px);
    draw_text(
        s,
        x as f32 - tw * 0.5,
        y as f32 - px * 0.35,
        text,
        color,
        px,
    );
}

// Thin wrappers
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
