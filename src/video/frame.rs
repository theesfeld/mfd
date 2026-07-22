//! Greyscale frames + FLIR blit.

use crate::geom::Rect;
use crate::widget::video_frame;
use crate::{Color, Surface};
use std::path::Path;

/// Greyscale frame in memory (0 = cold/black, 255 = hot/white for FLIR-style).
#[derive(Clone, Debug, Default)]
pub struct GreyFrame {
    pub w: u32,
    pub h: u32,
    pub pixels: Vec<u8>,
}

impl GreyFrame {
    pub fn synthetic(w: u32, h: u32, t: f32) -> Self {
        let mut pixels = vec![0u8; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let nx = x as f32 / w as f32;
                let ny = y as f32 / h as f32;
                // Horizon band + road wedge + noise (reads as crude FLIR).
                let sky = (40.0 + 20.0 * (nx * 3.0 + t * 0.2).sin()) as u8;
                let ground = (90.0 + 40.0 * ny + 15.0 * (nx * 8.0 + t).sin()) as u8;
                let road = if (nx - 0.5).abs() < 0.12 + 0.08 * ny {
                    160u8
                } else {
                    0
                };
                let base = if ny < 0.42 { sky } else { ground.max(road) };
                let n =
                    ((x.wrapping_mul(17) ^ y.wrapping_mul(31).wrapping_add(t as u32)) % 40) as u8;
                let i = (y * w + x) as usize;
                pixels[i] = base.saturating_add(n / 2);
            }
        }
        // Hot blob (engine / person stand-in)
        let cx = (w as f32 * (0.55 + 0.08 * t.sin())) as i32;
        let cy = (h as f32 * 0.55) as i32;
        for dy in -12..=12 {
            for dx in -8..=8 {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                    let d = (dx * dx + dy * dy) as f32;
                    if d < 100.0 {
                        let i = (py as u32 * w + px as u32) as usize;
                        pixels[i] = 220;
                    }
                }
            }
        }
        Self { w, h, pixels }
    }

    /// Load binary PGM (P5). Returns None on error.
    pub fn load_pgm(path: &Path) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        // Skip magic
        if data.len() < 3 || &data[0..2] != b"P5" {
            return None;
        }
        let mut i = 2usize;
        fn skip_ws_comments(data: &[u8], mut i: usize) -> usize {
            loop {
                while i < data.len() && data[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < data.len() && data[i] == b'#' {
                    while i < data.len() && data[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                break;
            }
            i
        }
        fn read_u32(data: &[u8], i: &mut usize) -> Option<u32> {
            *i = skip_ws_comments(data, *i);
            let start = *i;
            while *i < data.len() && data[*i].is_ascii_digit() {
                *i += 1;
            }
            if start == *i {
                return None;
            }
            std::str::from_utf8(&data[start..*i]).ok()?.parse().ok()
        }
        let w = read_u32(&data, &mut i)?;
        let h = read_u32(&data, &mut i)?;
        let maxv = read_u32(&data, &mut i)?;
        i = skip_ws_comments(&data, i);
        // After maxval, single whitespace then binary
        if i < data.len() && data[i].is_ascii_whitespace() {
            i += 1;
        }
        let need = (w * h) as usize;
        if data.len() < i + need || maxv == 0 {
            return None;
        }
        let mut pixels = data[i..i + need].to_vec();
        if maxv != 255 {
            for p in &mut pixels {
                *p = ((*p as u32 * 255) / maxv) as u8;
            }
        }
        Some(Self { w, h, pixels })
    }

    /// Resolve feed: env path or synthetic.
    pub fn resolve(t: f32, w: u32, h: u32) -> Self {
        if let Ok(p) = std::env::var("MFD_FLIR_PATH") {
            if let Some(f) = Self::load_pgm(Path::new(&p)) {
                return f;
            }
        }
        Self::synthetic(w.max(80), h.max(60), t)
    }
}

/// Draw greyscale frame into rect (nearest-neighbor), FLIR-style green or white-hot.
pub fn blit_grey_flir(
    s: &mut Surface,
    rect: Rect,
    frame: &GreyFrame,
    hot: Color,
    structure: Color,
) {
    video_frame(s, rect, structure);
    if frame.w == 0 || frame.h == 0 || frame.pixels.is_empty() {
        return;
    }
    let rw = rect.w.max(1) as u32;
    let rh = rect.h.max(1) as u32;
    let hr = (hot >> 16) & 0xff;
    let hg = (hot >> 8) & 0xff;
    let hb = hot & 0xff;
    for dy in 0..rh {
        for dx in 0..rw {
            let sx = dx * frame.w / rw;
            let sy = dy * frame.h / rh;
            let g = frame.pixels[(sy * frame.w + sx) as usize] as u32;
            // Tint greyscale toward `hot` color (green-hot FLIR).
            let r = (hr * g / 255) as u8;
            let gg = (hg * g / 255) as u8;
            let b = (hb * g / 255) as u8;
            let c = 0xFF00_0000 | ((r as u32) << 16) | ((gg as u32) << 8) | (b as u32);
            s.plot(rect.x + dx as i32, rect.y + dy as i32, c);
        }
    }
}
