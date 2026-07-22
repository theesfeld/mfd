//! **B612** cockpit font (Airbus / PolarSys) rasterized with fontdue.
//!
//! Coverage is composited onto **opaque black glass**: RGB is scaled by
//! coverage, alpha is always opaque so present paths keep the ink.
//!
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
    let font = mono();
    if let Some(m) = font.horizontal_line_metrics(px.max(8.0)) {
        return m.ascent - m.descent + m.line_gap;
    }
    px * 1.25
}

/// Draw B612 text. `(x, y)` is the **top-left of the line box** (not baseline).
pub fn draw_text(surface: &mut Surface, x: f32, y: f32, text: &str, color: Color, px: f32) {
    let font = mono();
    let size = px.max(8.0);
    let ascent = font
        .horizontal_line_metrics(size)
        .map(|m| m.ascent)
        .unwrap_or(size * 0.8);
    // Baseline in y-down surface coords.
    let baseline = y + ascent;
    let mut pen_x = x;

    for ch in text.chars() {
        if ch == ' ' {
            pen_x += font.metrics(' ', size).advance_width;
            continue;
        }
        let (metrics, bitmap) = font.rasterize(ch, size);
        if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
            pen_x += metrics.advance_width;
            continue;
        }
        // fontdue: ymin = bottom of bitmap in y-up coords relative to baseline.
        // Screen y-down: top of bitmap = baseline - ymin - height.
        let gx = pen_x + metrics.xmin as f32;
        let gy = baseline - metrics.ymin as f32 - metrics.height as f32;
        blit_glyph(
            surface,
            gx,
            gy,
            metrics.width,
            metrics.height,
            &bitmap,
            color,
        );
        pen_x += metrics.advance_width;
    }
}

pub fn draw_text_centered(
    surface: &mut Surface,
    cx: f32,
    cy: f32,
    text: &str,
    color: Color,
    px: f32,
) {
    let w = text_width(text, px);
    let h = text_height(px);
    draw_text(surface, cx - w * 0.5, cy - h * 0.5, text, color, px);
}

/// Composite coverage onto glass: RGB *= cov/255, alpha = opaque.
///
/// Must **always** write over opaque black. Comparing glyph alpha to the
/// existing pixel alpha (255) would skip almost all AA samples.
fn blit_glyph(
    surface: &mut Surface,
    x: f32,
    y: f32,
    w: usize,
    h: usize,
    coverage: &[u8],
    color: Color,
) {
    debug_assert_eq!(coverage.len(), w.saturating_mul(h));
    let sr = (color >> 16) & 0xFF;
    let sg = (color >> 8) & 0xFF;
    let sb = color & 0xFF;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;

    for row in 0..h {
        for col in 0..w {
            let cov = coverage[row * w + col] as u32;
            // Drop dust; keep soft edges.
            if cov < 12 {
                continue;
            }
            let px = x0 + col as i32;
            let py = y0 + row as i32;
            if px < 0 || py < 0 {
                continue;
            }

            // Scale ink toward black by coverage (glass is black).
            let r = sr * cov / 255;
            let g = sg * cov / 255;
            let b = sb * cov / 255;

            // Max with existing RGB so overlapping AA edges stay bright.
            let (nr, ng, nb) = if let Some(old) = surface.get(px, py) {
                let or_ = (old >> 16) & 0xFF;
                let og = (old >> 8) & 0xFF;
                let ob = old & 0xFF;
                (r.max(or_), g.max(og), b.max(ob))
            } else {
                (r, g, b)
            };
            let out = 0xFF00_0000 | (nr << 16) | (ng << 8) | nb;
            surface.plot(px, py, out);
        }
    }
}

// Aliases used by older call sites.
pub use draw_text as draw_text_stroke;
pub use draw_text_centered as draw_text_stroke_centered;
pub use text_height as stroke_text_height;
pub use text_width as stroke_text_width;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::GREEN;

    #[test]
    fn text_over_black_is_green() {
        let mut s = Surface::new(240, 48);
        s.clear_black();
        draw_text(&mut s, 4.0, 8.0, "BLANK HSD TEST", GREEN, 18.0);
        let mut greenish = 0u32;
        for y in 0..48 {
            for x in 0..240 {
                if let Some(c) = s.get(x, y) {
                    let g = (c >> 8) & 0xFF;
                    let r = (c >> 16) & 0xFF;
                    let a = (c >> 24) & 0xFF;
                    // Ink: green channel dominates, fully opaque.
                    if a == 255 && g > 40 && g > r {
                        greenish += 1;
                    }
                }
            }
        }
        assert!(
            greenish > 200,
            "expected many green text pixels over black, got {greenish}"
        );
    }

    #[test]
    fn softkey_words_not_empty_bitmaps() {
        let font = mono();
        for word in ["BLANK", "HAD", "SMS", "HSD", "DTE", "TEST"] {
            for ch in word.chars() {
                let (m, bm) = font.rasterize(ch, 16.0);
                let nz = bm.iter().filter(|&&c| c > 20).count();
                assert!(nz > 10, "{ch} in {word} has too few cover samples ({nz})");
                assert!(m.width > 0 && m.height > 0);
            }
        }
    }
}
