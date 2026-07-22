//! VGE — true vector graphics engine.
//!
//! Geometry (`line`, `circle`, transform) lights **individual pixels**.
//! This is not a bitmap/sprite display path.
//!
//! Hot path on `x86_64`: GNU assembly (`asm/x86_64/vge.s`).
//! Portable C fallback on other targets.
//!
//! C header: `include/vge.h`.

#![allow(non_camel_case_types)]

pub mod effects;
#[cfg(target_os = "linux")]
pub mod fb;
pub mod frame;
pub mod stroke;
pub mod term;

pub use stroke::{DisplayList, Stroke};

use std::f32::consts::PI;

pub const VERSION: &str = "0.1.0-dev.1";

/// Packed color `0xAARRGGBB` (alpha in the high byte).
///
/// Opaque strokes use `AA = 0xFF`. Fully transparent is `0` (no beam).
/// The terminal present path **skips** transparent pixels so the host
/// background (your TTY text) shows through.
pub type Color = u32;

#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[inline]
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[inline]
pub const fn alpha(c: Color) -> u8 {
    ((c >> 24) & 0xFF) as u8
}

/// Fully transparent — no pixel drawn; terminal content shows through.
pub const TRANSPARENT: Color = 0;
pub const BLACK: Color = rgb(0, 0, 0);
pub const GREEN: Color = rgb(0, 255, 70);
pub const GREEN_DIM: Color = rgb(0, 120, 45);
pub const AMBER: Color = rgb(255, 200, 40);
pub const RED: Color = rgb(255, 40, 40);
pub const CYAN: Color = rgb(40, 220, 255);
pub const WHITE: Color = rgb(220, 255, 230);

/// FFI surface layout — must match `VgeSurface` in `vge.h`.
#[repr(C)]
pub struct VgeSurface {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub _pad: u32,
    pub pixels: *mut u8,
}

/// Affine 2×3 transform — must match `VgeXform`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VgeXform {
    pub a: f32,
    pub b: f32,
    pub tx: f32,
    pub c: f32,
    pub d: f32,
    pub ty: f32,
}

extern "C" {
    pub fn vge_clear(s: *mut VgeSurface, color: u32);
    pub fn vge_plot(s: *mut VgeSurface, x: i32, y: i32, color: u32);
    pub fn vge_line(s: *mut VgeSurface, x0: i32, y0: i32, x1: i32, y1: i32, color: u32);
    pub fn vge_line_thick(
        s: *mut VgeSurface,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: u32,
        thickness: i32,
    );
    pub fn vge_circle(s: *mut VgeSurface, cx: i32, cy: i32, r: i32, color: u32);
    pub fn vge_rect_fill(s: *mut VgeSurface, x0: i32, y0: i32, x1: i32, y1: i32, color: u32);
    pub fn vge_xform_identity(m: *mut VgeXform);
    pub fn vge_xform_translate(m: *mut VgeXform, tx: f32, ty: f32);
    pub fn vge_xform_scale(m: *mut VgeXform, sx: f32, sy: f32);
    pub fn vge_xform_rotate(m: *mut VgeXform, radians: f32);
    pub fn vge_xform_apply(m: *const VgeXform, x: f32, y: f32, ox: *mut f32, oy: *mut f32);
    pub fn vge_line_xf(
        s: *mut VgeSurface,
        m: *const VgeXform,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        color: u32,
    );
    pub fn vge_polyline(s: *mut VgeSurface, xy: *const i32, n: i32, color: u32);
    pub fn vge_export_rgb24(s: *const VgeSurface, dest: *mut u8);
    pub fn vge_blit(dst: *mut VgeSurface, src: *const VgeSurface);
    pub fn vge_decay(s: *mut VgeSurface, factor_256: u32);
    pub fn vge_version() -> *const std::os::raw::c_char;
}

/// Owned pixel surface (XRGB8888, 4 bytes per pixel).
pub struct Surface {
    width: u32,
    height: u32,
    stride: u32,
    pixels: Vec<u8>,
}

impl Surface {
    /// Create a black surface. Width and height must be > 0.
    pub fn new(width: u32, height: u32) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let stride = w.saturating_mul(4);
        let len = (stride as usize).saturating_mul(h as usize);
        Self {
            width: w,
            height: h,
            stride,
            pixels: vec![0u8; len.max(4)],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stride(&self) -> u32 {
        self.stride
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    #[inline]
    fn as_ffi(&mut self) -> VgeSurface {
        VgeSurface {
            width: self.width,
            height: self.height,
            stride: self.stride,
            _pad: 0,
            pixels: self.pixels.as_mut_ptr(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        let mut s = self.as_ffi();
        unsafe { vge_clear(&mut s, color) };
    }

    /// Phosphor fade: channels *= factor_256/256. Prefer this over hard clear
    /// for smooth vector trails (classic CRT-style persistence).
    pub fn decay(&mut self, factor_256: u32) {
        let mut s = self.as_ffi();
        unsafe { vge_decay(&mut s, factor_256) };
    }

    /// Copy this surface into `dst` (min size). Double-buffer present path.
    pub fn blit_to(&self, dst: &mut Surface) {
        let src = VgeSurface {
            width: self.width,
            height: self.height,
            stride: self.stride,
            _pad: 0,
            pixels: self.pixels.as_ptr() as *mut u8,
        };
        let mut d = dst.as_ffi();
        unsafe { vge_blit(&mut d, &src) };
    }

    /// Copy into a raw VgeSurface (e.g. mmap'd frame buffer).
    pub fn blit_to_raw(&self, dst: &mut VgeSurface) {
        let src = VgeSurface {
            width: self.width,
            height: self.height,
            stride: self.stride,
            _pad: 0,
            pixels: self.pixels.as_ptr() as *mut u8,
        };
        unsafe { vge_blit(dst, &src) };
    }

    pub fn plot(&mut self, x: i32, y: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { vge_plot(&mut s, x, y, color) };
    }

    /// Light every pixel on the line from (x0,y0) to (x1,y1).
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { vge_line(&mut s, x0, y0, x1, y1, color) };
    }

    pub fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color, t: i32) {
        let mut s = self.as_ffi();
        unsafe { vge_line_thick(&mut s, x0, y0, x1, y1, color, t) };
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { vge_circle(&mut s, cx, cy, r, color) };
    }

    pub fn rect_fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { vge_rect_fill(&mut s, x0, y0, x1, y1, color) };
    }

    /// Line in world space through transform `m`.
    pub fn line_xf(&mut self, m: &Xform, x0: f32, y0: f32, x1: f32, y1: f32, color: Color) {
        let mut s = self.as_ffi();
        let xf = m.0;
        unsafe { vge_line_xf(&mut s, &xf, x0, y0, x1, y1, color) };
    }

    pub fn polyline(&mut self, points: &[(i32, i32)], color: Color) {
        if points.len() < 2 {
            return;
        }
        let mut flat: Vec<i32> = Vec::with_capacity(points.len() * 2);
        for &(x, y) in points {
            flat.push(x);
            flat.push(y);
        }
        let mut s = self.as_ffi();
        unsafe { vge_polyline(&mut s, flat.as_ptr(), points.len() as i32, color) };
    }

    /// Tight RGB888 export. Length = w*h*3.
    pub fn export_rgb24(&self) -> Vec<u8> {
        let n = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(3);
        let mut out = vec![0u8; n];
        let s = VgeSurface {
            width: self.width,
            height: self.height,
            stride: self.stride,
            _pad: 0,
            pixels: self.pixels.as_ptr() as *mut u8,
        };
        unsafe { vge_export_rgb24(&s, out.as_mut_ptr()) };
        out
    }

    /// RGBA8888 for Kitty (`f=32`). Length = w*h*4. Byte order R,G,B,A.
    pub fn export_rgba32(&self) -> Vec<u8> {
        let n = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        let mut out = vec![0u8; n];
        let mut i = 0usize;
        for y in 0..self.height {
            for x in 0..self.width {
                let c = self.get(x as i32, y as i32).unwrap_or(0);
                out[i] = ((c >> 16) & 0xFF) as u8;
                out[i + 1] = ((c >> 8) & 0xFF) as u8;
                out[i + 2] = (c & 0xFF) as u8;
                out[i + 3] = ((c >> 24) & 0xFF) as u8;
                i += 4;
            }
        }
        out
    }

    /// Clear to fully transparent (overlay-ready).
    pub fn clear_transparent(&mut self) {
        self.pixels.fill(0);
    }

    /// Read packed XRGB at (x,y). Returns None if out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<Color> {
        if x < 0 || y < 0 {
            return None;
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y * self.stride + x * 4) as usize;
        let b = &self.pixels[i..i + 4];
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// Affine transform (rotate / scale / translate).
#[derive(Clone, Copy, Debug)]
pub struct Xform(pub VgeXform);

impl Default for Xform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Xform {
    pub fn identity() -> Self {
        let mut m = VgeXform {
            a: 1.0,
            b: 0.0,
            tx: 0.0,
            c: 0.0,
            d: 1.0,
            ty: 0.0,
        };
        unsafe { vge_xform_identity(&mut m) };
        Self(m)
    }

    pub fn translate(mut self, tx: f32, ty: f32) -> Self {
        unsafe { vge_xform_translate(&mut self.0, tx, ty) };
        self
    }

    pub fn scale(mut self, sx: f32, sy: f32) -> Self {
        unsafe { vge_xform_scale(&mut self.0, sx, sy) };
        self
    }

    pub fn rotate(mut self, radians: f32) -> Self {
        unsafe { vge_xform_rotate(&mut self.0, radians) };
        self
    }

    pub fn rotate_deg(self, degrees: f32) -> Self {
        self.rotate(degrees * PI / 180.0)
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        let mut ox = 0.0f32;
        let mut oy = 0.0f32;
        unsafe { vge_xform_apply(&self.0, x, y, &mut ox, &mut oy) };
        (ox, oy)
    }
}

/// Whether this build linked the x86_64 assembly hot path.
pub fn using_assembly() -> bool {
    cfg!(vge_asm)
}

/// Version string from **libvge** (`vge_version` in assembly).
pub fn engine_version() -> &'static str {
    unsafe {
        let p = vge_version();
        if p.is_null() {
            return VERSION;
        }
        std::ffi::CStr::from_ptr(p).to_str().unwrap_or(VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_lights_endpoints() {
        let mut s = Surface::new(64, 64);
        s.clear(BLACK);
        s.line(0, 0, 63, 0, GREEN);
        assert_eq!(s.get(0, 0), Some(GREEN));
        assert_eq!(s.get(63, 0), Some(GREEN));
        assert_eq!(s.get(32, 0), Some(GREEN));
        assert_eq!(s.get(0, 1), Some(BLACK));
    }

    #[test]
    fn diagonal_sets_mid() {
        let mut s = Surface::new(11, 11);
        s.clear(BLACK);
        s.line(0, 0, 10, 10, RED);
        assert_eq!(s.get(0, 0), Some(RED));
        assert_eq!(s.get(10, 10), Some(RED));
        assert_eq!(s.get(5, 5), Some(RED));
    }

    #[test]
    fn circle_outline_not_fill() {
        let mut s = Surface::new(40, 40);
        s.clear(BLACK);
        s.circle(20, 20, 10, CYAN);
        assert_eq!(s.get(20 + 10, 20), Some(CYAN));
        assert_eq!(s.get(20, 20), Some(BLACK)); // center empty (outline only)
    }

    #[test]
    fn rotate_line_around_center() {
        let mut s = Surface::new(100, 100);
        s.clear(BLACK);
        let m = Xform::identity()
            .translate(50.0, 50.0)
            .rotate_deg(90.0)
            .translate(-50.0, -50.0);
        // Horizontal segment through center becomes vertical after 90° about center.
        s.line_xf(&m, 30.0, 50.0, 70.0, 50.0, AMBER);
        // After 90° CCW about (50,50): (30,50)->(50,30), (70,50)->(50,70)
        assert_eq!(s.get(50, 30), Some(AMBER));
        assert_eq!(s.get(50, 70), Some(AMBER));
    }

    #[test]
    fn export_rgb24_length() {
        let s = Surface::new(8, 4);
        let rgb = s.export_rgb24();
        assert_eq!(rgb.len(), 8 * 4 * 3);
    }

    #[test]
    fn version_nonzero() {
        assert!(!engine_version().is_empty());
    }
}
