//! Baked **B612 Mono** bitmap atlas (no runtime TTF).
//!
//! Glyph coverage is compiled into [`font_atlas_data`]. Draw with `plot`.
//! Regenerate: `cargo run --release --bin bake-font-atlas --features bake_font`

#[path = "font_atlas_data.rs"]
mod font_atlas_data;

use crate::{Color, Surface};
use font_atlas_data::{Face, Glyph, COVERAGE, FACES, FIRST_CHAR, GLYPHS, LAST_CHAR, N_CHARS};

/// Discrete face sizes baked into the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FontSize {
    /// ~12 px (dense softkeys).
    Sm = 0,
    /// ~16 px (default body).
    Md = 1,
    /// ~20 px (titles).
    Lg = 2,
}

impl FontSize {
    pub fn as_index(self) -> usize {
        self as usize
    }

    pub fn px(self) -> u16 {
        FACES[self.as_index()].size
    }

    /// Map a floating pixel request to the nearest baked face.
    pub fn nearest(px: f32) -> Self {
        let p = px.max(1.0);
        let mut best = FontSize::Md;
        let mut best_d = f32::MAX;
        for sz in [FontSize::Sm, FontSize::Md, FontSize::Lg] {
            let d = (p - sz.px() as f32).abs();
            if d < best_d {
                best_d = d;
                best = sz;
            }
        }
        best
    }
}

fn face(sz: FontSize) -> Face {
    FACES[sz.as_index()]
}

fn glyph(sz: FontSize, ch: char) -> Option<Glyph> {
    let c = ch as u32;
    if ch == '\0' || c > 255 {
        return None;
    }
    let b = c as u8;
    if !(FIRST_CHAR..=LAST_CHAR).contains(&b) {
        return None;
    }
    let idx = sz.as_index() * N_CHARS + (b - FIRST_CHAR) as usize;
    GLYPHS.get(idx).copied()
}

/// Pixel width of `text` at baked size.
pub fn text_width_size(text: &str, sz: FontSize) -> f32 {
    let mut w = 0u32;
    for ch in text.chars() {
        if let Some(g) = glyph(sz, ch) {
            w += g.advance as u32;
        } else if ch == ' ' {
            w += face(sz).size as u32 / 2;
        }
    }
    w as f32
}

pub fn text_height_size(sz: FontSize) -> f32 {
    face(sz).line_height as f32
}

/// Compatibility: width for approximate `px` (nearest face).
pub fn text_width(text: &str, px: f32) -> f32 {
    text_width_size(text, FontSize::nearest(px))
}

pub fn text_height(px: f32) -> f32 {
    text_height_size(FontSize::nearest(px))
}

/// Draw text with a discrete baked size. `(x, y)` = top-left of line box.
pub fn draw_text_size(
    surface: &mut Surface,
    x: f32,
    y: f32,
    text: &str,
    color: Color,
    sz: FontSize,
) {
    let f = face(sz);
    let baseline = y + f.ascent as f32;
    let mut pen_x = x;
    for ch in text.chars() {
        let Some(g) = glyph(sz, ch) else {
            if ch == ' ' {
                pen_x += (f.size as f32) * 0.5;
            }
            continue;
        };
        if g.w > 0 && g.h > 0 && g.len > 0 {
            let gx = pen_x + g.xmin as f32;
            let gy = baseline - g.ymin as f32 - g.h as f32;
            let start = g.off as usize;
            let end = start + g.len as usize;
            if end <= COVERAGE.len() {
                blit_glyph(
                    surface,
                    gx,
                    gy,
                    g.w as usize,
                    g.h as usize,
                    &COVERAGE[start..end],
                    color,
                );
            }
        }
        pen_x += g.advance as f32;
    }
}

pub fn draw_text_size_centered(
    surface: &mut Surface,
    cx: f32,
    cy: f32,
    text: &str,
    color: Color,
    sz: FontSize,
) {
    let w = text_width_size(text, sz);
    let h = text_height_size(sz);
    draw_text_size(surface, cx - w * 0.5, cy - h * 0.5, text, color, sz);
}

/// Draw text; `px` selects nearest baked face (12 / 16 / 20).
pub fn draw_text(surface: &mut Surface, x: f32, y: f32, text: &str, color: Color, px: f32) {
    draw_text_size(surface, x, y, text, color, FontSize::nearest(px));
}

pub fn draw_text_centered(
    surface: &mut Surface,
    cx: f32,
    cy: f32,
    text: &str,
    color: Color,
    px: f32,
) {
    draw_text_size_centered(surface, cx, cy, text, color, FontSize::nearest(px));
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
    if coverage.len() < w.saturating_mul(h) {
        return;
    }
    let sr = (color >> 16) & 0xFF;
    let sg = (color >> 8) & 0xFF;
    let sb = color & 0xFF;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;

    for row in 0..h {
        for col in 0..w {
            let cov = coverage[row * w + col] as u32;
            if cov < 12 {
                continue;
            }
            let px = x0 + col as i32;
            let py = y0 + row as i32;
            if px < 0 || py < 0 {
                continue;
            }
            let r = sr * cov / 255;
            let g = sg * cov / 255;
            let b = sb * cov / 255;
            let (nr, ng, nb) = if let Some(old) = surface.get(px, py) {
                let or_ = (old >> 16) & 0xFF;
                let og = (old >> 8) & 0xFF;
                let ob = old & 0xFF;
                (r.max(or_), g.max(og), b.max(ob))
            } else {
                (r, g, b)
            };
            surface.plot(px, py, 0xFF00_0000 | (nr << 16) | (ng << 8) | nb);
        }
    }
}

// Aliases
pub use draw_text as draw_text_stroke;
pub use draw_text_centered as draw_text_stroke_centered;
pub use text_height as stroke_text_height;
pub use text_width as stroke_text_width;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::GREEN;

    #[test]
    fn atlas_has_three_faces() {
        assert_eq!(FACES.len(), 3);
        assert_eq!(GLYPHS.len(), 3 * N_CHARS);
        assert!(!COVERAGE.is_empty());
    }

    #[test]
    fn text_over_black_is_green() {
        let mut s = Surface::new(320, 48);
        s.clear_black();
        draw_text_size(&mut s, 4.0, 8.0, "BLANK HSD TEST", GREEN, FontSize::Md);
        let mut greenish = 0u32;
        for y in 0..48 {
            for x in 0..320 {
                if let Some(c) = s.get(x, y) {
                    let g = (c >> 8) & 0xFF;
                    let r = (c >> 16) & 0xFF;
                    let a = (c >> 24) & 0xFF;
                    if a == 255 && g > 40 && g > r {
                        greenish += 1;
                    }
                }
            }
        }
        assert!(greenish > 200, "expected green text pixels, got {greenish}");
    }

    #[test]
    fn nearest_picks_faces() {
        assert_eq!(FontSize::nearest(11.0), FontSize::Sm);
        assert_eq!(FontSize::nearest(16.0), FontSize::Md);
        assert_eq!(FontSize::nearest(22.0), FontSize::Lg);
    }
}
