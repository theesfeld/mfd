//! Additional MFD widgets (rings, gates, grids, lists, …).

use crate::color::WHITE;
use crate::font::{draw_text, draw_text_centered, text_height, text_width};
use crate::geom::Rect;
use crate::{Color, Surface};
use std::f32::consts::PI;

/// Concentric range rings (HSD).
pub fn range_rings(s: &mut Surface, cx: i32, cy: i32, r_outer: i32, n: i32, color: Color) {
    let n = n.clamp(1, 8);
    for i in 1..=n {
        let r = r_outer * i / n;
        if r > 0 {
            s.circle(cx, cy, r, color);
        }
    }
}

/// Bearing / heading pointer from center.
pub fn bearing_pointer(s: &mut Surface, cx: i32, cy: i32, len: f32, deg: f32, color: Color) {
    let rad = deg.to_radians() - PI * 0.5;
    s.line_aa(
        cx,
        cy,
        cx + (len * rad.cos()) as i32,
        cy + (len * rad.sin()) as i32,
        color,
    );
}

/// Square track gate centered at (cx, cy).
pub fn track_gate(s: &mut Surface, cx: i32, cy: i32, half: i32, color: Color) {
    let h = half.max(4);
    s.line_aa(cx - h, cy - h, cx + h, cy - h, color);
    s.line_aa(cx + h, cy - h, cx + h, cy + h, color);
    s.line_aa(cx + h, cy + h, cx - h, cy + h, color);
    s.line_aa(cx - h, cy + h, cx - h, cy - h, color);
}

/// Simple crosshair.
pub fn crosshair(s: &mut Surface, cx: i32, cy: i32, arm: i32, gap: i32, color: Color) {
    let a = arm.max(6);
    let g = gap.clamp(0, a - 1);
    s.line_aa(cx - a, cy, cx - g, cy, color);
    s.line_aa(cx + g, cy, cx + a, cy, color);
    s.line_aa(cx, cy - a, cx, cy - g, color);
    s.line_aa(cx, cy + g, cx, cy + a, color);
}

/// Radar B-scope style grid in rect.
pub fn bscope_grid(s: &mut Surface, rect: Rect, div: i32, color: Color) {
    let d = div.clamp(2, 16);
    for i in 0..=d {
        let x = rect.x + rect.w * i / d;
        let y = rect.y + rect.h * i / d;
        s.line_aa(x, rect.y, x, rect.bottom(), color);
        s.line_aa(rect.x, y, rect.right(), y, color);
    }
}

/// Selectable text list (DTE / PFL / CNI).
pub fn list_menu(
    s: &mut Surface,
    rect: Rect,
    lines: &[&str],
    selected: Option<usize>,
    font_px: f32,
    color: Color,
    sel_color: Color,
) {
    let row_h = font_px + 4.0;
    for (i, line) in lines.iter().enumerate() {
        let y = rect.y as f32 + 2.0 + i as f32 * row_h;
        if y + font_px > rect.bottom() as f32 {
            break;
        }
        let col = if selected == Some(i) {
            sel_color
        } else {
            color
        };
        if selected == Some(i) {
            s.line_aa(
                rect.x + 2,
                y as i32 + (font_px * 0.5) as i32,
                rect.x + 8,
                y as i32 + (font_px * 0.5) as i32,
                sel_color,
            );
        }
        draw_text(s, rect.x as f32 + 12.0, y, line, col, font_px);
    }
}

/// Weapon station grid (SMS).
#[allow(clippy::too_many_arguments)]
pub fn station_grid(
    s: &mut Surface,
    rect: Rect,
    labels: &[&str],
    cols: i32,
    selected: usize,
    font_px: f32,
    color: Color,
    sel_color: Color,
) {
    let cols = cols.clamp(1, 6);
    let n = labels.len() as i32;
    let rows = (n + cols - 1) / cols;
    if rows <= 0 {
        return;
    }
    let cw = rect.w / cols;
    let ch = rect.h / rows;
    for (i, lab) in labels.iter().enumerate() {
        let col = (i as i32) % cols;
        let row = (i as i32) / cols;
        let r = Rect::new(rect.x + col * cw, rect.y + row * ch, cw - 4, ch - 4);
        let c = if i == selected { sel_color } else { color };
        s.line_aa(r.x, r.y, r.right(), r.y, color);
        s.line_aa(r.right(), r.y, r.right(), r.bottom(), color);
        s.line_aa(r.right(), r.bottom(), r.x, r.bottom(), color);
        s.line_aa(r.x, r.bottom(), r.x, r.y, color);
        draw_text_centered(s, r.center().0 as f32, r.center().1 as f32, lab, c, font_px);
    }
}

/// Large numeric / short string readout.
pub fn numeric_readout(s: &mut Surface, cx: f32, cy: f32, text: &str, color: Color, font_px: f32) {
    draw_text_centered(s, cx, cy, text, color, font_px);
}

/// Caution / warning box with title.
pub fn caution_box(s: &mut Surface, rect: Rect, title: &str, font_px: f32, color: Color) {
    s.line_aa(rect.x, rect.y, rect.right(), rect.y, color);
    s.line_aa(rect.right(), rect.y, rect.right(), rect.bottom(), color);
    s.line_aa(rect.right(), rect.bottom(), rect.x, rect.bottom(), color);
    s.line_aa(rect.x, rect.bottom(), rect.x, rect.y, color);
    draw_text_centered(
        s,
        rect.center().0 as f32,
        rect.center().1 as f32,
        title,
        color,
        font_px,
    );
}

/// Artificial horizon cue (pitch ladder stub — single bar + bank).
pub fn horizon_cue(s: &mut Surface, cx: i32, cy: i32, half: i32, bank_deg: f32, color: Color) {
    let rad = bank_deg.to_radians();
    let (c, sn) = (rad.cos(), rad.sin());
    let h = half as f32;
    let x0 = cx as f32 - h * c;
    let y0 = cy as f32 - h * sn;
    let x1 = cx as f32 + h * c;
    let y1 = cy as f32 + h * sn;
    s.line_aa(x0 as i32, y0 as i32, x1 as i32, y1 as i32, color);
    s.line_aa(cx - 8, cy, cx - 2, cy, color);
    s.line_aa(cx + 2, cy, cx + 8, cy, color);
}

/// Horizontal progress / load strip 0..1.
pub fn progress_strip(s: &mut Surface, rect: Rect, value: f32, color: Color, frame: Color) {
    let v = value.clamp(0.0, 1.0);
    s.line_aa(rect.x, rect.y, rect.right(), rect.y, frame);
    s.line_aa(rect.right(), rect.y, rect.right(), rect.bottom(), frame);
    s.line_aa(rect.right(), rect.bottom(), rect.x, rect.bottom(), frame);
    s.line_aa(rect.x, rect.bottom(), rect.x, rect.y, frame);
    let fill = ((rect.w as f32) * v) as i32;
    if fill > 1 {
        let mid = rect.y + rect.h / 2;
        s.line_aa(rect.x + 1, mid, rect.x + fill, mid, color);
        s.line_aa(rect.x + 1, mid + 1, rect.x + fill, mid + 1, color);
    }
}

/// Sensor video frame (empty FOV box).
pub fn video_frame(s: &mut Surface, rect: Rect, color: Color) {
    s.line_aa(rect.x, rect.y, rect.right(), rect.y, color);
    s.line_aa(rect.right(), rect.y, rect.right(), rect.bottom(), color);
    s.line_aa(rect.right(), rect.bottom(), rect.x, rect.bottom(), color);
    s.line_aa(rect.x, rect.bottom(), rect.x, rect.y, color);
}

/// Full OSB chrome labels **inside** the face (no clip off edges).
///
/// Side labels use a reserved margin and short strings. Real MFD OSB text sits
/// in the bezel outside glass; on a single square framebuffer we reserve a
/// border so labels stay visible.
///
/// * `active` — SOI OSB (displayed format/support). Steady bright + underline box.
/// * `flash` — off-glass owning format slot (warn pointer). Uses `flash_color` when `flash_on`.
#[allow(clippy::too_many_arguments)]
pub fn osb_chrome(
    s: &mut Surface,
    bounds: Rect,
    top: &[&str; 5],
    right: &[&str; 5],
    bottom: &[&str; 5],
    left: &[&str; 5],
    font_px: f32,
    color: Color,
    active: Option<u8>,
) {
    osb_chrome_ex(
        s, bounds, top, right, bottom, left, font_px, color, active, None, false, WHITE,
    );
}

/// Extended chrome: active SOI + optional warning flash on a second OSB.
#[allow(clippy::too_many_arguments)]
pub fn osb_chrome_ex(
    s: &mut Surface,
    bounds: Rect,
    top: &[&str; 5],
    right: &[&str; 5],
    bottom: &[&str; 5],
    left: &[&str; 5],
    font_px: f32,
    color: Color,
    active: Option<u8>,
    flash: Option<u8>,
    flash_on: bool,
    flash_color: Color,
) {
    let fp = font_px.clamp(8.0, 14.0);
    let margin = (fp * 2.2).ceil() as i32 + 6;
    let margin = margin.min(bounds.w / 6).min(bounds.h / 6).max(18);

    let top_y = bounds.y + margin / 2;
    let bot_y = bounds.bottom() - margin / 2;
    let left_x = bounds.x + margin / 2;
    let right_x = bounds.right() - margin / 2;

    let slot_w = (bounds.w - 2 * margin).max(10) / 5;
    let slot_h = (bounds.h - 2 * margin).max(10) / 5;
    let inner_x0 = bounds.x + margin;
    let inner_y0 = bounds.y + margin;

    let color_for = |osb: u8| -> Color {
        if flash == Some(osb) && flash_on {
            flash_color
        } else if active == Some(osb) {
            WHITE
        } else {
            color
        }
    };

    let mark_active =
        |s: &mut Surface, cx: f32, cy: f32, lab: &str, osb: u8, col: Color, fsz: f32| {
            if lab.is_empty() {
                return;
            }
            draw_text_centered(s, cx, cy, lab, col, fsz);
            if active == Some(osb) {
                // Underline box — pilot knows SOI without reading content.
                let tw = text_width(lab, fsz);
                let y = (cy + text_height(fsz) * 0.42) as i32;
                let x0 = (cx - tw * 0.5) as i32 - 2;
                let x1 = (cx + tw * 0.5) as i32 + 2;
                s.line_aa(x0, y, x1, y, WHITE);
                s.line_aa(x0, y + 1, x1, y + 1, WHITE);
            }
        };

    for i in 0..5usize {
        let ii = i as i32;
        let cx = (inner_x0 + slot_w * ii + slot_w / 2) as f32;

        // Top OSB 1-5
        let osb = (i + 1) as u8;
        mark_active(s, cx, top_y as f32, top[i], osb, color_for(osb), fp);

        // Bottom OSB 15..11
        let osb = (15 - i) as u8;
        mark_active(s, cx, bot_y as f32, bottom[i], osb, color_for(osb), fp);

        let cy = (inner_y0 + slot_h * ii + slot_h / 2) as f32;

        // Right OSB 6-10
        let osb = (i + 6) as u8;
        let lab = right[i];
        let tw = text_width(lab, fp * 0.8);
        let rx = (right_x as f32).min(bounds.right() as f32 - tw * 0.5 - 2.0);
        mark_active(s, rx, cy, lab, osb, color_for(osb), fp * 0.8);

        // Left OSB 20-16
        let osb = (20 - i) as u8;
        let lab = left[i];
        let tw = text_width(lab, fp * 0.8);
        let lx = (left_x as f32).max(bounds.x as f32 + tw * 0.5 + 2.0);
        mark_active(s, lx, cy, lab, osb, color_for(osb), fp * 0.8);
    }
}

/// Inner content rect after OSB margin (use with [`osb_chrome`]).
pub fn content_after_osb(bounds: Rect, font_px: f32) -> Rect {
    let fp = font_px.clamp(8.0, 14.0);
    let margin = (fp * 2.2).ceil() as i32 + 6;
    let margin = margin.min(bounds.w / 6).min(bounds.h / 6).max(18);
    bounds.inset(margin + 4)
}
