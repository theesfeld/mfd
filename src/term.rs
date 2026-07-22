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
    if let Some((c, r, _, _)) = terminal_winsize() {
        return (c, r);
    }
    if let (Ok(c), Ok(r)) = (std::env::var("COLUMNS"), std::env::var("LINES")) {
        if let (Ok(c), Ok(r)) = (c.parse::<u16>(), r.parse::<u16>()) {
            if c > 0 && r > 0 {
                return (c, r);
            }
        }
    }
    (80, 24)
}

/// `(cols, rows, pixel_w, pixel_h)` from `TIOCGWINSZ` when available.
/// `pixel_*` may be 0 on some terminals.
fn terminal_winsize() -> Option<(u16, u16, u16, u16)> {
    #[cfg(unix)]
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
            && ws.ws_col > 0
            && ws.ws_row > 0
        {
            return Some((ws.ws_col, ws.ws_row, ws.ws_xpixel, ws.ws_ypixel));
        }
    }
    let _ = ();
    None
}

/// Approximate pixel size of one character cell `(width, height)`.
///
/// Prefer real `ws_xpixel`/`ws_ypixel`. Fallback **1∶2** (common mono fonts).
pub fn cell_pixel_size() -> (f32, f32) {
    if let Some((cols, rows, xpix, ypix)) = terminal_winsize() {
        if xpix > 0 && ypix > 0 && cols > 0 && rows > 0 {
            return (xpix as f32 / cols as f32, ypix as f32 / rows as f32);
        }
    }
    // Typical terminal cell: half as wide as tall → N×N cells look *tall*.
    (8.0, 16.0)
}

fn max_pixels() -> (u32, u32) {
    // Allow true 4" on hi-DPI (~190 ppi → ~760 px; 220 ppi → ~880). Default 1024.
    let mw = std::env::var("MFD_MAX_W")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024u32);
    let mh = std::env::var("MFD_MAX_H")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024u32);
    (mw.max(64), mh.max(64))
}

/// Pixel surface size for a viewport (capped for present speed).
pub fn surface_size_for_viewport(backend: TermBackend, vp: Viewport) -> (u32, u32) {
    let (mw, mh) = max_pixels();
    let cols = vp.cols.max(1) as u32;
    let rows = vp.rows.max(1) as u32;
    let (w, h) = match backend {
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

/// Default physical face size (inches). F-16 MLU color MFD ≈ **4×4 in**.
pub const MFD_FACE_INCHES_DEFAULT: f32 = 4.0;

/// Face size in inches from `MFD_FACE_IN` or default 4.0.
pub fn mfd_face_inches() -> f32 {
    std::env::var("MFD_FACE_IN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(MFD_FACE_INCHES_DEFAULT)
        .clamp(1.0, 12.0)
}

/// How PPI was obtained (for logs / calibration).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PpiSource {
    Env,
    EdidDetailed,
    EdidCm,
    Fallback96,
}

/// Detect display **pixels per inch** (PPI) for ruler sizing.
///
/// Order:
/// 1. `MFD_PPI` — manual ruler calibration (always wins)
/// 2. DRM EDID detailed timing size in **mm** (best automatic)
/// 3. DRM EDID screen size in **cm** (coarser)
/// 4. Fallback **96** (not ruler-accurate — calibrate with `MFD_PPI`)
pub fn display_ppi() -> f32 {
    display_ppi_info().0
}

/// `(ppi, source)` for diagnostics.
pub fn display_ppi_info() -> (f32, PpiSource) {
    if let Ok(s) = std::env::var("MFD_PPI") {
        if let Ok(v) = s.parse::<f32>() {
            if v.is_finite() && (40.0..600.0).contains(&v) {
                return (v, PpiSource::Env);
            }
        }
    }
    if let Some(ppi) = ppi_from_drm_edid(true) {
        return (ppi, PpiSource::EdidDetailed);
    }
    if let Some(ppi) = ppi_from_drm_edid(false) {
        return (ppi, PpiSource::EdidCm);
    }
    (96.0, PpiSource::Fallback96)
}

/// Unified **ruler layout**: one side length drives both framebuffer and viewport.
#[derive(Clone, Debug)]
pub struct PhysicalFace {
    /// Requested edge length (inches).
    pub inches_requested: f32,
    /// PPI used for the calculation.
    pub ppi: f32,
    pub ppi_source: PpiSource,
    /// Framebuffer side (1∶1), after clamps.
    pub side_px: u32,
    /// Present cell box (aspect-corrected).
    pub viewport: Viewport,
    /// On-glass edge after integer cell snap (inches).
    pub on_glass_in: f32,
    /// True if terminal/max caps forced a smaller face than requested.
    pub clipped: bool,
}

impl PhysicalFace {
    /// Compute a ruler-accurate square face for this host + backend.
    pub fn layout(backend: TermBackend, inches: f32) -> Self {
        let inches = inches.clamp(1.0, 12.0);
        let (ppi, ppi_source) = display_ppi_info();
        let (cw, ch) = cell_pixel_size();
        let (tc, tr) = terminal_cells();

        // Ideal screen pixels for N inches.
        let want = inches * ppi;

        // Largest square that fits the terminal window (device pixels).
        let fit = (tc as f32 * cw).min(tr as f32 * ch).max(64.0);

        // Present payload cap (still allow real 4" on hi-DPI; default 1024).
        let (mw, mh) = max_pixels();
        let cap = mw.min(mh).max(64) as f32;

        let mut side_f = want.min(fit).min(cap);
        // Backend soft caps (ascii is for CI only — not ruler-accurate).
        side_f = match backend {
            TermBackend::Ascii => side_f.min(160.0),
            TermBackend::HalfBlock => side_f.min(640.0),
            TermBackend::Kitty => side_f,
        };
        let side_px = (side_f.round() as u32).clamp(128, 4096);
        let clipped = side_f + 0.5 < want;

        // Viewport must show the **same** side on glass (same pixel side).
        let (cols, rows) = cells_for_screen_square(tc, tr, cw, ch, side_px as f32);
        let col = tc.saturating_sub(cols) / 2;
        let row = tr.saturating_sub(rows) / 2;
        let viewport = Viewport {
            col,
            row,
            cols,
            rows,
        };

        let vis_w = cols as f32 * cw;
        let vis_h = rows as f32 * ch;
        let on_glass_px = vis_w.min(vis_h);
        let on_glass_in = on_glass_px / ppi;

        Self {
            inches_requested: inches,
            ppi,
            ppi_source,
            side_px,
            viewport,
            on_glass_in,
            clipped,
        }
    }

    pub fn surface_size(&self) -> (u32, u32) {
        (self.side_px, self.side_px)
    }
}

/// Square pixel surface sized to physical inches (uses [`PhysicalFace::layout`]).
pub fn square_mfd_pixels(backend: TermBackend) -> (u32, u32) {
    PhysicalFace::layout(backend, mfd_face_inches()).surface_size()
}

/// Cell viewport for physical inches (uses [`PhysicalFace::layout`]).
pub fn square_mfd_viewport(_frac: f32) -> Viewport {
    PhysicalFace::layout(detect_backend(), mfd_face_inches()).viewport
}

/// Explicit inches + backend (preferred for demos).
pub fn physical_mfd_layout(backend: TermBackend, inches: f32) -> PhysicalFace {
    PhysicalFace::layout(backend, inches)
}

/// Read connected DRM EDID → PPI.
/// `prefer_detailed`: use detailed-timing mm when present (more accurate than cm fields).
fn ppi_from_drm_edid(prefer_detailed: bool) -> Option<f32> {
    let drm = std::path::Path::new("/sys/class/drm");
    let entries = std::fs::read_dir(drm).ok()?;
    let mut best: Option<f32> = None;
    for ent in entries.flatten() {
        let path = ent.path();
        let name = path.file_name()?.to_string_lossy();
        if !name.contains('-') {
            continue;
        }
        let status = std::fs::read_to_string(path.join("status")).ok()?;
        if !status.trim().eq_ignore_ascii_case("connected") {
            continue;
        }
        let edid = std::fs::read(path.join("edid")).ok()?;
        let modes = std::fs::read_to_string(path.join("modes")).ok()?;
        let line = modes.lines().next()?;
        let (rw, rh) = parse_mode_wh(line)?;

        let ppi = if prefer_detailed {
            edid_ppi_detailed(&edid, rw, rh)
        } else {
            edid_ppi_cm(&edid, rw, rh)
        };
        if let Some(p) = ppi {
            best = Some(match best {
                Some(b) => b.max(p), // prefer denser / primary often listed first; take max valid
                None => p,
            });
            // Prefer first connected with a good detailed size
            if prefer_detailed {
                return Some(p);
            }
        }
    }
    best
}

fn edid_ppi_cm(edid: &[u8], mode_w: u32, mode_h: u32) -> Option<f32> {
    if edid.len() < 0x17 {
        return None;
    }
    let h_cm = edid[0x15] as f32;
    let v_cm = edid[0x16] as f32;
    if h_cm < 5.0 || v_cm < 5.0 {
        return None;
    }
    let ppi_w = mode_w as f32 / (h_cm / 2.54);
    let ppi_h = mode_h as f32 / (v_cm / 2.54);
    let ppi = (ppi_w + ppi_h) * 0.5;
    valid_ppi(ppi)
}

/// Detailed timing descriptors (18-byte blocks at 54,72,90,108) can store size in mm.
fn edid_ppi_detailed(edid: &[u8], mode_w: u32, mode_h: u32) -> Option<f32> {
    if edid.len() < 126 {
        return None;
    }
    for base in [54, 72, 90, 108] {
        if base + 17 >= edid.len() {
            break;
        }
        // Monitor descriptors have pixel clock = 0.
        if edid[base] == 0 && edid[base + 1] == 0 {
            continue;
        }
        // Horizontal image size mm: low 8 @ +12, high 4 in +14[7:4]
        // Vertical image size mm: low 8 @ +13, high 4 in +14[3:0]
        let h_mm = edid[base + 12] as u16 | (((edid[base + 14] as u16) & 0xF0) << 4);
        let v_mm = edid[base + 13] as u16 | (((edid[base + 14] as u16) & 0x0F) << 8);
        if h_mm < 50 || v_mm < 50 {
            continue;
        }
        let ppi_w = mode_w as f32 / (h_mm as f32 / 25.4);
        let ppi_h = mode_h as f32 / (v_mm as f32 / 25.4);
        if let Some(p) = valid_ppi((ppi_w + ppi_h) * 0.5) {
            return Some(p);
        }
    }
    None
}

fn valid_ppi(ppi: f32) -> Option<f32> {
    if ppi.is_finite() && (50.0..500.0).contains(&ppi) {
        Some(ppi)
    } else {
        None
    }
}

fn parse_mode_wh(s: &str) -> Option<(u32, u32)> {
    let s = s.trim();
    let (a, b) = s.split_once('x')?;
    // modes may be "2560x1600i" etc.
    let b = b.trim_end_matches(|c: char| !c.is_ascii_digit());
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Cell counts so the **on-screen** box is `side_px`×`side_px` (aspect-correct).
pub fn cells_for_screen_square(
    term_cols: u16,
    term_rows: u16,
    cell_w: f32,
    cell_h: f32,
    side_px: f32,
) -> (u16, u16) {
    let tc = term_cols.max(1) as i32;
    let tr = term_rows.max(1) as i32;
    let cw = cell_w.max(1.0);
    let ch = cell_h.max(1.0);
    let side = side_px.max(1.0);

    let mut cols = (side / cw).round() as i32;
    let mut rows = (side / ch).round() as i32;
    cols = cols.clamp(8, tc);
    rows = rows.clamp(4, tr);

    // Equalize visual width/height after integer snap.
    let vis_w = cols as f32 * cw;
    let vis_h = rows as f32 * ch;
    if vis_w > vis_h + cw * 0.25 {
        cols = ((vis_h / cw).round() as i32).clamp(8, tc);
    } else if vis_h > vis_w + ch * 0.25 {
        rows = ((vis_w / ch).round() as i32).clamp(4, tr);
    }
    (cols as u16, rows as u16)
}

/// Pure layout helper (tests / callers): square cells for a fraction of the TTY.
pub fn visual_square_cells(
    term_cols: u16,
    term_rows: u16,
    cell_w: f32,
    cell_h: f32,
    frac: f32,
) -> (u16, u16) {
    let tc = term_cols.max(1) as f32;
    let tr = term_rows.max(1) as f32;
    let f = frac.clamp(0.4, 1.0);
    let side = (tc * cell_w.max(1.0) * f).min(tr * cell_h.max(1.0) * f);
    cells_for_screen_square(term_cols, term_rows, cell_w, cell_h, side)
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

    #[test]
    fn visual_square_uses_more_cols_when_cells_are_tall() {
        // 8×16 px cells → need ~2× cols as rows for a square.
        let (cols, rows) = visual_square_cells(200, 60, 8.0, 16.0, 0.9);
        assert!(
            cols > rows,
            "cols={cols} should exceed rows={rows} for 1:2 cells"
        );
        let vis_w = cols as f32 * 8.0;
        let vis_h = rows as f32 * 16.0;
        let err = (vis_w - vis_h).abs() / vis_w.max(vis_h);
        assert!(
            err < 0.15,
            "visual aspect error {err} (w={vis_w} h={vis_h})"
        );
    }

    #[test]
    fn physical_4in_at_96dpi() {
        // 4" × 96 ppi = 384 px side before clamps.
        let side = (4.0_f32 * 96.0).round() as u32;
        assert_eq!(side, 384);
        let (cols, rows) = cells_for_screen_square(120, 40, 8.0, 16.0, 384.0);
        let w = cols as f32 * 8.0;
        let h = rows as f32 * 16.0;
        assert!((w - h).abs() / w.max(h) < 0.12, "w={w} h={h}");
    }

    #[test]
    fn layout_framebuffer_matches_requested_when_room() {
        // Huge terminal, known ppi via env is tested indirectly: cells for 764px.
        let side = 765.0_f32;
        let (cols, rows) = cells_for_screen_square(300, 100, 8.0, 16.0, side);
        let w = cols as f32 * 8.0;
        let h = rows as f32 * 16.0;
        assert!((w - h).abs() < 20.0, "aspect w={w} h={h}");
        assert!((w - side).abs() < side * 0.08, "size w={w} want≈{side}");
    }
}
