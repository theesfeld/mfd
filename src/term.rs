//! Present a MFD pixel surface in the terminal.
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
//! Force backend: `MFD_TERM=kitty|half|ascii`  
//! Cap pixels: `MFD_MAX_W`, `MFD_MAX_H` (defaults 1280×720 for MFD density)

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
    if let Ok(v) = std::env::var("MFD_TERM") {
        match v.to_ascii_lowercase().as_str() {
            "kitty" | "pixel" | "gfx" => return TermBackend::Kitty,
            "half" | "halfblock" | "block" => return TermBackend::HalfBlock,
            "ascii" | "dumb" | "tty" => return TermBackend::Ascii,
            _ => {}
        }
    }
    if std::env::var_os("MFD_FORCE_ASCII").is_some() {
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
    // Square MFD default 512 (keep present light). Override with MFD_MAX_*.
    let mw = std::env::var("MFD_MAX_W")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(512u32);
    let mh = std::env::var("MFD_MAX_H")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(512u32);
    (mw.max(64), mh.max(64))
}

/// Pixel surface size for a viewport (capped for present speed).
pub fn surface_size_for_viewport(backend: TermBackend, vp: Viewport) -> (u32, u32) {
    let (mw, mh) = max_pixels();
    let cols = vp.cols.max(1) as u32;
    let rows = vp.rows.max(1) as u32;
    let (w, h) = match backend {
        // Keep density modest — large Kitty payloads queue and crawl after minutes.
        TermBackend::Kitty => (cols * 6, rows * 12),
        TermBackend::HalfBlock => (cols, rows * 2),
        TermBackend::Ascii => (cols, rows),
    };
    (w.min(mw), h.min(mh))
}

/// Recommended full-terminal surface (capped).
pub fn suggested_surface_size(backend: TermBackend) -> (u32, u32) {
    surface_size_for_viewport(backend, Viewport::full_terminal())
}

/// Square cell viewport (F-16 class face is square — MLU color MFD ≈ **4×4 in / 10×10 cm**).
///
/// Centered in the terminal. `frac` of the smaller terminal dimension (0.5–1.0).
pub fn square_mfd_viewport(frac: f32) -> Viewport {
    let (tc, tr) = terminal_cells();
    let f = frac.clamp(0.4, 1.0);
    // Cell aspect is ~0.5 width:height of a cell; for a *visual* square use more cols.
    // Approx: square pixels when cols ≈ rows * 2 for half-block; for Kitty we map
    // surface to cell box so use equal cell-side using min dimension in "cell units".
    let side = ((tc.min(tr) as f32) * f) as u16;
    let side = side.max(12);
    // Prefer slightly wider cell box so letterboxing matches square image.
    let cols = side.min(tc).max(10);
    let rows = side.min(tr).max(8);
    // Use min of square-ish cell count
    let edge = cols.min(rows);
    let cols = edge;
    let rows = edge;
    let col = tc.saturating_sub(cols) / 2;
    let row = tr.saturating_sub(rows) / 2;
    Viewport {
        col,
        row,
        cols,
        rows,
    }
}

/// Square pixel size for an MFD face (default 512, square, capped).
pub fn square_mfd_pixels(backend: TermBackend) -> (u32, u32) {
    let (mw, mh) = max_pixels();
    let side = mw.min(mh).min(640);
    let side = match backend {
        TermBackend::Ascii => side.min(120),
        TermBackend::HalfBlock => side.min(400),
        TermBackend::Kitty => side,
    };
    (side.max(128), side.max(128))
}

/// Reusable present buffers (avoids multi-MB alloc/frame → terminal crawl).
#[derive(Default)]
pub struct PresentScratch {
    pub rgba: Vec<u8>,
    pub b64: String,
    pub out: Vec<u8>,
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

/// RAII raw (non-canonical) stdin so single keypresses are available without Enter.
///
/// Disables `ICANON` + `ECHO`, sets `VMIN=0` / `VTIME=0` (non-blocking reads).
/// Restores prior termios on drop.
#[cfg(unix)]
pub struct RawStdin {
    fd: i32,
    original: libc::termios,
}

#[cfg(unix)]
impl RawStdin {
    /// Enable raw-ish input on stdin. No-op failure if not a TTY.
    pub fn enter() -> io::Result<Self> {
        unsafe {
            if libc::isatty(libc::STDIN_FILENO) == 0 {
                return Err(io::Error::other("stdin is not a tty"));
            }
            let mut original: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut original) != 0 {
                return Err(io::Error::last_os_error());
            }
            let mut raw = original;
            // Byte-at-a-time, no echo. Keep ISIG so Ctrl+C still works.
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                fd: libc::STDIN_FILENO,
                original,
            })
        }
    }

    /// Non-blocking: drain all pending input bytes (oldest first).
    pub fn read_keys(&self, out: &mut Vec<u8>) -> io::Result<()> {
        out.clear();
        unsafe {
            loop {
                let mut buf = [0u8; 64];
                let r = libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len());
                if r < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.raw_os_error() == Some(libc::EAGAIN)
                        || err.raw_os_error() == Some(libc::EWOULDBLOCK)
                    {
                        break;
                    }
                    // EINTR: try again
                    if err.raw_os_error() == Some(libc::EINTR) {
                        continue;
                    }
                    return Err(err);
                }
                if r == 0 {
                    break;
                }
                out.extend_from_slice(&buf[..r as usize]);
                // One read is enough for interactive keys; more if paste flood.
                if r < buf.len() as isize {
                    break;
                }
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for RawStdin {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

/// Poll one key without raw mode (line-buffered — usually wrong for demos).
/// Prefer [`RawStdin`].
pub fn poll_key_byte() -> io::Result<Option<u8>> {
    #[cfg(unix)]
    unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 0 {
            return Ok(None);
        }
        let mut fds = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        if libc::poll(&mut fds as *mut _, 1, 0) > 0 && (fds.revents & libc::POLLIN) != 0 {
            let mut buf = [0u8; 8];
            let r = libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, buf.len());
            if r > 0 {
                return Ok(Some(buf[0]));
            }
        }
    }
    Ok(None)
}

/// Present at top-left, full suggested area.
pub fn present(surface: &Surface, backend: TermBackend) -> io::Result<()> {
    present_at(surface, backend, Viewport::full_terminal())
}

/// Tracks cells painted last frame so moved strokes erase cleanly without
/// wiping the whole terminal (true overlay).
#[derive(Debug, Default)]
pub struct OverlayState {
    /// Packed cell keys: (row << 16) | col within the viewport grid.
    prev: Vec<u32>,
}

impl OverlayState {
    pub fn new() -> Self {
        Self { prev: Vec::new() }
    }
}

/// Present inside a cell rectangle. Transparent pixels leave host cells alone.
/// Pass `state` for half-block/ascii so prior stroke cells are erased when the beam moves.
pub fn present_at(surface: &Surface, backend: TermBackend, vp: Viewport) -> io::Result<()> {
    present_at_state(surface, backend, vp, None)
}

/// Like [`present_at`] with erase-tracking for crisp moving strokes over TTY text.
pub fn present_at_state(
    surface: &Surface,
    backend: TermBackend,
    vp: Viewport,
    state: Option<&mut OverlayState>,
) -> io::Result<()> {
    present_at_state_scratch(surface, backend, vp, state, None)
}

/// Present with optional reusable scratch (required for long-running demos).
pub fn present_at_state_scratch(
    surface: &Surface,
    backend: TermBackend,
    vp: Viewport,
    state: Option<&mut OverlayState>,
    scratch: Option<&mut PresentScratch>,
) -> io::Result<()> {
    match backend {
        TermBackend::Kitty => present_kitty_at(surface, vp, scratch),
        TermBackend::HalfBlock => present_halfblock_at(surface, vp, state),
        TermBackend::Ascii => present_ascii_at(surface, vp, state),
    }
}

fn present_kitty_at(
    surface: &Surface,
    vp: Viewport,
    scratch: Option<&mut PresentScratch>,
) -> io::Result<()> {
    let cols = vp.cols.max(1);
    let rows = vp.rows.max(1);
    let w = surface.width();
    let h = surface.height();
    let id = KITTY_ID.load(Ordering::Relaxed);

    let mut local = PresentScratch::default();
    let sc = scratch.unwrap_or(&mut local);
    surface.export_rgba32_into(&mut sc.rgba);

    // Encode base64 into reusable string.
    sc.b64.clear();
    // estimate 4/3
    sc.b64.reserve(sc.rgba.len() * 4 / 3 + 8);
    sc.b64.push_str(&b64_encode(&sc.rgba));

    sc.out.clear();
    sc.out.reserve(sc.b64.len() + 256);
    push_cup(&mut sc.out, vp.row + 1, vp.col + 1);

    // f=32 RGBA; q=2 quiet. Reuse image id so terminal replaces (less queue growth).
    let header = format!("a=T,f=32,t=d,s={w},v={h},c={cols},r={rows},i={id},q=2");
    let chunk = 4096usize;
    let bytes = sc.b64.as_bytes();
    let mut offset = 0;
    let mut first = true;
    while offset < bytes.len() {
        let end = (offset + chunk).min(bytes.len());
        let more = if end < bytes.len() { 1 } else { 0 };
        if first {
            sc.out.extend_from_slice(b"\x1b_G");
            sc.out.extend_from_slice(header.as_bytes());
            sc.out.extend_from_slice(b",m=");
            sc.out.push(if more == 1 { b'1' } else { b'0' });
            sc.out.push(b';');
            sc.out.extend_from_slice(&bytes[offset..end]);
            sc.out.extend_from_slice(b"\x1b\\");
            first = false;
        } else {
            sc.out.extend_from_slice(b"\x1b_Gm=");
            sc.out.push(if more == 1 { b'1' } else { b'0' });
            sc.out.push(b';');
            sc.out.extend_from_slice(&bytes[offset..end]);
            sc.out.extend_from_slice(b"\x1b\\");
        }
        offset = end;
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&sc.out)?;
    stdout.flush()
}

/// Half-block overlay: paint only opaque cells; erase previous stroke cells
/// that are now transparent (so motion stays crisp over TTY text).
fn present_halfblock_at(
    surface: &Surface,
    vp: Viewport,
    state: Option<&mut OverlayState>,
) -> io::Result<()> {
    let w = surface.width() as usize;
    let h = surface.height() as usize;
    let stride = surface.stride() as usize;
    let px = surface.pixels();
    let rows = h.div_ceil(2);

    let mut now: Vec<u32> = Vec::with_capacity(w * 4);
    let mut buf = Vec::with_capacity(rows * w * 48 + 64);

    for row in 0..rows {
        let y0 = row * 2;
        let y1 = y0 + 1;
        for x in 0..w {
            let top = load_px(px, stride, x, y0, w, h);
            let bot = if y1 < h {
                load_px(px, stride, x, y1, w, h)
            } else {
                0
            };
            if alpha_byte(top) == 0 && alpha_byte(bot) == 0 {
                continue;
            }
            let key = ((row as u32) << 16) | (x as u32);
            now.push(key);
            push_cup(&mut buf, vp.row + 1 + row as u16, vp.col + 1 + x as u16);
            let (tr, tg, tb) = unpack_rgb(top);
            let (br, bg, bb) = unpack_rgb(bot);
            let ta = alpha_byte(top);
            let ba = alpha_byte(bot);
            if ta == 0 {
                buf.extend_from_slice(b"\x1b[38;2;");
                push_u8(&mut buf, br);
                buf.push(b';');
                push_u8(&mut buf, bg);
                buf.push(b';');
                push_u8(&mut buf, bb);
                buf.extend_from_slice(b"m\xE2\x96\x84\x1b[0m"); // ▄
            } else if ba == 0 {
                buf.extend_from_slice(b"\x1b[38;2;");
                push_u8(&mut buf, tr);
                buf.push(b';');
                push_u8(&mut buf, tg);
                buf.push(b';');
                push_u8(&mut buf, tb);
                buf.extend_from_slice(b"m\xE2\x96\x80\x1b[0m"); // ▀
            } else {
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
                buf.extend_from_slice(b"m\xE2\x96\x80\x1b[0m");
            }
        }
    }

    // Erase cells that had strokes last frame but are clear now.
    if let Some(st) = state {
        now.sort_unstable();
        for &key in &st.prev {
            if now.binary_search(&key).is_err() {
                let row = (key >> 16) as u16;
                let col = (key & 0xFFFF) as u16;
                push_cup(&mut buf, vp.row + 1 + row, vp.col + 1 + col);
                buf.push(b' ');
            }
        }
        st.prev = now;
    }

    let mut stdout = io::stdout().lock();
    stdout.write_all(&buf)?;
    stdout.flush()
}

#[inline]
fn alpha_byte(c: u32) -> u8 {
    ((c >> 24) & 0xFF) as u8
}

fn present_ascii_at(
    surface: &Surface,
    vp: Viewport,
    state: Option<&mut OverlayState>,
) -> io::Result<()> {
    const RAMP: &[u8] = b" .:-=+*#%@";
    let w = surface.width() as usize;
    let h = surface.height() as usize;
    let stride = surface.stride() as usize;
    let px = surface.pixels();
    let mut now: Vec<u32> = Vec::new();
    let mut buf = Vec::with_capacity(h * w * 24 + 32);

    for y in 0..h {
        for x in 0..w {
            let c = load_px(px, stride, x, y, w, h);
            if alpha_byte(c) == 0 {
                continue;
            }
            now.push(((y as u32) << 16) | (x as u32));
            push_cup(&mut buf, vp.row + 1 + y as u16, vp.col + 1 + x as u16);
            let (r, g, b) = unpack_rgb(c);
            let lum = (r as u32 * 3 + g as u32 * 6 + b as u32) / 10;
            let idx = (lum * (RAMP.len() as u32 - 1) / 255) as usize;
            let ch = RAMP[idx.max(1)];
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
    if let Some(st) = state {
        now.sort_unstable();
        for &key in &st.prev {
            if now.binary_search(&key).is_err() {
                let row = (key >> 16) as u16;
                let col = (key & 0xFFFF) as u16;
                push_cup(&mut buf, vp.row + 1 + row, vp.col + 1 + col);
                buf.push(b' ');
            }
        }
        st.prev = now;
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
