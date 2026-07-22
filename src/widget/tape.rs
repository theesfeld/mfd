//! Vertical / horizontal **tape** gauges (fuel, temp, altitude strip, …).

use crate::color::{Ink, GREEN_DIM, WHITE};
use crate::font::{draw_text, draw_text_centered};
use crate::geom::Rect;
use crate::{Color, Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapeOrientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug)]
pub struct TapeOpts {
    pub orientation: TapeOrientation,
    pub font_px: f32,
    pub color: Color,
    /// Value in 0..1.
    pub value: f32,
    pub label: &'static str,
}

impl Default for TapeOpts {
    fn default() -> Self {
        Self {
            orientation: TapeOrientation::Vertical,
            font_px: 12.0,
            color: WHITE,
            value: 0.5,
            label: "",
        }
    }
}

/// Draw a tape gauge in `rect`.
pub fn tape_gauge(s: &mut Surface, rect: Rect, opts: TapeOpts) {
    let v = opts.value.clamp(0.0, 1.0);
    let fh = opts.font_px;
    if !opts.label.is_empty() {
        draw_text(
            s,
            rect.x as f32 + 2.0,
            rect.y as f32 + 2.0,
            opts.label,
            opts.color,
            fh,
        );
    }
    let top = rect.y + (fh as i32) + 4;
    let body = Rect::new(rect.x, top, rect.w, (rect.bottom() - top).max(8));
    // Frame
    s.line_aa(body.x, body.y, body.right(), body.y, GREEN_DIM);
    s.line_aa(body.right(), body.y, body.right(), body.bottom(), GREEN_DIM);
    s.line_aa(
        body.right(),
        body.bottom(),
        body.x,
        body.bottom(),
        GREEN_DIM,
    );
    s.line_aa(body.x, body.bottom(), body.x, body.y, GREEN_DIM);

    match opts.orientation {
        TapeOrientation::Vertical => {
            let mid = body.x + body.w / 2;
            let n = 11;
            for i in 0..n {
                let t = i as f32 / (n - 1) as f32;
                let yy = body.bottom() - ((body.h as f32) * t) as i32;
                let half = if i % 5 == 0 { body.w / 5 } else { body.w / 10 };
                s.line_aa(mid - half, yy, mid + half, yy, GREEN_DIM);
            }
            let fill = ((body.h as f32) * v) as i32;
            if fill > 0 {
                s.line_aa(mid, body.bottom(), mid, body.bottom() - fill, opts.color);
                s.line_aa(
                    mid + 1,
                    body.bottom(),
                    mid + 1,
                    body.bottom() - fill,
                    opts.color,
                );
            }
            let iy = body.bottom() - fill;
            let arm = (body.w / 3).max(4);
            s.line_aa(mid - arm, iy, mid + arm, iy, opts.color);
        }
        TapeOrientation::Horizontal => {
            let mid = body.y + body.h / 2;
            let n = 11;
            for i in 0..n {
                let t = i as f32 / (n - 1) as f32;
                let xx = body.x + ((body.w as f32) * t) as i32;
                let half = if i % 5 == 0 { body.h / 5 } else { body.h / 10 };
                s.line_aa(xx, mid - half, xx, mid + half, GREEN_DIM);
            }
            let fill = ((body.w as f32) * v) as i32;
            if fill > 0 {
                s.line_aa(body.x, mid, body.x + fill, mid, opts.color);
            }
            let ix = body.x + fill;
            let arm = (body.h / 3).max(4);
            s.line_aa(ix, mid - arm, ix, mid + arm, opts.color);
        }
    }

    let pct = format!("{}", (v * 100.0).round() as i32);
    draw_text_centered(
        s,
        body.center().0 as f32,
        body.y as f32 + fh * 0.6,
        &pct,
        Ink::Readout.color(),
        fh * 0.85,
    );
}
