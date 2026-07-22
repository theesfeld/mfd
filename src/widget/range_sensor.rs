//! Forward **collision / parking range** display (HSD-language arcs).
//!
//! Not true LIDAR point cloud — distance bars + polar arcs for ultrasonic /
//! radar / stereo depth channels hosts can feed.

use crate::font::draw_text;
use crate::geom::Rect;
use crate::{Color, Surface};

/// Ranges in **meters** (None = no reading).
#[derive(Clone, Copy, Debug, Default)]
pub struct RangeSnapshot {
    pub front: Option<f32>,
    pub front_left: Option<f32>,
    pub front_right: Option<f32>,
    pub rear: Option<f32>,
    pub rear_left: Option<f32>,
    pub rear_right: Option<f32>,
    /// Host max display range (m).
    pub scale_m: f32,
}

impl RangeSnapshot {
    pub fn synthetic(t: f32) -> Self {
        Self {
            front: Some(2.5 + 1.5 * (t * 0.4).sin().abs()),
            front_left: Some(3.0 + (t * 0.35).cos().abs()),
            front_right: Some(3.2 + 0.8 * (t * 0.5).sin().abs()),
            rear: Some(1.2 + 0.6 * (t * 0.3).cos().abs()),
            rear_left: Some(2.0),
            rear_right: Some(2.1),
            scale_m: 5.0,
        }
    }

    /// Optional `MFD_RANGE= f,fl,fr,r,rl,rr` meters.
    pub fn from_env_or_synthetic(t: f32) -> Self {
        if let Ok(s) = std::env::var("MFD_RANGE") {
            let parts: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
            if parts.len() >= 3 {
                return Self {
                    front: Some(parts[0]),
                    front_left: parts.get(1).copied(),
                    front_right: parts.get(2).copied(),
                    rear: parts.get(3).copied(),
                    rear_left: parts.get(4).copied(),
                    rear_right: parts.get(5).copied(),
                    scale_m: 5.0,
                };
            }
        }
        Self::synthetic(t)
    }
}

fn zone_color(m: f32, ok: Color, caution: Color, warn: Color) -> Color {
    if m < 0.8 {
        warn
    } else if m < 1.8 {
        caution
    } else {
        ok
    }
}

/// Plan-view car + range arcs + numeric bars.
#[allow(clippy::too_many_arguments)]
pub fn range_display(
    s: &mut Surface,
    rect: Rect,
    r: &RangeSnapshot,
    structure: Color,
    ok: Color,
    caution: Color,
    warn: Color,
    readout: Color,
) {
    let scale = r.scale_m.max(1.0);
    let cx = rect.center().0;
    let cy = rect.center().1 + rect.h / 10;
    let max_r = (rect.w.min(rect.h) / 2 - 12).max(20);

    // Range rings at 1m, 2m, scale
    for ring_m in [1.0_f32, 2.0, scale] {
        let rr = ((ring_m / scale) * max_r as f32) as i32;
        s.circle(cx, cy, rr, structure);
    }

    // Ownship box
    let bw = rect.w / 10;
    let bh = rect.h / 8;
    s.line_aa(cx - bw, cy - bh, cx + bw, cy - bh, readout);
    s.line_aa(cx + bw, cy - bh, cx + bw, cy + bh, readout);
    s.line_aa(cx + bw, cy + bh, cx - bw, cy + bh, readout);
    s.line_aa(cx - bw, cy + bh, cx - bw, cy - bh, readout);

    // Front arc (upper half)
    draw_sector_hits(
        s,
        cx,
        cy,
        max_r,
        scale,
        &[(-40.0, r.front_left), (0.0, r.front), (40.0, r.front_right)],
        ok,
        caution,
        warn,
    );
    // Rear
    draw_sector_hits(
        s,
        cx,
        cy,
        max_r,
        scale,
        &[(140.0, r.rear_left), (180.0, r.rear), (220.0, r.rear_right)],
        ok,
        caution,
        warn,
    );

    // Numeric strip
    let fh = 11.0_f32;
    let y0 = rect.y as f32 + 4.0;
    let line = |lab: &str, val: Option<f32>| {
        format!(
            "{lab} {}",
            val.map(|m| format!("{m:.1}m"))
                .unwrap_or_else(|| "---".into())
        )
    };
    draw_text(
        s,
        rect.x as f32 + 4.0,
        y0,
        &line("F", r.front),
        r.front
            .map(|m| zone_color(m, ok, caution, warn))
            .unwrap_or(structure),
        fh,
    );
    draw_text(
        s,
        rect.x as f32 + 4.0,
        y0 + 14.0,
        &line("FL", r.front_left),
        structure,
        fh,
    );
    draw_text(
        s,
        rect.x as f32 + 4.0,
        y0 + 28.0,
        &line("FR", r.front_right),
        structure,
        fh,
    );
    draw_text(
        s,
        rect.right() as f32 - 70.0,
        y0,
        &line("R", r.rear),
        r.rear
            .map(|m| zone_color(m, ok, caution, warn))
            .unwrap_or(structure),
        fh,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_sector_hits(
    s: &mut Surface,
    cx: i32,
    cy: i32,
    max_r: i32,
    scale: f32,
    beams: &[(f32, Option<f32>)],
    ok: Color,
    caution: Color,
    warn: Color,
) {
    for &(deg, dist) in beams {
        let Some(m) = dist else { continue };
        let frac = (m / scale).clamp(0.05, 1.0);
        let len = (max_r as f32 * frac) as i32;
        let rad = deg.to_radians();
        // 0° = up
        let x1 = cx as f32 + rad.sin() * len as f32;
        let y1 = cy as f32 - rad.cos() * len as f32;
        let col = zone_color(m, ok, caution, warn);
        s.line_aa(cx, cy, x1 as i32, y1 as i32, col);
        s.circle(x1 as i32, y1 as i32, 4, col);
    }
}
