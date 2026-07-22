//! Schematic **line / topo-style** map (not a full DEM GIS).
//!
//! Contour ellipses + roads + ownship. Demo scrolls the world under a fixed
//! ownship chevron (heading-up).

use crate::font::draw_text;
use crate::geom::Rect;
use crate::{Color, Surface};

/// Draw abstract topo: contour rings, ridge lines, road, grid, ownship.
///
/// `heading_deg` — nose up. `speed_mph` and `t` drive scroll offset so the
/// map page has clear demo motion (world moves under the vehicle).
#[allow(clippy::too_many_arguments)]
pub fn schematic_topo_map(
    s: &mut Surface,
    rect: Rect,
    heading_deg: f32,
    speed_mph: f32,
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
    let hdg = heading_deg.to_radians();
    // Distance traveled along track (pixels). Base crawl + speed.
    let dist = t * 18.0 + speed_mph.max(0.0) * t * 0.35;
    // World scroll: opposite of travel so terrain streams under nose.
    let scroll_n = -dist * hdg.cos(); // north component in world px
    let scroll_e = -dist * hdg.sin();

    // Light grid (fixed screen — situational glass)
    for i in 1..4 {
        let x = rect.x + rect.w * i / 4;
        let y = rect.y + rect.h * i / 4;
        s.line_aa(x, rect.y + 2, x, rect.bottom() - 2, structure);
        s.line_aa(rect.x + 2, y, rect.right() - 2, y, structure);
    }

    // World → screen (heading-up): rotate world so heading is +Y up... wait nose is -Y screen up
    // Screen: +x right, +y down. Heading-up: forward = up = -Y.
    let map_w = rect.w as f32;
    let map_h = rect.h as f32;
    let to_screen = |wx: f32, wy: f32| -> (i32, i32) {
        // World offset then rotate by -heading
        let dx = wx + scroll_e;
        let dy = wy + scroll_n;
        let ch = hdg.cos();
        let sh = hdg.sin();
        // Rotate so vehicle heading points screen-up
        let rx = dx * ch + dy * sh;
        let ry = -dx * sh + dy * ch;
        let sx = cx as f32 + rx;
        let sy = cy as f32 - ry; // north/forward → up
        (sx as i32, sy as i32)
    };

    let in_pad = |x: i32, y: i32| -> bool {
        x >= rect.x - 20 && x <= rect.right() + 20 && y >= rect.y - 20 && y <= rect.bottom() + 20
    };

    // Contour “hills” as nested ellipses (two peaks), world-fixed
    let hills = [
        (-map_w * 0.25, map_h * 0.15, map_w * 0.22, map_h * 0.18),
        (map_w * 0.30, -map_h * 0.20, map_w * 0.18, map_h * 0.16),
        (map_w * 0.05, map_h * 0.45, map_w * 0.14, map_h * 0.12),
    ];
    for &(hx, hy, rx, ry) in &hills {
        for k in 1..=4 {
            let a = rx * k as f32 / 4.0;
            let b = ry * k as f32 / 4.0;
            draw_ellipse_world(s, &to_screen, hx, hy, a, b, contour, &in_pad);
        }
    }

    // Stream polyline (world)
    let stream: [(f32, f32); 7] = [
        (-map_w * 0.4, map_h * 0.35),
        (-map_w * 0.15, map_h * 0.10),
        (0.0, -map_h * 0.05),
        (map_w * 0.12, -map_h * 0.25),
        (map_w * 0.28, -map_h * 0.10),
        (map_w * 0.35, map_h * 0.20),
        (map_w * 0.40, map_h * 0.40),
    ];
    for w in stream.windows(2) {
        let (x0, y0) = to_screen(w[0].0, w[0].1);
        let (x1, y1) = to_screen(w[1].0, w[1].1);
        if in_pad(x0, y0) || in_pad(x1, y1) {
            s.line_aa(x0, y0, x1, y1, road);
        }
    }

    // Road (straighter diagonal in world)
    {
        let (x0, y0) = to_screen(-map_w * 0.45, map_h * 0.30);
        let (x1, y1) = to_screen(map_w * 0.45, -map_h * 0.25);
        s.line_thick(x0, y0, x1, y1, road, 2);
        // parallel shoulder
        let (x0b, y0b) = to_screen(-map_w * 0.45, map_h * 0.30 + 8.0);
        let (x1b, y1b) = to_screen(map_w * 0.45, -map_h * 0.25 + 8.0);
        s.line_aa(x0b, y0b, x1b, y1b, structure);
    }

    // Secondary track
    {
        let (x0, y0) = to_screen(-map_w * 0.1, -map_h * 0.4);
        let (x1, y1) = to_screen(map_w * 0.15, map_h * 0.4);
        s.line_aa(x0, y0, x1, y1, structure);
    }

    // Ownship fixed at center (heading-up chevron points up)
    let len = 16.0_f32;
    let tip_x = cx as f32;
    let tip_y = cy as f32 - len;
    let lx = cx as f32 - 8.0;
    let ly = cy as f32 + 8.0;
    let rx = cx as f32 + 8.0;
    let ry = cy as f32 + 8.0;
    s.line_thick(tip_x as i32, tip_y as i32, lx as i32, ly as i32, ownship, 2);
    s.line_thick(tip_x as i32, tip_y as i32, rx as i32, ry as i32, ownship, 2);
    s.line_aa(lx as i32, ly as i32, rx as i32, ry as i32, ownship);
    // short track history
    s.line_aa(cx, cy + 10, cx, cy + 22, ownship);

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
        "HDG-UP · DEMO SCROLL",
        ink,
        9.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_ellipse_world(
    s: &mut Surface,
    to_screen: &dyn Fn(f32, f32) -> (i32, i32),
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    color: Color,
    in_pad: &dyn Fn(i32, i32) -> bool,
) {
    if rx < 2.0 || ry < 2.0 {
        return;
    }
    let steps = 40;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let a = std::f32::consts::TAU * (i as f32) / steps as f32;
        let wx = cx + rx * a.cos();
        let wy = cy + ry * a.sin();
        let p = to_screen(wx, wy);
        if let Some(q) = prev {
            if in_pad(q.0, q.1) || in_pad(p.0, p.1) {
                s.line_aa(q.0, q.1, p.0, p.1, color);
            }
        }
        prev = Some(p);
    }
}
