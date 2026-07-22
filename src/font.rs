//! **B612** cockpit font (Airbus / PolarSys) rasterized with fontdue.
//!
//! Smooth anti-aliased glyphs on black glass — not 5×7 stair-steps.
//! Font files: `assets/fonts/B612Mono-Regular.ttf` (EPL-2.0; see NOTICE).

use crate::{Color, Surface};
use fontdue::Font;
use std::sync::OnceLock;

static FONT_MONO: OnceLock<Font> = OnceLock::new();

fn mono() -> &'static Font {
    FONT_MONO.get_or_init(|| {
        let bytes = include_bytes!("../assets/fonts/B612Mono-Regular.ttf");
        Font::from_bytes(bytes.as_slice(), fontdue::FontSettings::default())
            .expect("B612 Mono must load")
    })
}

/// Measure text width in pixels at `px` size.
pub fn text_width(text: &str, px: f32) -> f32 {
    let font = mono();
    let mut w = 0.0f32;
    for ch in text.chars() {
        let m = font.metrics(ch, px);
        w += m.advance_width;
    }
    w
}

pub fn text_height(px: f32) -> f32 {
    px * 1.15
}

/// Draw B612 text with coverage AA into the surface.
/// `(x, y)` is baseline-ish top-left of the line box (top of em).
pub fn draw_text(surface: &mut Surface, x: f32, y: f32, text: &str, color: Color, px: f32) {
    let font = mono();
    let size = px.max(8.0);
    let mut pen_x = x;
    let baseline = y + size * 0.85;

    for ch in text.chars() {
        if ch == ' ' {
            let m = font.metrics(' ', size);
            pen_x += m.advance_width;
            continue;
        }
        let (metrics, bitmap) = font.rasterize(ch, size);
        blit_glyph(
            surface,
            pen_x + metrics.xmin as f32,
            baseline - metrics.height as f32 - metrics.ymin as f32,
            metrics.width,
            metrics.height,
            &bitmap,
            color,
        );
        pen_x += metrics.advance_width;
    }
}

pub fn draw_text_centered(surface: &mut Surface, cx: f32, cy: f32, text: &str, color: Color, px: f32) {
    let w = text_width(text, px);
    let h = text_height(px);
    draw_text(surface, cx - w * 0.5, cy - h * 0.5, text, color, px);
}

fn blit_glyph(
    surface: &mut Surface,
    x: f32,
    y: f32,
    w: usize,
    h: usize,
    coverage: &[u8],
    color: Color,
) {
    let base_a = (color >> 24) & 0xFF;
    let r = (color >> 16) & 0xFF;
    let g = (color >> 8) & 0xFF;
    let b = color & 0xFF;
    for row in 0..h {
        for col in 0..w {
            let cov = coverage[row * w + col] as u32;
            if cov < 8 {
                continue;
            }
            let a = (base_a * cov / 255).min(255);
            let c = (a << 24) | (r << 16) | (g << 8) | b;
            let px = (x + col as f32).round() as i32;
            let py = (y + row as f32).round() as i32;
            // Simple max-alpha plot (black bg).
            if let Some(old) = surface.get(px, py) {
                let oa = (old >> 24) & 0xFF;
                if a >= oa {
                    surface.plot(px, py, c);
                }
            } else {
                surface.plot(px, py, c);
            }
        }
    }
}

// Keep stroke helpers as aliases for callers that want drawn thin labels.
pub use draw_text as draw_text_stroke;
pub use draw_text_centered as draw_text_stroke_centered;
pub use text_height as stroke_text_height;
pub use text_width as stroke_text_width;
