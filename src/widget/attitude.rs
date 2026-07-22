//! Artificial horizon (attitude ball) + heading tape / cardinals.
//!
//! Real MFDs often show attitude on HUD or dedicated formats; vehicle use is
//! the same symbology for pitch/roll/heading.

use crate::font::{draw_text, draw_text_centered, text_width};
use crate::geom::Rect;
use crate::{Color, Surface};

/// Degrees → nearest 8- or 16-point compass label.
pub fn heading_cardinal(deg: f32) -> &'static str {
    let d = ((deg % 360.0) + 360.0) % 360.0;
    // 16-point: 22.5° sectors
    const NAMES: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    let i = ((d + 11.25) / 22.5) as usize % 16;
    NAMES[i]
}

/// Artificial horizon: sky/ground split, bank, pitch ladder cues, aircraft ref.
#[allow(clippy::too_many_arguments)]
pub fn attitude_ball(
    s: &mut Surface,
    rect: Rect,
    pitch_deg: f32,
    roll_deg: f32,
    sky: Color,
    ground: Color,
    ink: Color,
    dim: Color,
) {
    let cx = rect.center().0;
    let cy = rect.center().1;
    let r = (rect.w.min(rect.h) / 2 - 4).max(20);

    // Clip disk by only drawing inside circle (sample angles)
    let pitch = pitch_deg.clamp(-40.0, 40.0);
    let roll = roll_deg.to_radians();
    let (cr, sr) = (roll.cos(), roll.sin());

    // Horizon line offset by pitch (pixels per degree)
    let ppd = r as f32 / 45.0;
    let pitch_off = -pitch * ppd;

    // Fill sky/ground relative to rotated horizon
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy > r * r {
                continue;
            }
            // Un-rotate sample into horizon frame
            let x = dx as f32;
            let y = dy as f32;
            let uy = -x * sr + y * cr - pitch_off;
            let col = if uy < 0.0 { sky } else { ground };
            s.plot(cx + dx, cy + dy, col);
        }
    }

    // Horizon line (rotated through pitch offset)
    let half = r as f32 * 0.95;
    let oy = pitch_off;
    let p0x = cx as f32 + (-half) * cr - oy * sr;
    let p0y = cy as f32 + (-half) * sr + oy * cr;
    let p1x = cx as f32 + half * cr - oy * sr;
    let p1y = cy as f32 + half * sr + oy * cr;
    s.line_aa(p0x as i32, p0y as i32, p1x as i32, p1y as i32, ink);

    // Pitch ladder short ticks
    for step in [-20i32, -10, 10, 20] {
        let po = pitch_off + step as f32 * ppd;
        let half_t = r as f32 * 0.25;
        let ax = cx as f32 + (-half_t) * cr - po * sr;
        let ay = cy as f32 + (-half_t) * sr + po * cr;
        let bx = cx as f32 + half_t * cr - po * sr;
        let by = cy as f32 + half_t * sr + po * cr;
        s.line_aa(ax as i32, ay as i32, bx as i32, by as i32, dim);
    }

    // Fixed aircraft reference (wings)
    s.line_aa(cx - r / 3, cy, cx - 8, cy, ink);
    s.line_aa(cx + 8, cy, cx + r / 3, cy, ink);
    s.line_aa(cx - 8, cy, cx, cy + 6, ink);
    s.line_aa(cx + 8, cy, cx, cy + 6, ink);
    s.circle(cx, cy, 2, ink);

    // Bank scale ticks on rim
    for a in [-60i32, -30, 0, 30, 60] {
        let rad = (a as f32 + roll_deg).to_radians();
        let x0 = cx as f32 + (r as f32 - 6.0) * rad.sin();
        let y0 = cy as f32 - (r as f32 - 6.0) * rad.cos();
        let x1 = cx as f32 + r as f32 * rad.sin();
        let y1 = cy as f32 - r as f32 * rad.cos();
        s.line_aa(x0 as i32, y0 as i32, x1 as i32, y1 as i32, dim);
    }

    // Outer ring
    s.circle(cx, cy, r, ink);
}

/// Heading readout: large degrees + cardinal, optional tape ticks.
pub fn heading_display(
    s: &mut Surface,
    rect: Rect,
    heading_deg: f32,
    ink: Color,
    dim: Color,
    font_px: f32,
) {
    let hdg = ((heading_deg % 360.0) + 360.0) % 360.0;
    let card = heading_cardinal(hdg);
    let cx = rect.center().0 as f32;
    let cy = rect.center().1 as f32;
    draw_text_centered(
        s,
        cx,
        cy - font_px * 0.9,
        &format!("{hdg:05.1}°"),
        ink,
        font_px * 1.4,
    );
    draw_text_centered(s, cx, cy + font_px * 0.6, card, ink, font_px * 1.2);

    // Mini tape: neighbors
    let left = heading_cardinal(hdg - 45.0);
    let right = heading_cardinal(hdg + 45.0);
    draw_text(s, rect.x as f32 + 4.0, cy, left, dim, font_px * 0.75);
    let tw = text_width(right, font_px * 0.75);
    draw_text(
        s,
        rect.right() as f32 - tw - 4.0,
        cy,
        right,
        dim,
        font_px * 0.75,
    );

    // Lubber line
    s.line_aa(
        rect.center().0,
        rect.y + 2,
        rect.center().0,
        rect.y + 10,
        ink,
    );
}

/// Compact compass rose (heading-up: lubber at top).
#[allow(clippy::too_many_arguments)]
pub fn heading_rose(
    s: &mut Surface,
    cx: i32,
    cy: i32,
    r: i32,
    heading_deg: f32,
    ink: Color,
    dim: Color,
    font_px: f32,
) {
    s.circle(cx, cy, r, dim);
    let hdg = ((heading_deg % 360.0) + 360.0) % 360.0;
    // Heading-up: north is at -heading on the rose
    for (name, ang) in [("N", 0.0_f32), ("E", 90.0), ("S", 180.0), ("W", 270.0)] {
        let a = (ang - hdg).to_radians();
        let x = cx as f32 + (r as f32 - font_px) * a.sin();
        let y = cy as f32 - (r as f32 - font_px) * a.cos();
        draw_text_centered(
            s,
            x,
            y,
            name,
            if name == "N" { ink } else { dim },
            font_px * 0.7,
        );
    }
    // Ownship lubber at top of rose
    s.line_aa(cx, cy - r + 2, cx, cy - r + 12, ink);
    s.line_aa(cx - 4, cy - r + 10, cx, cy - r + 2, ink);
    s.line_aa(cx + 4, cy - r + 10, cx, cy - r + 2, ink);
}
