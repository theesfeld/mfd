//! Fighter **page calls** (F-16-class layouts as starting set).
//!
//! Each function fills a [`Page`] using multiple widgets — same pattern as a
//! real MFD: softkeys + content. Layouts are **inspired by public/sim
//! documentation**, not OEM ROM.

use crate::color::{AMBER, CYAN, GREEN, GREEN_DIM, RED, WHITE};
use crate::geom::Rect;
use crate::page::Page;
use crate::widget::{RoundGaugeOpts, TapeOpts, TapeOrientation};

/// Shared top OSB set used by several F-16-style pages.
pub const F16_OSB_TOP: &[&str] = &["BLANK", "HAD", "SMS", "HSD", "DTE", "TEST"];

/// Bottom OSB set (content-dependent; representative set).
pub const F16_OSB_BOT: &[&str] = &["SWAP", "SMS", "HSD", "TGP", "DCLT", "CNTL"];

fn chrome(page: &mut Page, top_sel: Option<usize>, title: &str) {
    let b = page.bounds.inset(4);
    // Softkey row tall enough for B612 ascent + descender gap.
    let th = (page.font_px * 1.8).ceil() as i32 + 6;
    page.softkeys(Rect::new(b.x, b.y, b.w, th), F16_OSB_TOP, top_sel);
    page.softkeys(
        Rect::new(b.x, b.bottom() - th, b.w, th),
        F16_OSB_BOT,
        None,
    );
    page.label_centered(
        b.center().0 as f32,
        b.y as f32 + th as f32 + page.font_px * 0.7,
        title,
        GREEN,
    );
}

/// Stores Management System page — weapon stations / load summary.
pub fn sms(page: &mut Page, selected_station: usize, master_arm: bool) {
    page.clear();
    page.bezel();
    chrome(page, Some(2), "SMS");
    let c = page.content_rect((page.font_px * 2.8) as i32, (page.font_px * 1.6) as i32);
    let stations = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
    let cols = 3i32;
    let cell_w = c.w / cols;
    let cell_h = c.h / 3;
    for (i, st) in stations.iter().enumerate() {
        let col = (i as i32) % cols;
        let row = (i as i32) / cols;
        let r = Rect::new(c.x + col * cell_w, c.y + row * cell_h, cell_w - 4, cell_h - 4);
        page.surface
            .line_aa(r.x, r.y, r.right(), r.y, GREEN_DIM);
        page.surface
            .line_aa(r.right(), r.y, r.right(), r.bottom(), GREEN_DIM);
        page.surface
            .line_aa(r.right(), r.bottom(), r.x, r.bottom(), GREEN_DIM);
        page.surface
            .line_aa(r.x, r.bottom(), r.x, r.y, GREEN_DIM);
        let colr = if i == selected_station { WHITE } else { GREEN };
        page.label_centered(r.center().0 as f32, r.center().1 as f32, st, colr);
    }
    let arm = if master_arm { "MASTER ARM" } else { "SAFE" };
    let arm_c = if master_arm { RED } else { GREEN_DIM };
    page.label(c.x as f32 + 4.0, c.bottom() as f32 - page.font_px, arm, arm_c);
}

/// Horizontal Situation Display — range rings + heading cue.
pub fn hsd(page: &mut Page, heading_deg: f32, range_nm: f32) {
    page.clear();
    page.bezel();
    chrome(page, Some(3), "HSD");
    let c = page.content_rect((page.font_px * 2.8) as i32, (page.font_px * 1.6) as i32);
    let (cx, cy) = c.center();
    let r = (c.w.min(c.h) / 2 - 12).max(20);
    for ring in 1..=3 {
        page.surface.circle(cx, cy, r * ring / 3, CYAN);
    }
    // Ownship + heading line
    let rad = heading_deg.to_radians() - std::f32::consts::FRAC_PI_2;
    let len = r as f32 * 0.9;
    page.surface.line_aa(
        cx,
        cy,
        cx + (len * rad.cos()) as i32,
        cy + (len * rad.sin()) as i32,
        WHITE,
    );
    page.surface.circle(cx, cy, 4, GREEN);
    page.label(
        c.x as f32 + 4.0,
        c.y as f32 + 4.0,
        &format!("HDG {:.0}", heading_deg.rem_euclid(360.0)),
        GREEN,
    );
    page.label(
        c.x as f32 + 4.0,
        c.y as f32 + page.font_px + 6.0,
        &format!("RNG {:.0} NM", range_nm),
        GREEN_DIM,
    );
}

/// Targeting pod page — track box + status.
pub fn tgp(page: &mut Page, track_x: f32, track_y: f32, laser: bool) {
    page.clear();
    page.bezel();
    chrome(page, None, "TGP");
    let c = page.content_rect((page.font_px * 2.8) as i32, (page.font_px * 1.6) as i32);
    // FOV frame
    let fov = c.inset(c.w / 8);
    page.surface
        .line_aa(fov.x, fov.y, fov.right(), fov.y, GREEN_DIM);
    page.surface
        .line_aa(fov.right(), fov.y, fov.right(), fov.bottom(), GREEN_DIM);
    page.surface
        .line_aa(fov.right(), fov.bottom(), fov.x, fov.bottom(), GREEN_DIM);
    page.surface
        .line_aa(fov.x, fov.bottom(), fov.x, fov.y, GREEN_DIM);
    // Track gate
    let tx = fov.x as f32 + track_x.clamp(0.0, 1.0) * fov.w as f32;
    let ty = fov.y as f32 + track_y.clamp(0.0, 1.0) * fov.h as f32;
    let g = 12;
    page.surface.line_aa(
        tx as i32 - g,
        ty as i32 - g,
        tx as i32 + g,
        ty as i32 - g,
        WHITE,
    );
    page.surface.line_aa(
        tx as i32 + g,
        ty as i32 - g,
        tx as i32 + g,
        ty as i32 + g,
        WHITE,
    );
    page.surface.line_aa(
        tx as i32 + g,
        ty as i32 + g,
        tx as i32 - g,
        ty as i32 + g,
        WHITE,
    );
    page.surface.line_aa(
        tx as i32 - g,
        ty as i32 + g,
        tx as i32 - g,
        ty as i32 - g,
        WHITE,
    );
    let lz = if laser { "LASER ARM" } else { "LASER SAFE" };
    page.label(
        c.x as f32 + 4.0,
        c.bottom() as f32 - page.font_px,
        lz,
        if laser { RED } else { GREEN_DIM },
    );
}

/// Fire-control radar style page — range/azimuth grid + contact.
pub fn fcr(page: &mut Page, az_frac: f32, rng_frac: f32) {
    page.clear();
    page.bezel();
    chrome(page, None, "FCR");
    let c = page.content_rect((page.font_px * 2.8) as i32, (page.font_px * 1.6) as i32);
    // B-scope style grid
    for i in 0..=4 {
        let x = c.x + c.w * i / 4;
        let y = c.y + c.h * i / 4;
        page.surface.line_aa(x, c.y, x, c.bottom(), GREEN_DIM);
        page.surface.line_aa(c.x, y, c.right(), y, GREEN_DIM);
    }
    let px = c.x as f32 + az_frac.clamp(0.0, 1.0) * c.w as f32;
    let py = c.bottom() as f32 - rng_frac.clamp(0.0, 1.0) * c.h as f32;
    page.surface.circle(px as i32, py as i32, 5, AMBER);
    page.label(c.x as f32 + 4.0, c.y as f32 + 4.0, "RWS", GREEN);
}

/// Engine / systems style page — round gauges + tapes (jet EMS flavor).
pub fn eng(page: &mut Page, rpm: f32, noz: f32, oil: f32, ftit: f32) {
    page.clear();
    page.bezel();
    chrome(page, None, "ENG");
    let c = page.content_rect((page.font_px * 2.8) as i32, (page.font_px * 1.6) as i32);
    let half_w = c.w / 2;
    page.round_gauge(
        Rect::new(c.x, c.y, half_w, c.h / 2),
        RoundGaugeOpts {
            value: rpm,
            redline: Some(0.85),
            label: "RPM",
            font_px: page.font_px * 0.85,
            ..Default::default()
        },
    );
    page.round_gauge(
        Rect::new(c.x + half_w, c.y, half_w, c.h / 2),
        RoundGaugeOpts {
            value: noz,
            redline: None,
            label: "NOZ",
            color: CYAN,
            font_px: page.font_px * 0.85,
            ..Default::default()
        },
    );
    let tape_h = c.h / 2 - 8;
    let ty = c.y + c.h / 2 + 4;
    let tw = c.w / 2 - 8;
    page.tape(
        Rect::new(c.x + 4, ty, tw, tape_h),
        TapeOpts {
            orientation: TapeOrientation::Vertical,
            font_px: page.font_px * 0.8,
            color: AMBER,
            value: oil,
            label: "OIL",
        },
    );
    page.tape(
        Rect::new(c.x + tw + 8, ty, tw, tape_h),
        TapeOpts {
            orientation: TapeOrientation::Vertical,
            font_px: page.font_px * 0.8,
            color: RED,
            value: ftit,
            label: "FTIT",
        },
    );
}

/// Data transfer / DTC style page — simple list.
pub fn dte(page: &mut Page, lines: &[&str]) {
    page.clear();
    page.bezel();
    chrome(page, Some(4), "DTE");
    let c = page.content_rect((page.font_px * 2.8) as i32, (page.font_px * 1.6) as i32);
    for (i, line) in lines.iter().enumerate().take(12) {
        let y = c.y as f32 + 4.0 + i as f32 * (page.font_px + 4.0);
        page.label(c.x as f32 + 8.0, y, line, GREEN);
    }
}

/// Built-in test page.
pub fn test(page: &mut Page, ok: bool) {
    page.clear();
    page.bezel();
    chrome(page, Some(5), "TEST");
    let msg = if ok { "BIT GO" } else { "BIT FAIL" };
    let col = if ok { GREEN } else { RED };
    let (cx, cy) = page.bounds.center();
    page.label_centered(cx as f32, cy as f32, msg, col);
}

/// Blank page with chrome only.
pub fn blank(page: &mut Page) {
    page.clear();
    page.bezel();
    chrome(page, Some(0), "BLANK");
}

/// Fuel page — tapes (also reusable for automotive fuel).
pub fn fuel(page: &mut Page, total: f32, internal: f32, external: f32) {
    page.clear();
    page.bezel();
    chrome(page, None, "FUEL");
    let c = page.content_rect((page.font_px * 2.8) as i32, (page.font_px * 1.6) as i32);
    let tw = c.w / 3 - 6;
    for (i, (lab, val, col)) in [
        ("TOT", total, GREEN),
        ("INT", internal, CYAN),
        ("EXT", external, AMBER),
    ]
    .iter()
    .enumerate()
    {
        page.tape(
            Rect::new(c.x + i as i32 * (tw + 6), c.y, tw, c.h),
            TapeOpts {
                orientation: TapeOrientation::Vertical,
                font_px: page.font_px * 0.85,
                color: *col,
                value: *val,
                label: lab,
            },
        );
    }
}
