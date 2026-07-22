//! Text labels (B612).

use crate::font::{draw_text, draw_text_centered};
use crate::{Color, Surface};

pub fn label(s: &mut Surface, x: f32, y: f32, text: &str, color: Color, px: f32) {
    draw_text(s, x, y, text, color, px);
}

pub fn label_centered(s: &mut Surface, cx: f32, cy: f32, text: &str, color: Color, px: f32) {
    draw_text_centered(s, cx, cy, text, color, px);
}
