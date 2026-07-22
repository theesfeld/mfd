//! Artificial horizon (attitude ball) + heading tape / cardinals.
//!
//! Real MFDs often show attitude on HUD or dedicated formats; vehicle use is
//! the same symbology for pitch/roll/heading.

use crate::font::{draw_text, draw_text_centered, text_width};
use crate::geom::Rect;
use crate::{Color, Surface};

/// Degrees → nearest 16-point compass label.
pub fn heading_cardinal(deg: f32) -> &'static str {
    let d = ((deg % 360.0) + 360.0) % 360.0;
    const NAMES: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    let i = ((d + 11.25) / 22.5) as usize % 16;
    NAMES[i]
}

fn scale_rgb(c: Color, f: f32) -> Color {
    let a = (c >> 24) & 0xFF;
    let r = (((c >> 16) & 0xFF) as f32 * f).clamp(0.0, 255.0) as u32;
    let g = (((c >> 8) & 0xFF) as f32 * f).clamp(0.0, 255.0) as u32;
    let b = ((c & 0xFF) as f32 * f).clamp(0.0, 255.0) as u32;
    (a << 24) | (r << 16) | (g << 8) | b
}

/// Artificial horizon sphere: sky/ground, bank, pitch ladder, compass on horizon.
///
/// `heading_deg` places N/E/S/W (and mid marks) on the horizon ring so the ball
/// shows direction as well as tilt.
#[allow(clippy::too_many_arguments)]
pub fn attitude_ball(
    s: &mut Surface,
    rect: Rect,
    pitch_deg: f32,
    roll_deg: f32,
    heading_deg: f32,
    sky: Color,
    ground: Color,
    ink: Color,
    dim: Color,
) {
    let cx = rect.center().0;
    let cy = rect.center().1;
    let r = (rect.w.min(rect.h) / 2 - 4).max(24);
    let r_f = r as f32;

    let pitch = pitch_deg.clamp(-60.0, 60.0);
    let roll = roll_deg.to_radians();
    let (cr, sr) = (roll.cos(), roll.sin());

    // Stronger pitch scale: more of the sphere moves with attitude.
    let ppd = r_f / 35.0;
    let pitch_off = -pitch * ppd;

    // Sphere fill with limb darkening + light from upper-left (3D ball look).
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let x = dx as f32;
            let y = dy as f32;
            // Unit disk → approximate sphere Z for shading.
            let nz = ((1.0 - (d2 as f32) / (r_f * r_f)).max(0.0)).sqrt();
            // Light vector (nx, ny, nz) · L
            let lx = -0.45;
            let ly = -0.55;
            let lz = 0.70;
            let ndotl = (x / r_f * lx + y / r_f * ly + nz * lz).clamp(0.0, 1.0);
            let shade = 0.35 + 0.65 * ndotl;

            // Un-rotate into horizon frame
            let uy = -x * sr + y * cr - pitch_off;
            let base = if uy < 0.0 { sky } else { ground };
            // Near horizon: slight desat/dark band for depth
            let near = (uy.abs() / (r_f * 0.12)).clamp(0.0, 1.0);
            let depth = 0.85 + 0.15 * near;
            s.plot(cx + dx, cy + dy, scale_rgb(base, shade * depth));
        }
    }

    // Horizon line (thick)
    let half = r_f * 0.98;
    let oy = pitch_off;
    let p0x = cx as f32 + (-half) * cr - oy * sr;
    let p0y = cy as f32 + (-half) * sr + oy * cr;
    let p1x = cx as f32 + half * cr - oy * sr;
    let p1y = cy as f32 + half * sr + oy * cr;
    s.line_thick(p0x as i32, p0y as i32, p1x as i32, p1y as i32, ink, 2);

    // Pitch ladder with labels
    for step in [-30i32, -20, -10, 10, 20, 30] {
        let po = pitch_off + step as f32 * ppd;
        // Cull ticks far outside disk
        if po.abs() > r_f * 0.92 {
            continue;
        }
        let half_t = r_f * if step % 20 == 0 { 0.32 } else { 0.22 };
        let ax = cx as f32 + (-half_t) * cr - po * sr;
        let ay = cy as f32 + (-half_t) * sr + po * cr;
        let bx = cx as f32 + half_t * cr - po * sr;
        let by = cy as f32 + half_t * sr + po * cr;
        s.line_aa(ax as i32, ay as i32, bx as i32, by as i32, dim);
        // Numeric pitch on right end of ladder
        let lx = cx as f32 + (half_t + 6.0) * cr - po * sr;
        let ly = cy as f32 + (half_t + 6.0) * sr + po * cr;
        if (lx - cx as f32).hypot(ly - cy as f32) < r_f - 8.0 {
            draw_text_centered(s, lx, ly - 3.0, &format!("{}", step.abs()), dim, 9.0);
        }
    }

    // Compass marks on the horizon line (heading-relative world directions).
    // World bearing B is at angle (B - heading) along the horizon from center.
    let hdg = ((heading_deg % 360.0) + 360.0) % 360.0;
    let marks: [(&str, f32, bool); 8] = [
        ("N", 0.0, true),
        ("NE", 45.0, false),
        ("E", 90.0, true),
        ("SE", 135.0, false),
        ("S", 180.0, true),
        ("SW", 225.0, false),
        ("W", 270.0, true),
        ("NW", 315.0, false),
    ];
    for &(name, bearing, major) in &marks {
        // Along-horizon offset: +right when looking over the nose (heading-up).
        let delta = (bearing - hdg + 540.0) % 360.0 - 180.0; // -180..180
                                                             // Map degrees of yaw to pixels along horizon (same ppd-ish scale).
        let along = (delta / 90.0) * (r_f * 0.85);
        if along.abs() > r_f * 0.88 {
            continue;
        }
        // Point on horizon in ball frame, then rotate by bank.
        let hx = along;
        let hy = pitch_off;
        let sx = cx as f32 + hx * cr - hy * sr;
        let sy = cy as f32 + hx * sr + hy * cr;
        if (sx - cx as f32).hypot(sy - cy as f32) > r_f - 4.0 {
            continue;
        }
        // Tick into sky (horizon-frame (0, -tick), bank-rotated).
        let tick = if major { 10.0 } else { 6.0 };
        let t1x = sx + (-tick) * sr;
        let t1y = sy + (-tick) * cr;
        s.line_aa(sx as i32, sy as i32, t1x as i32, t1y as i32, ink);
        if major {
            draw_text_centered(s, t1x, t1y - 10.0, name, ink, 10.0);
        } else {
            draw_text_centered(s, t1x, t1y - 8.0, name, dim, 8.0);
        }
    }

    // Fixed aircraft reference (wings + chevron) — stays level with glass
    s.line_thick(cx - r / 3, cy, cx - 10, cy, ink, 2);
    s.line_thick(cx + 10, cy, cx + r / 3, cy, ink, 2);
    s.line_aa(cx - 10, cy, cx, cy + 8, ink);
    s.line_aa(cx + 10, cy, cx, cy + 8, ink);
    s.circle(cx, cy, 3, ink);

    // Bank scale on outer rim (fixed marks) + rotating triangle pointer at top
    for a in [-60i32, -45, -30, -20, -10, 0, 10, 20, 30, 45, 60] {
        let rad = (a as f32).to_radians();
        let len = if a == 0 || a.abs() == 30 || a.abs() == 60 {
            8.0
        } else {
            5.0
        };
        let x0 = cx as f32 + (r_f - len) * rad.sin();
        let y0 = cy as f32 - (r_f - len) * rad.cos();
        let x1 = cx as f32 + r_f * rad.sin();
        let y1 = cy as f32 - r_f * rad.cos();
        s.line_aa(x0 as i32, y0 as i32, x1 as i32, y1 as i32, dim);
    }
    // Bank pointer (moves with roll): triangle at roll angle on rim
    {
        let rad = roll;
        let tip_x = cx as f32 + (r_f + 2.0) * rad.sin();
        let tip_y = cy as f32 - (r_f + 2.0) * rad.cos();
        let base = r_f - 10.0;
        let b0x = cx as f32 + base * (rad + 0.12).sin();
        let b0y = cy as f32 - base * (rad + 0.12).cos();
        let b1x = cx as f32 + base * (rad - 0.12).sin();
        let b1y = cy as f32 - base * (rad - 0.12).cos();
        s.line_aa(tip_x as i32, tip_y as i32, b0x as i32, b0y as i32, ink);
        s.line_aa(tip_x as i32, tip_y as i32, b1x as i32, b1y as i32, ink);
        s.line_aa(b0x as i32, b0y as i32, b1x as i32, b1y as i32, ink);
    }
    // Fixed lubber at top of rim
    s.line_thick(cx, cy - r - 2, cx, cy - r + 8, ink, 2);

    // Outer rings for sphere shell
    s.circle(cx, cy, r, ink);
    s.circle(cx, cy, r + 1, dim);
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
    s.line_aa(cx, cy - r + 2, cx, cy - r + 12, ink);
    s.line_aa(cx - 4, cy - r + 10, cx, cy - r + 2, ink);
    s.line_aa(cx + 4, cy - r + 10, cx, cy - r + 2, ink);
}
