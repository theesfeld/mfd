//! Round / arc **gauge** (tach, RPM, oil, engine, …).

use crate::color::{GREEN, GREEN_DIM, RED, WHITE};
use crate::font::draw_text_centered;
use crate::geom::Rect;
use crate::{Color, Surface};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct RoundGaugeOpts {
    /// 0..1 needle position.
    pub value: f32,
    /// Start of red band 0..1 (None = no redline).
    pub redline: Option<f32>,
    pub color: Color,
    pub label: &'static str,
    pub font_px: f32,
    /// Sweep radians (default 270°).
    pub sweep: f32,
    /// Angle at value=0 (screen y-down).
    pub ang0: f32,
}

impl Default for RoundGaugeOpts {
    fn default() -> Self {
        Self {
            value: 0.0,
            redline: Some(0.78),
            color: GREEN,
            label: "",
            font_px: 12.0,
            sweep: PI * 1.5,
            ang0: PI * 0.75,
        }
    }
}

/// Draw a round gauge filling `rect` (uses min dimension as diameter).
pub fn round_gauge(s: &mut Surface, rect: Rect, opts: RoundGaugeOpts) {
    let (cx, cy) = rect.center();
    let r = (rect.w.min(rect.h) / 2 - 4).max(16);
    let v = opts.value.clamp(0.0, 1.0);

    s.circle(cx, cy, r, GREEN_DIM);
    s.circle(cx, cy, r - 1, GREEN_DIM);

    // Ticks
    for k in 0..=10 {
        let t = k as f32 / 10.0;
        let a = opts.ang0 + t * opts.sweep;
        let (c, sn) = (a.cos(), a.sin());
        let outer = r as f32 - 1.0;
        let inner = r as f32 * if k % 2 == 0 { 0.82 } else { 0.90 };
        let col = if opts.redline.map(|rl| t >= rl).unwrap_or(false) {
            RED
        } else {
            GREEN
        };
        s.line_aa(
            cx + (outer * c) as i32,
            cy + (outer * sn) as i32,
            cx + (inner * c) as i32,
            cy + (inner * sn) as i32,
            col,
        );
    }

    if let Some(rl) = opts.redline {
        draw_arc(
            s,
            cx,
            cy,
            (r as f32 * 0.94) as i32,
            rl,
            1.0,
            opts.ang0,
            opts.sweep,
            RED,
        );
    }

    // Needle
    let a = opts.ang0 + v * opts.sweep;
    let (c, sn) = (a.cos(), a.sin());
    let tip = r as f32 * 0.86;
    let tail = r as f32 * 0.12;
    s.line_aa(
        cx + (-tail * c) as i32,
        cy + (-tail * sn) as i32,
        cx + (tip * c) as i32,
        cy + (tip * sn) as i32,
        WHITE,
    );
    s.circle(cx, cy, 3, WHITE);

    if !opts.label.is_empty() {
        draw_text_centered(
            s,
            cx as f32,
            cy as f32 + r as f32 * 0.35,
            opts.label,
            GREEN_DIM,
            opts.font_px,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_arc(
    s: &mut Surface,
    cx: i32,
    cy: i32,
    r: i32,
    t0: f32,
    t1: f32,
    ang0: f32,
    sweep: f32,
    color: Color,
) {
    let r = r.max(4) as f32;
    let a0 = ang0 + t0.clamp(0.0, 1.0) * sweep;
    let a1 = ang0 + t1.clamp(0.0, 1.0) * sweep;
    let segs = (((a1 - a0).abs() * r) / 1.5).ceil() as i32;
    let segs = segs.clamp(8, 256);
    let mut prev = (cx + (r * a0.cos()) as i32, cy + (r * a0.sin()) as i32);
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let a = a0 + (a1 - a0) * t;
        let cur = (cx + (r * a.cos()) as i32, cy + (r * a.sin()) as i32);
        s.line_aa(prev.0, prev.1, cur.0, cur.1, color);
        prev = cur;
    }
}
