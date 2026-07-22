//! Present a VGE pixel surface in the terminal.
//!
//! # Speed rule
//!
//! The **raster** path is near-instant. The **present** path is the bottleneck
//! (Kitty base64, half-block ANSI). This module:
//!
//! - builds output in one buffer, one write
//! - caps default pixel density so present stays fast
//! - supports a **viewport** (cell rectangle) so vectors sit on top of text
//!
//! Force backend: `VGE_TERM=kitty|half|ascii`  
//! Cap pixels: `VGE_MAX_W`, `VGE_MAX_H` (defaults 960×540)

use crate::{Color, Surface};
use std::io::{self, Write};
use std::sync::atomic::{AtomicU32, Ordering};

static KITTY_ID: AtomicU32 = AtomicU32::new(42);

/// How to put engine pixels on a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermBackend {
    /// Kitty graphics protocol (Ghostty, Kitty, WezTerm, …).
    Kitty,
    /// Unicode half-block + 24-bit ANSI.
    HalfBlock,
    /// ASCII density (dumb host).
    Ascii,
}

/// Cell rectangle for overlay placement (1-based row/col for CSI CUP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    /// Left cell (0-based).
    pub col: u16,
    /// Top cell (0-based).
    pub row: u16,
    /// Width in cells.
    pub cols: u16,
    /// Height in cells.
    pub rows: u16,
}

impl Viewport {
    pub fn full_terminal() -> Self {
        let (c, r) = terminal_cells();
        Self {
            col: 0,
            row: 0,
            cols: c.max(1),
            rows: r.saturating_sub(1).max(1),
        }
    }

    /// Centered box using a fraction of the terminal (e.g. 0.7 = 70%).
    pub fn centered_frac(frac_w: f32, frac_h: f32) -> Self {
        let (tc, tr) = terminal_cells();
        let cols = ((tc as f32) * frac_w.clamp(0.1, 1.0)) as u16;
        let rows = ((tr as f32) * frac_h.clamp(0.1, 1.0)) as u16;
        let cols = cols.max(10).min(tc);
        let rows = rows.max(4).min(tr.saturating_sub(1).max(1));
        let col = tc.saturating_sub(cols) / 2;
        let row = tr.saturating_sub(rows) / 2;
        Self {
            col,
            row,
            cols,
            rows,
        }
    }
}

/// Detect a workable backend.
pub fn detect_backend() -> TermBackend {
    if let Ok(v) = std::env::var("VGE_TERM") {
        match v.to_ascii_lowercase().as_str() {
            "kitty" | "pixel" | "gfx" => return TermBackend::Kitty,
            "half" | "halfblock" | "block" => return TermBackend::HalfBlock,
            "ascii" | "dumb" | "tty" => return TermBackend::Ascii,
            _ => {}
        }
    }
    if std::env::var_os("VGE_FORCE_ASCII").is_some() {
        return TermBackend::Ascii;
    }

    let prog = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let colorterm = std::env::var("COLORTERM")
        .unwrap_or_default()
        .to_ascii_lowercase();

    if std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var_os("WEZTERM_EXECUTABLE").is_some()
        || std::env::var_os("WEZTERM_PANE").is_some()
        || std::env::var_os("GHOSTTY_RESOURCES_DIR").is_some()
        || prog.contains("ghostty")
        || prog.contains("kitty")
        || prog.contains("wezterm")
        || term.contains("kitty")
        || term.contains("ghostty")
    {
        return TermBackend::Kitty;
    }

    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return TermBackend::HalfBlock;
    }
    if term == "dumb" || term.is_empty() {
        return if atty_stdout() {
            TermBackend::HalfBlock
        } else {
            TermBackend::Ascii
        };
    }
    TermBackend::HalfBlock
}

fn atty_stdout() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Terminal size in character cells `(cols, rows)`.
pub fn terminal_cells() -> (u16, u16) {
    if let (Ok(c), Ok(r)) = (std::env::var("COLUMNS"), std::env::var("LINES")) {
        if let (Ok(c), Ok(r)) = (c.parse::<u16>(), r.parse::<u16>()) {
            if c > 0 && r > 0 {
                return (c, r);
            }
        }
    }
    #[cfg(unix)]
    {
        unsafe {
            let mut ws: libc::winsize = std::mem::zeroed();
            if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
                && ws.ws_col > 0
                && ws.ws_row > 0
            {
                return (ws.ws_col, ws.ws_row);
            }
        }
    }
    (80, 24)
}

fn max_pixels() -> (u32, u32) {
    // Defaults keep Kitty payloads small so the terminal does not stutter.
    let mw = std::env::var("VGE_MAX_W")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(640u32);
    let mh = std::env::var("VGE_MAX_H")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(360u32);
    (mw.max(64), mh.max(64))
}

/// Pixel surface size for a viewport (capped for present speed).
pub fn surface_size_for_viewport(backend: TermBackend, vp: Viewport) -> (u32, u32) {
    let (mw, mh) = max_pixels();
    let cols = vp.cols.max(1) as u32;
    let rows = vp.rows.max(1) as u32;
    let (w, h) = match backend {
        // Low device px/cell: terminal scales the image to the cell box.
        // Less base64 ⇒ smoother present (less queue pressure on the emulator).
        TermBackend::Kitty => (cols * 3, rows * 6),
        TermBackend::HalfBlock => (cols, rows * 2),
        TermBackend::Ascii => (cols, rows),
    };
    (w.min(mw), h.min(mh))
}

/// Recommended full-terminal surface (capped).
pub fn suggested_surface_size(backend: TermBackend) -> (u32, u32) {
    surface_size_for_viewport(backend, Viewport::full_terminal())
}

/// Hide cursor only (keep normal screen — overlay mode).
pub fn enter_overlay() -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b[?25l")?;
    out.flush()
}

/// Full alternate screen (legacy demo mode).
pub fn enter_fullscreen() -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b[?1049h\x1b[H\x1b[2J\x1b[?25l")?;
    out.flush()
}

pub fn leave_overlay() -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b_Ga=d,d=a\x1b\\")?;
    write!(out, "\x1b[?25h")?;
    out.flush()
}

pub fn leave_fullscreen() -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b_Ga=d,d=a\x1b\\")?;
    write!(out, "\x1b[?25h\x1b[?1049l")?;
    out.flush()
}

/// Present at top-left, full suggested area.
pub fn present(surface: &Surface, backend: TermBackend) -> io::Result<()> {
    present_at(surface, backend, Viewport::full_terminal())
}

/// Present inside a cell rectangle. Text outside the viewport is untouched
/// (overlay). This is how vectors sit **on top of** a normal TUI.
pub fn present_at(surface: &Surface, backend: TermBackend, vp: Viewport) -> io::Result<()> {
    match backend {
        TermBackend::Kitty => present_kitty_at(surface, vp),
        TermBackend::HalfBlock => present_halfblock_at(surface, vp),
        TermBackend::Ascii => present_ascii_at(surface, vp),
    }
}

fn present_kitty_at(surface: &Surface, vp: Viewport) -> io::Result<()> {
    let cols = vp.cols.max(1);
    let rows = vp.rows.max(1);
    let w = surface.width();
    let h = surface.height();
    let rgb = surface.export_rgb24();
    let id = KITTY_ID.load(Ordering::Relaxed);

    // Build one buffer: cursor → graphics payload.
    let mut out = Vec::with_capacity(rgb.len() * 2 + 128);
    // CUP is 1-based
    push_cup(&mut out, vp.row + 1, vp.col + 1);

    let header = format!("a=T,f=24,t=d,s={w},v={h},c={cols},r={rows},i={id},q=2");
    let b64 = b64_encode(&rgb);
    let chunk = 4096usize;
    let bytes = b64.as_bytes();
    let mut offset = 0;
    let mut first = true;
    while offset < bytes.len() {
        let end = (offset + chunk).min(bytes.len());
        let more = if end < bytes.len() { 1 } else { 0 };
        if first {
            out.extend_from_slice(b"\x1b_G");
            out.extend_from_slice(header.as_bytes());
            out.extend_from_slice(b",m=");
            out.push(if more == 1 { b'1' } else { b'0' });
            out.push(b';');
            out.extend_from_slice(&bytes[offset..end]);
            out.extend_from_slice(b"\x1b\\");
            first = false;
        } else {
            out.extend_from_slice(b"\x1b_Gm=");
            out.push(if more == 1 { b'1' } else { b'0' });
            out.push(b';');
            out.extend_from_slice(&bytes[offset..end]);
            out.extend_from_slice(b"\x1b\\");
        }
        offset = end;
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&out)?;
    stdout.flush()
}

/// Fast half-block: one buffer, raw pixel walk, single write.
fn present_halfblock_at(surface: &Surface, vp: Viewport) -> io::Result<()> {
    let w = surface.width() as usize;
    let h = surface.height() as usize;
    let stride = surface.stride() as usize;
    let px = surface.pixels();
    let rows = h.div_ceil(2);

    // ~40 bytes per cell worst case
    let mut buf = Vec::with_capacity(rows * w * 40 + 32);

    for row in 0..rows {
        push_cup(&mut buf, vp.row + 1 + row as u16, vp.col + 1);
        let y0 = row * 2;
        let y1 = y0 + 1;
        for x in 0..w {
            let top = load_px(px, stride, x, y0, w, h);
            let bot = if y1 < h {
                load_px(px, stride, x, y1, w, h)
            } else {
                0
            };
            let (tr, tg, tb) = unpack_rgb(top);
            let (br, bg, bb) = unpack_rgb(bot);
            // \x1b[38;2;R;G;Bm\x1b[48;2;R;G;Bm▀
            buf.extend_from_slice(b"\x1b[38;2;");
            push_u8(&mut buf, tr);
            buf.push(b';');
            push_u8(&mut buf, tg);
            buf.push(b';');
            push_u8(&mut buf, tb);
            buf.extend_from_slice(b"m\x1b[48;2;");
            push_u8(&mut buf, br);
            buf.push(b';');
            push_u8(&mut buf, bg);
            buf.push(b';');
            push_u8(&mut buf, bb);
            buf.extend_from_slice(b"m\xE2\x96\x80"); // ▀ UTF-8
        }
        buf.extend_from_slice(b"\x1b[0m");
    }

    let mut stdout = io::stdout().lock();
    stdout.write_all(&buf)?;
    stdout.flush()
}

fn present_ascii_at(surface: &Surface, vp: Viewport) -> io::Result<()> {
    const RAMP: &[u8] = b" .:-=+*#%@";
    let w = surface.width() as usize;
    let h = surface.height() as usize;
    let stride = surface.stride() as usize;
    let px = surface.pixels();
    let mut buf = Vec::with_capacity(h * w * 24 + 32);

    for y in 0..h {
        push_cup(&mut buf, vp.row + 1 + y as u16, vp.col + 1);
        for x in 0..w {
            let c = load_px(px, stride, x, y, w, h);
            let (r, g, b) = unpack_rgb(c);
            let lum = (r as u32 * 3 + g as u32 * 6 + b as u32) / 10;
            let idx = (lum * (RAMP.len() as u32 - 1) / 255) as usize;
            let ch = RAMP[idx];
            if r | g | b == 0 {
                buf.push(ch);
            } else {
                buf.extend_from_slice(b"\x1b[38;2;");
                push_u8(&mut buf, r);
                buf.push(b';');
                push_u8(&mut buf, g);
                buf.push(b';');
                push_u8(&mut buf, b);
                buf.push(b'm');
                buf.push(ch);
                buf.extend_from_slice(b"\x1b[0m");
            }
        }
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&buf)?;
    stdout.flush()
}

#[inline]
fn load_px(px: &[u8], stride: usize, x: usize, y: usize, w: usize, h: usize) -> Color {
    if x >= w || y >= h {
        return 0;
    }
    let i = y * stride + x * 4;
    if i + 3 >= px.len() {
        return 0;
    }
    u32::from_le_bytes([px[i], px[i + 1], px[i + 2], px[i + 3]])
}

#[inline]
fn unpack_rgb(c: Color) -> (u8, u8, u8) {
    (
        ((c >> 16) & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        (c & 0xFF) as u8,
    )
}

#[inline]
fn push_u8(buf: &mut Vec<u8>, n: u8) {
    if n >= 100 {
        buf.push(b'0' + n / 100);
        buf.push(b'0' + (n / 10) % 10);
        buf.push(b'0' + n % 10);
    } else if n >= 10 {
        buf.push(b'0' + n / 10);
        buf.push(b'0' + n % 10);
    } else {
        buf.push(b'0' + n);
    }
}

fn push_cup(buf: &mut Vec<u8>, row_1: u16, col_1: u16) {
    // CSI row;col H
    buf.extend_from_slice(b"\x1b[");
    push_u16(buf, row_1);
    buf.push(b';');
    push_u16(buf, col_1);
    buf.push(b'H');
}

fn push_u16(buf: &mut Vec<u8>, n: u16) {
    if n >= 1000 {
        buf.push(b'0' + (n / 1000) as u8);
        buf.push(b'0' + ((n / 100) % 10) as u8);
        buf.push(b'0' + ((n / 10) % 10) as u8);
        buf.push(b'0' + (n % 10) as u8);
    } else if n >= 100 {
        buf.push(b'0' + (n / 100) as u8);
        buf.push(b'0' + ((n / 10) % 10) as u8);
        buf.push(b'0' + (n % 10) as u8);
    } else if n >= 10 {
        buf.push(b'0' + (n / 10) as u8);
        buf.push(b'0' + (n % 10) as u8);
    } else {
        buf.push(b'0' + n as u8);
    }
}

fn b64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    if i < data.len() {
        let a = data[i] as u32;
        let b = if i + 1 < data.len() {
            data[i + 1] as u32
        } else {
            0
        };
        let n = (a << 16) | (b << 8);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if i + 1 < data.len() {
            out.push(T[((n >> 6) & 63) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b64_hello() {
        assert_eq!(b64_encode(b"hello"), "aGVsbG8=");
    }

    #[test]
    fn detect_does_not_panic() {
        let _ = detect_backend();
    }

    #[test]
    fn surface_size_is_capped() {
        let vp = Viewport {
            col: 0,
            row: 0,
            cols: 500,
            rows: 200,
        };
        let (w, h) = surface_size_for_viewport(TermBackend::Kitty, vp);
        let (mw, mh) = max_pixels();
        assert!(w <= mw && h <= mh);
    }
}
