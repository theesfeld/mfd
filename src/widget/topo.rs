//! Schematic **line / topo-style** map (not a full DEM GIS).
//!
//! Contour ellipses + roads + ownship. Good enough for MFD-style situational
//! glass without terrain databases.

use crate::font::draw_text;
use crate::geom::Rect;
use crate::{Color, Surface};

/// Draw abstract topo: contour rings, ridge lines, road, grid, ownship.
#[allow(clippy::too_many_arguments)]
pub fn schematic_topo_map(
    s: &mut Surface,
    rect: Rect,
    heading_deg: f32,
    t: f32,
    structure: Color,
    contour: Color,
    road: Color,
    ownship: Color,
    ink: Color,
) {
    // Frame
    s.line_aa(rect.x, rect.y, rect.right(), rect.y, structure);
    s.line_aa(rect.right(), rect.y, rect.right(), rect.bottom(), structure);
    s.line_aa(
        rect.right(),
        rect.bottom(),
        rect.x,
        rect.bottom(),
        structure,
    );
    s.line_aa(rect.x, rect.bottom(), rect.x, rect.y, structure);

    let cx = rect.center().0;
    let cy = rect.center().1;

    // Light grid
    for i in 1..4 {
        let x = rect.x + rect.w * i / 4;
        let y = rect.y + rect.h * i / 4;
        s.line_aa(x, rect.y + 2, x, rect.bottom() - 2, structure);
        s.line_aa(rect.x + 2, y, rect.right() - 2, y, structure);
    }

    // Contour “hills” as nested ellipses (two peaks)
    let hills = [
        (cx - rect.w / 5, cy - rect.h / 6, rect.w / 5, rect.h / 6),
        (cx + rect.w / 4, cy + rect.h / 8, rect.w / 6, rect.h / 7),
    ];
    for &(hx, hy, rx, ry) in &hills {
        for k in 1..=4 {
            let a = rx * k / 4;
            let b = ry * k / 4;
            draw_ellipse(s, hx, hy, a, b, contour);
        }
    }

    // Ridge / stream polylines
    let stream: [(i32, i32); 6] = [
        (rect.x + 8, rect.y + rect.h / 3),
        (cx - rect.w / 6, cy - rect.h / 8),
        (cx, cy + 4),
        (cx + rect.w / 8, cy + rect.h / 5),
        (rect.right() - 12, rect.bottom() - rect.h / 4),
        (rect.right() - 8, rect.bottom() - 8),
    ];
    for w in stream.windows(2) {
        s.line_aa(w[0].0, w[0].1, w[1].0, w[1].1, road);
    }

    // Road (straighter)
    s.line_aa(
        rect.x + 4,
        rect.bottom() - rect.h / 4,
        rect.right() - 4,
        rect.y + rect.h / 3,
        road,
    );
    s.line_aa(
        rect.x + 4,
        rect.bottom() - rect.h / 4 + 2,
        rect.right() - 4,
        rect.y + rect.h / 3 + 2,
        road,
    );

    // Ownship (heading-up chevron)
    let rad = heading_deg.to_radians();
    let len = 14.0_f32;
    let nx = rad.sin();
    let ny = -rad.cos();
    let tip_x = cx as f32 + nx * len;
    let tip_y = cy as f32 + ny * len + 3.0 * t.sin(); // slight motion
    let lx = cx as f32 - nx * 8.0 - ny * 7.0;
    let ly = cy as f32 - ny * 8.0 + nx * 7.0;
    let rx = cx as f32 - nx * 8.0 + ny * 7.0;
    let ry = cy as f32 - ny * 8.0 - nx * 7.0;
    s.line_aa(tip_x as i32, tip_y as i32, lx as i32, ly as i32, ownship);
    s.line_aa(tip_x as i32, tip_y as i32, rx as i32, ry as i32, ownship);
    s.line_aa(lx as i32, ly as i32, rx as i32, ry as i32, ownship);

    draw_text(
        s,
        rect.x as f32 + 4.0,
        rect.y as f32 + 4.0,
        "SCHEMATIC · NOT DEM",
        structure,
        9.0,
    );
    draw_text(
        s,
        rect.x as f32 + 4.0,
        rect.bottom() as f32 - 12.0,
        "LINE/TOPO STYLE",
        ink,
        9.0,
    );
}

fn draw_ellipse(s: &mut Surface, cx: i32, cy: i32, rx: i32, ry: i32, color: Color) {
    if rx < 2 || ry < 2 {
        return;
    }
    let steps = 48;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let a = std::f32::consts::TAU * (i as f32) / steps as f32;
        let x = cx as f32 + rx as f32 * a.cos();
        let y = cy as f32 + ry as f32 * a.sin();
        let p = (x as i32, y as i32);
        if let Some(q) = prev {
            s.line_aa(q.0, q.1, p.0, p.1, color);
        }
        prev = Some(p);
    }
}
