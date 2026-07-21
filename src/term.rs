//! Present a VGE pixel surface **in the terminal**.
//!
//! The engine still lights individual pixels. This module only maps that
//! pixel buffer to whatever the host can show:
//!
//! 1. **Kitty** graphics protocol — true RGB pixels (Ghostty, Kitty, WezTerm, …)
//! 2. **Half-block** truecolor — 2 vertical pixels per cell (most terminals + many TTYs)
//! 3. **ASCII** density — last resort for dumb / mono TTY
//!
//! Force with env: `VGE_TERM=kitty|half|ascii`

use crate::{Color, Surface};
use std::io::{self, Write};
use std::sync::atomic::{AtomicU32, Ordering};

static KITTY_ID: AtomicU32 = AtomicU32::new(1);

/// How to put engine pixels on a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermBackend {
    /// Kitty graphics protocol (real pixel image placement).
    Kitty,
    /// Unicode half-block + 24-bit ANSI (works widely, including many TTYs).
    HalfBlock,
    /// ASCII density (dumb terminal / no Unicode).
    Ascii,
}

/// Detect a workable backend. Prefer real pixels when the host supports them.
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
    if std::env::var_os("OBDTUI_FORCE_ASCII").is_some() {
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

    // Truecolor terminals (most modern emulators): half-block path.
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return TermBackend::HalfBlock;
    }

    // Linux console / bare host often still do Unicode + color.
    if term == "dumb" || term.is_empty() {
        if atty_stdout() {
            return TermBackend::HalfBlock;
        }
        return TermBackend::Ascii;
    }

    // xterm, rxvt, alacritty, foot, konsole, … → half-block truecolor.
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

/// Recommended pixel surface size for the current backend and terminal.
pub fn suggested_surface_size(backend: TermBackend) -> (u32, u32) {
    let (cols, rows) = terminal_cells();
    let cols = cols.max(20) as u32;
    let rows = rows.saturating_sub(1).max(10) as u32; // leave status line
    match backend {
        TermBackend::Kitty => {
            // ~10×20 device pixels per cell (crisp vectors).
            (cols * 10, rows * 20)
        }
        TermBackend::HalfBlock => {
            // 1 cell wide × 2 pixels tall per cell.
            (cols, rows * 2)
        }
        TermBackend::Ascii => (cols, rows),
    }
}

/// Enter alternate screen, clear, hide cursor.
pub fn enter_fullscreen() -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b[?1049h\x1b[H\x1b[2J\x1b[?25l")?;
    out.flush()
}

/// Leave alternate screen, show cursor.
pub fn leave_fullscreen() -> io::Result<()> {
    let mut out = io::stdout().lock();
    // Delete any Kitty image we may have placed.
    write!(out, "\x1b_Ga=d,d=a\x1b\\")?;
    write!(out, "\x1b[?25h\x1b[?1049l")?;
    out.flush()
}

/// Present surface at top-left of the terminal.
pub fn present(surface: &Surface, backend: TermBackend) -> io::Result<()> {
    match backend {
        TermBackend::Kitty => present_kitty(surface),
        TermBackend::HalfBlock => present_halfblock(surface),
        TermBackend::Ascii => present_ascii(surface),
    }
}

fn present_kitty(surface: &Surface) -> io::Result<()> {
    let (cols, rows) = terminal_cells();
    let cols = cols.max(1);
    let rows = rows.saturating_sub(1).max(1);
    let rgb = surface.export_rgb24();
    let w = surface.width();
    let h = surface.height();
    let id = KITTY_ID.load(Ordering::Relaxed);

    let mut out = io::stdout().lock();
    write!(out, "\x1b[H")?;

    let header = format!("a=T,f=24,t=d,s={w},v={h},c={cols},r={rows},i={id},q=2");
    let b64 = b64_encode(&rgb);
    let chunk = 4096usize;
    let bytes = b64.as_bytes();
    let mut offset = 0;
    let mut first = true;
    while offset < bytes.len() {
        let end = (offset + chunk).min(bytes.len());
        let more = if end < bytes.len() { 1 } else { 0 };
        let slice = &bytes[offset..end];
        if first {
            write!(
                out,
                "\x1b_G{},m={};{}\x1b\\",
                header,
                more,
                std::str::from_utf8(slice).unwrap_or("")
            )?;
            first = false;
        } else {
            write!(
                out,
                "\x1b_Gm={};{}\x1b\\",
                more,
                std::str::from_utf8(slice).unwrap_or("")
            )?;
        }
        offset = end;
    }
    out.flush()
}

fn present_halfblock(surface: &Surface) -> io::Result<()> {
    let w = surface.width() as i32;
    let h = surface.height() as i32;
    let mut out = io::stdout().lock();
    write!(out, "\x1b[H")?;

    let rows = (h + 1) / 2;
    for row in 0..rows {
        let y0 = row * 2;
        let y1 = y0 + 1;
        for x in 0..w {
            let top = surface.get(x, y0).unwrap_or(0);
            let bot = if y1 < h {
                surface.get(x, y1).unwrap_or(0)
            } else {
                0
            };
            let (tr, tg, tb) = unpack_rgb(top);
            let (br, bg, bb) = unpack_rgb(bot);
            // ▀ = upper half; fg = top, bg = bottom.
            write!(out, "\x1b[38;2;{tr};{tg};{tb}m\x1b[48;2;{br};{bg};{bb}m▀")?;
        }
        write!(out, "\x1b[0m\r\n")?;
    }
    out.flush()
}

fn present_ascii(surface: &Surface) -> io::Result<()> {
    const RAMP: &[u8] = b" .:-=+*#%@";
    let w = surface.width() as i32;
    let h = surface.height() as i32;
    let mut out = io::stdout().lock();
    write!(out, "\x1b[H")?;
    for y in 0..h {
        for x in 0..w {
            let c = surface.get(x, y).unwrap_or(0);
            let (r, g, b) = unpack_rgb(c);
            let lum = (r as u32 * 3 + g as u32 * 6 + b as u32) / 10;
            let idx = (lum * (RAMP.len() as u32 - 1) / 255) as usize;
            let ch = RAMP[idx] as char;
            if r | g | b == 0 {
                write!(out, "{ch}")?;
            } else {
                write!(out, "\x1b[38;2;{r};{g};{b}m{ch}\x1b[0m")?;
            }
        }
        write!(out, "\r\n")?;
    }
    out.flush()
}

#[inline]
fn unpack_rgb(c: Color) -> (u8, u8, u8) {
    (
        ((c >> 16) & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        (c & 0xFF) as u8,
    )
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
}
