//! Discrete **status** widgets (ON/OFF cells, tire TPM layout).
//!
//! Automotive pages use these like jet station grids: same glass language,
//! different data.

use crate::font::{draw_text, draw_text_centered, text_width};
use crate::geom::Rect;
use crate::{Color, Surface};

/// One labelled discrete status (light, door, belt, drive mode, …).
#[derive(Clone, Copy, Debug)]
pub struct StatusItem {
    pub label: &'static str,
    /// True = active / closed / ON / buckled (context-dependent).
    pub on: bool,
}

/// Grid of status cells (MFD list/station style).
pub fn status_grid(
    s: &mut Surface,
    rect: Rect,
    items: &[StatusItem],
    cols: i32,
    font_px: f32,
    on_color: Color,
    off_color: Color,
) {
    if items.is_empty() || rect.w < 8 || rect.h < 8 {
        return;
    }
    let cols = cols.clamp(1, 6);
    let n = items.len() as i32;
    let rows = (n + cols - 1) / cols;
    let cw = (rect.w / cols).max(8);
    let ch = (rect.h / rows).max(12);
    let fh = font_px.clamp(8.0, 14.0);
    for (i, it) in items.iter().enumerate() {
        let col = (i as i32) % cols;
        let row = (i as i32) / cols;
        let r = Rect::new(rect.x + col * cw + 1, rect.y + row * ch + 1, cw - 2, ch - 2);
        let c = if it.on { on_color } else { off_color };
        // Cell frame
        s.line_aa(r.x, r.y, r.right(), r.y, c);
        s.line_aa(r.right(), r.y, r.right(), r.bottom(), c);
        s.line_aa(r.right(), r.bottom(), r.x, r.bottom(), c);
        s.line_aa(r.x, r.bottom(), r.x, r.y, c);
        // Fill bar on left when ON
        if it.on {
            s.line_aa(r.x + 2, r.y + 2, r.x + 2, r.bottom() - 2, c);
            s.line_aa(r.x + 3, r.y + 2, r.x + 3, r.bottom() - 2, c);
        }
        draw_text_centered(s, r.center().0 as f32, r.center().1 as f32, it.label, c, fh);
    }
}

/// One tire corner for TPM page.
#[derive(Clone, Copy, Debug, Default)]
pub struct TireReading {
    /// Pressure (psi or kPa — page labels unit).
    pub pressure: f32,
    /// Temperature °C.
    pub temp_c: f32,
    pub alert: bool,
}

/// Vehicle plan view: FL FR / RL RR tire cells + center car outline.
#[allow(clippy::too_many_arguments)]
pub fn tire_grid(
    s: &mut Surface,
    rect: Rect,
    fl: TireReading,
    fr: TireReading,
    rl: TireReading,
    rr: TireReading,
    font_px: f32,
    ok: Color,
    alert: Color,
    structure: Color,
) {
    let fh = font_px.clamp(8.0, 13.0);
    let mid_x = rect.center().0;
    let mid_y = rect.center().1;
    // Simple car body box
    let body = Rect::new(
        mid_x - rect.w / 8,
        mid_y - rect.h / 5,
        rect.w / 4,
        rect.h * 2 / 5,
    );
    s.line_aa(body.x, body.y, body.right(), body.y, structure);
    s.line_aa(body.right(), body.y, body.right(), body.bottom(), structure);
    s.line_aa(
        body.right(),
        body.bottom(),
        body.x,
        body.bottom(),
        structure,
    );
    s.line_aa(body.x, body.bottom(), body.x, body.y, structure);

    let cell_w = rect.w / 3;
    let cell_h = rect.h / 3;
    let corners = [
        (Rect::new(rect.x, rect.y, cell_w, cell_h), fl, "FL"),
        (
            Rect::new(rect.right() - cell_w, rect.y, cell_w, cell_h),
            fr,
            "FR",
        ),
        (
            Rect::new(rect.x, rect.bottom() - cell_h, cell_w, cell_h),
            rl,
            "RL",
        ),
        (
            Rect::new(
                rect.right() - cell_w,
                rect.bottom() - cell_h,
                cell_w,
                cell_h,
            ),
            rr,
            "RR",
        ),
    ];
    for (r, t, name) in corners {
        let c = if t.alert { alert } else { ok };
        s.line_aa(r.x + 2, r.y + 2, r.right() - 2, r.y + 2, c);
        s.line_aa(r.right() - 2, r.y + 2, r.right() - 2, r.bottom() - 2, c);
        s.line_aa(r.right() - 2, r.bottom() - 2, r.x + 2, r.bottom() - 2, c);
        s.line_aa(r.x + 2, r.bottom() - 2, r.x + 2, r.y + 2, c);
        draw_text_centered(s, r.center().0 as f32, r.y as f32 + fh, name, c, fh * 0.9);
        draw_text_centered(
            s,
            r.center().0 as f32,
            r.center().1 as f32,
            &format!("{:.0}", t.pressure),
            c,
            fh,
        );
        draw_text_centered(
            s,
            r.center().0 as f32,
            r.bottom() as f32 - fh,
            &format!("{:.0}C", t.temp_c),
            c,
            fh * 0.85,
        );
    }
}

/// Big value + unit under a short title (speed, temp).
#[allow(clippy::too_many_arguments)]
pub fn value_readout(
    s: &mut Surface,
    cx: f32,
    cy: f32,
    title: &str,
    value: &str,
    unit: &str,
    color: Color,
    title_px: f32,
    value_px: f32,
) {
    draw_text_centered(s, cx, cy - value_px * 0.7, title, color, title_px);
    draw_text_centered(s, cx, cy + value_px * 0.15, value, color, value_px);
    if !unit.is_empty() {
        let tw = text_width(value, value_px);
        draw_text(
            s,
            cx + tw * 0.55,
            cy + value_px * 0.05,
            unit,
            color,
            title_px,
        );
    }
}
