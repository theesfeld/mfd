//! OSB / softkey legend row (bezel button menus).

use crate::color::{Ink, GREEN, GREEN_DIM};
use crate::font::{draw_text_centered, text_height};
use crate::geom::Rect;
use crate::Surface;

#[derive(Clone, Copy, Debug)]
pub struct SoftkeyLayout {
    /// Font size in pixels.
    pub font_px: f32,
    /// Highlight index (0-based) or None.
    pub selected: Option<usize>,
}

impl Default for SoftkeyLayout {
    fn default() -> Self {
        Self {
            font_px: 14.0,
            selected: None,
        }
    }
}

/// Draw a horizontal softkey row in `rect` (typical top or bottom chrome).
///
/// `labels` are short OSB legends (e.g. `SMS`, `HSD`, `TGP`).
pub fn softkey_row(s: &mut Surface, rect: Rect, labels: &[&str], layout: SoftkeyLayout) {
    if labels.is_empty() || rect.w <= 0 {
        return;
    }
    let n = labels.len() as i32;
    let slot = rect.w / n;
    let cy = rect.y as f32 + rect.h as f32 * 0.5;
    let fh = layout.font_px;
    for (i, lab) in labels.iter().enumerate() {
        let cx = rect.x as f32 + slot as f32 * (i as f32 + 0.5);
        let col = if layout.selected == Some(i) {
            GREEN
        } else {
            GREEN_DIM
        };
        draw_text_centered(s, cx, cy, lab, col, fh);
        // Tick under selected.
        if layout.selected == Some(i) {
            let tw = crate::font::text_width(lab, fh);
            let y = (cy + text_height(fh) * 0.45) as i32;
            s.line_aa(
                (cx - tw * 0.5) as i32,
                y,
                (cx + tw * 0.5) as i32,
                y,
                Ink::Primary.color(),
            );
        }
    }
}
