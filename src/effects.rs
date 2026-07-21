//! Cheap post-process effects on a pixel surface.
//!
//! These run in system RAM after geometry draw. They are optional.
//! Prefer the assembly line path for motion; use effects for style.

use crate::{rgb, Color, Surface};

/// Soft glow: expand bright pixels by `radius` with falloff.
/// Cost grows with radius × non-black pixels.
pub fn glow(surface: &mut Surface, radius: i32, strength: u8) {
    if radius <= 0 {
        return;
    }
    let w = surface.width() as i32;
    let h = surface.height() as i32;
    let src = surface.pixels().to_vec();
    let stride = surface.stride() as usize;

    for y in 0..h {
        for x in 0..w {
            let base = load(&src, stride, x, y, w, h);
            if base == 0 {
                continue;
            }
            let (br, bg, bb) = unpack(base);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let dist2 = dx * dx + dy * dy;
                    if dist2 > radius * radius {
                        continue;
                    }
                    let fall = 1.0 - (dist2 as f32).sqrt() / (radius as f32 + 0.001);
                    let k = (fall * strength as f32 / 255.0).clamp(0.0, 1.0);
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let cur = surface.get(nx, ny).unwrap_or(0);
                    let (cr, cg, cb) = unpack(cur);
                    let nr = cr.saturating_add((br as f32 * k) as u8);
                    let ng = cg.saturating_add((bg as f32 * k) as u8);
                    let nb = cb.saturating_add((bb as f32 * k) as u8);
                    surface.plot(nx, ny, rgb(nr, ng, nb));
                }
            }
        }
    }
}

/// Cheap “bloom”: box-blur a threshold mask, then add back.
pub fn bloom(surface: &mut Surface, threshold: u8, passes: u32) {
    let w = surface.width() as usize;
    let h = surface.height() as usize;
    if w < 3 || h < 3 {
        return;
    }
    let mut bright = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let c = surface.get(x as i32, y as i32).unwrap_or(0);
            let (r, g, b) = unpack(c);
            let lum = (r as u32 + g as u32 + b as u32) / 3;
            let i = (y * w + x) * 3;
            if lum >= threshold as u32 {
                bright[i] = r;
                bright[i + 1] = g;
                bright[i + 2] = b;
            }
        }
    }
    let mut tmp = bright.clone();
    let n = passes.clamp(1, 4);
    for _ in 0..n {
        box_blur_rgb(&bright, &mut tmp, w, h);
        std::mem::swap(&mut bright, &mut tmp);
    }
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 3;
            let add_r = bright[i] / 2;
            let add_g = bright[i + 1] / 2;
            let add_b = bright[i + 2] / 2;
            if add_r | add_g | add_b == 0 {
                continue;
            }
            let c = surface.get(x as i32, y as i32).unwrap_or(0);
            let (r, g, b) = unpack(c);
            surface.plot(
                x as i32,
                y as i32,
                rgb(
                    r.saturating_add(add_r),
                    g.saturating_add(add_g),
                    b.saturating_add(add_b),
                ),
            );
        }
    }
}

/// Radar-style angular fade: darken pixels by angle distance from `beam_rad`.
/// `width_rad` is the bright sector half-angle.
pub fn radar_fade(surface: &mut Surface, cx: i32, cy: i32, beam_rad: f32, width_rad: f32) {
    let w = surface.width() as i32;
    let h = surface.height() as i32;
    let width = width_rad.max(0.05);
    for y in 0..h {
        for x in 0..w {
            let c = surface.get(x, y).unwrap_or(0);
            if c == 0 {
                continue;
            }
            let ang = ((y - cy) as f32).atan2((x - cx) as f32);
            let mut d = (ang - beam_rad).abs();
            while d > std::f32::consts::PI {
                d -= 2.0 * std::f32::consts::PI;
            }
            d = d.abs();
            let k = (1.0 - (d / width).min(1.0)).clamp(0.05, 1.0);
            let (r, g, b) = unpack(c);
            surface.plot(
                x,
                y,
                rgb(
                    (r as f32 * k) as u8,
                    (g as f32 * k) as u8,
                    (b as f32 * k) as u8,
                ),
            );
        }
    }
}

/// Scanline dim (CRT aesthetic). Even rows × `even_k`/255.
pub fn scanlines(surface: &mut Surface, even_k: u8) {
    let w = surface.width() as i32;
    let h = surface.height() as i32;
    let k = even_k as f32 / 255.0;
    for y in (0..h).step_by(2) {
        for x in 0..w {
            let c = surface.get(x, y).unwrap_or(0);
            if c == 0 {
                continue;
            }
            let (r, g, b) = unpack(c);
            surface.plot(
                x,
                y,
                rgb(
                    (r as f32 * k) as u8,
                    (g as f32 * k) as u8,
                    (b as f32 * k) as u8,
                ),
            );
        }
    }
}

fn box_blur_rgb(src: &[u8], dst: &mut [u8], w: usize, h: usize) {
    for y in 0..h {
        for x in 0..w {
            let mut sr = 0u32;
            let mut sg = 0u32;
            let mut sb = 0u32;
            let mut n = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let i = (ny as usize * w + nx as usize) * 3;
                    sr += src[i] as u32;
                    sg += src[i + 1] as u32;
                    sb += src[i + 2] as u32;
                    n += 1;
                }
            }
            let i = (y * w + x) * 3;
            dst[i] = (sr / n) as u8;
            dst[i + 1] = (sg / n) as u8;
            dst[i + 2] = (sb / n) as u8;
        }
    }
}

#[inline]
fn unpack(c: Color) -> (u8, u8, u8) {
    (
        ((c >> 16) & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        (c & 0xFF) as u8,
    )
}

#[inline]
fn load(px: &[u8], stride: usize, x: i32, y: i32, w: i32, h: i32) -> Color {
    if x < 0 || y < 0 || x >= w || y >= h {
        return 0;
    }
    let i = y as usize * stride + x as usize * 4;
    if i + 3 >= px.len() {
        return 0;
    }
    u32::from_le_bytes([px[i], px[i + 1], px[i + 2], px[i + 3]])
}
