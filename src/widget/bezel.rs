//! Outer bezel / panel frame.

use crate::color::GREEN_DIM;
use crate::geom::Rect;
use crate::Surface;

/// 1px outer frame for a page or content window.
pub fn bezel_frame(s: &mut Surface, rect: Rect) {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.right() - 1;
    let y1 = rect.bottom() - 1;
    s.line_aa(x0, y0, x1, y0, GREEN_DIM);
    s.line_aa(x1, y0, x1, y1, GREEN_DIM);
    s.line_aa(x1, y1, x0, y1, GREEN_DIM);
    s.line_aa(x0, y1, x0, y0, GREEN_DIM);
}
