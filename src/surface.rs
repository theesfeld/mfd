//! Pixel surface + FFI to pure-asm **libmfd** (plot/line/circle).

use crate::Color;
use std::f32::consts::PI;

#[repr(C)]
pub struct MfdSurface {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub _pad: u32,
    pub pixels: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MfdXform {
    pub a: f32,
    pub b: f32,
    pub tx: f32,
    pub c: f32,
    pub d: f32,
    pub ty: f32,
}

extern "C" {
    pub fn mfd_clear(s: *mut MfdSurface, color: u32);
    pub fn mfd_plot(s: *mut MfdSurface, x: i32, y: i32, color: u32);
    pub fn mfd_line(s: *mut MfdSurface, x0: i32, y0: i32, x1: i32, y1: i32, color: u32);
    pub fn mfd_line_aa(s: *mut MfdSurface, x0: i32, y0: i32, x1: i32, y1: i32, color: u32);
    pub fn mfd_line_thick(
        s: *mut MfdSurface,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: u32,
        thickness: i32,
    );
    pub fn mfd_circle(s: *mut MfdSurface, cx: i32, cy: i32, r: i32, color: u32);
    pub fn mfd_rect_fill(s: *mut MfdSurface, x0: i32, y0: i32, x1: i32, y1: i32, color: u32);
    pub fn mfd_polyline(s: *mut MfdSurface, xy: *const i32, n: i32, color: u32);
    pub fn mfd_export_rgb24(s: *const MfdSurface, dest: *mut u8);
    pub fn mfd_version() -> *const std::os::raw::c_char;
    pub fn mfd_xform_identity(m: *mut MfdXform);
    pub fn mfd_xform_translate(m: *mut MfdXform, tx: f32, ty: f32);
    pub fn mfd_xform_scale(m: *mut MfdXform, sx: f32, sy: f32);
    pub fn mfd_xform_rotate(m: *mut MfdXform, radians: f32);
    pub fn mfd_line_xf(
        s: *mut MfdSurface,
        m: *const MfdXform,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        color: u32,
    );
}

pub struct Surface {
    width: u32,
    height: u32,
    stride: u32,
    pixels: Vec<u8>,
}

impl Surface {
    pub fn new(width: u32, height: u32) -> Self {
        let stride = width.saturating_mul(4);
        let n = stride.saturating_mul(height) as usize;
        Self {
            width,
            height,
            stride,
            pixels: vec![0u8; n],
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

    fn as_ffi(&mut self) -> MfdSurface {
        MfdSurface {
            width: self.width,
            height: self.height,
            stride: self.stride,
            _pad: 0,
            pixels: self.pixels.as_mut_ptr(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        let mut s = self.as_ffi();
        unsafe { mfd_clear(&mut s, color) };
    }

    pub fn clear_black(&mut self) {
        self.clear(crate::color::BLACK);
    }

    pub fn plot(&mut self, x: i32, y: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { mfd_plot(&mut s, x, y, color) };
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        self.line_aa(x0, y0, x1, y1, color);
    }

    pub fn line_fast(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { mfd_line(&mut s, x0, y0, x1, y1, color) };
    }

    pub fn line_aa(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { mfd_line_aa(&mut s, x0, y0, x1, y1, color) };
    }

    pub fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color, t: i32) {
        let mut s = self.as_ffi();
        unsafe { mfd_line_thick(&mut s, x0, y0, x1, y1, color, t) };
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { mfd_circle(&mut s, cx, cy, r, color) };
    }

    pub fn rect_fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut s = self.as_ffi();
        unsafe { mfd_rect_fill(&mut s, x0, y0, x1, y1, color) };
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
        unsafe { mfd_polyline(&mut s, flat.as_ptr(), points.len() as i32, color) };
    }

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

    pub fn export_rgba32(&self) -> Vec<u8> {
        let n = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        let mut out = vec![0u8; n];
        self.export_rgba32_into(&mut out);
        out
    }

    /// Write RGBA8888 into `out` (resized as needed). Avoids alloc when reusing a buffer.
    pub fn export_rgba32_into(&self, out: &mut Vec<u8>) {
        let n = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        out.resize(n, 0);
        let stride = self.stride as usize;
        let px = &self.pixels;
        let mut i = 0usize;
        for y in 0..self.height as usize {
            let row = y * stride;
            for x in 0..self.width as usize {
                let o = row + x * 4;
                // Storage is little-endian 0xAARRGGBB as bytes B,G,R,A
                let b = px[o];
                let g = px[o + 1];
                let r = px[o + 2];
                let a = px[o + 3];
                out[i] = r;
                out[i + 1] = g;
                out[i + 2] = b;
                out[i + 3] = a;
                i += 4;
            }
        }
    }

    /// Scale all RGB channels by `factor` (0..1+). Alpha unchanged. Used for BRT.
    pub fn apply_brightness(&mut self, factor: f32) {
        let f = factor.clamp(0.0, 2.0);
        if (f - 1.0).abs() < 0.001 {
            return;
        }
        let stride = self.stride as usize;
        for y in 0..self.height as usize {
            let row = y * stride;
            for x in 0..self.width as usize {
                let o = row + x * 4;
                // LE: B G R A
                self.pixels[o] = ((self.pixels[o] as f32) * f).min(255.0) as u8;
                self.pixels[o + 1] = ((self.pixels[o + 1] as f32) * f).min(255.0) as u8;
                self.pixels[o + 2] = ((self.pixels[o + 2] as f32) * f).min(255.0) as u8;
            }
        }
    }

    /// Expand or compress mid-tones around 0.5 (CON rocker). `level` 0..1, 0.5 = neutral.
    pub fn apply_contrast(&mut self, level: f32) {
        let l = level.clamp(0.0, 1.0);
        // Map 0..1 → contrast factor 0.5..2.0 centered at 1.0 when level=0.5
        let c = 0.5 + l * 1.5;
        if (c - 1.0).abs() < 0.02 {
            return;
        }
        let stride = self.stride as usize;
        for y in 0..self.height as usize {
            let row = y * stride;
            for x in 0..self.width as usize {
                let o = row + x * 4;
                for ch in 0..3 {
                    let v = self.pixels[o + ch] as f32 / 255.0;
                    let out = ((v - 0.5) * c + 0.5).clamp(0.0, 1.0);
                    self.pixels[o + ch] = (out * 255.0) as u8;
                }
            }
        }
    }

    /// Dim non-near-black pixels toward black (SYM symbology intensity). `level` 0..1.
    pub fn apply_symbology(&mut self, level: f32) {
        let f = level.clamp(0.15, 1.0);
        if (f - 1.0).abs() < 0.02 {
            return;
        }
        let stride = self.stride as usize;
        for y in 0..self.height as usize {
            let row = y * stride;
            for x in 0..self.width as usize {
                let o = row + x * 4;
                // Skip near-black glass
                let sum =
                    self.pixels[o] as u16 + self.pixels[o + 1] as u16 + self.pixels[o + 2] as u16;
                if sum < 24 {
                    continue;
                }
                self.pixels[o] = ((self.pixels[o] as f32) * f).min(255.0) as u8;
                self.pixels[o + 1] = ((self.pixels[o + 1] as f32) * f).min(255.0) as u8;
                self.pixels[o + 2] = ((self.pixels[o + 2] as f32) * f).min(255.0) as u8;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Xform(pub MfdXform);

impl Xform {
    pub fn identity() -> Self {
        let mut m = MfdXform {
            a: 1.0,
            b: 0.0,
            tx: 0.0,
            c: 0.0,
            d: 1.0,
            ty: 0.0,
        };
        unsafe { mfd_xform_identity(&mut m) };
        Self(m)
    }

    pub fn translate(mut self, tx: f32, ty: f32) -> Self {
        unsafe { mfd_xform_translate(&mut self.0, tx, ty) };
        self
    }

    pub fn rotate(mut self, radians: f32) -> Self {
        unsafe { mfd_xform_rotate(&mut self.0, radians) };
        self
    }

    pub fn rotate_deg(self, degrees: f32) -> Self {
        self.rotate(degrees * PI / 180.0)
    }
}

pub fn using_assembly() -> bool {
    cfg!(mfd_asm)
}

pub fn engine_version() -> &'static str {
    unsafe {
        let p = mfd_version();
        if p.is_null() {
            return "unknown";
        }
        std::ffi::CStr::from_ptr(p).to_str().unwrap_or("unknown")
    }
}
