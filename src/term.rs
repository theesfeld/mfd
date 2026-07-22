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
///
/// **Note:** On Ghostty/Kitty under fractional Wayland scale, these values are
/// often **buffer** pixels, not panel device pixels. Use
/// [`cell_pixel_size_device`] for ruler layout.
pub fn cell_pixel_size() -> (f32, f32) {
    if let Some((cols, rows, xpix, ypix)) = terminal_winsize() {
        if xpix > 0 && ypix > 0 && cols > 0 && rows > 0 {
            return (xpix as f32 / cols as f32, ypix as f32 / rows as f32);
        }
    }
    // Typical terminal cell: half as wide as tall → N×N cells look *tall*.
    (8.0, 16.0)
}

/// How `TIOCGWINSZ` pixels relate to **panel device pixels**.
///
/// Ghostty (and some other GPU terminals) report buffer size at their content
/// scale. Compositor scale may differ (e.g. content ~2, niri scale 1.5). EDID
/// PPI is always in panel device pixels. Mixing spaces understates the face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelSpace {
    /// Multiply winsize/buffer pixels by this to get panel device pixels.
    pub winsize_to_device: f32,
    pub source: PxSpaceSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PxSpaceSource {
    /// `MFD_PX_SCALE` env (device px per winsize px).
    Env,
    /// Compositor window size × output scale vs winsize.
    Compositor,
    /// No correction (assume winsize already device).
    Identity,
}

/// Detect winsize → device pixel scale.
///
/// Order:
/// 1. `MFD_PX_SCALE` — manual (device_px = winsize_px × scale)
/// 2. Compositor: window logical size × output scale / winsize
/// 3. Identity `1.0`
pub fn pixel_space() -> PixelSpace {
    if let Ok(s) = std::env::var("MFD_PX_SCALE") {
        if let Ok(v) = s.parse::<f32>() {
            if v.is_finite() && (0.25..4.0).contains(&v) {
                return PixelSpace {
                    winsize_to_device: v,
                    source: PxSpaceSource::Env,
                };
            }
        }
    }
    if let Some(corr) = compositor_winsize_to_device() {
        return PixelSpace {
            winsize_to_device: corr,
            source: PxSpaceSource::Compositor,
        };
    }
    PixelSpace {
        winsize_to_device: 1.0,
        source: PxSpaceSource::Identity,
    }
}

/// Cell size in **panel device pixels** (same space as EDID PPI).
pub fn cell_pixel_size_device() -> (f32, f32) {
    let (cw, ch) = cell_pixel_size();
    let s = pixel_space().winsize_to_device;
    (cw * s, ch * s)
}

/// Match this process's terminal window on the compositor; return
/// `device_w / winsize_xpixel` (and average with height when both valid).
fn compositor_winsize_to_device() -> Option<f32> {
    let (_, _, xpix, ypix) = terminal_winsize()?;
    if xpix == 0 || ypix == 0 {
        return None;
    }
    let (dev_w, dev_h) = window_device_size_px()?;
    let mut parts = Vec::with_capacity(2);
    if xpix > 32 {
        let r = dev_w / xpix as f32;
        if (0.25..4.0).contains(&r) {
            parts.push(r);
        }
    }
    if ypix > 32 {
        let r = dev_h / ypix as f32;
        if (0.25..4.0).contains(&r) {
            parts.push(r);
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.iter().sum::<f32>() / parts.len() as f32)
}

/// Terminal window size in panel device pixels `(w, h)`.
fn window_device_size_px() -> Option<(f32, f32)> {
    let pids = process_ancestor_pids();
    if let Some(v) = niri_window_device_size(&pids) {
        return Some(v);
    }
    if let Some(v) = hypr_window_device_size(&pids) {
        return Some(v);
    }
    if let Some(v) = sway_window_device_size(&pids) {
        return Some(v);
    }
    None
}

fn process_ancestor_pids() -> Vec<u32> {
    let mut out = Vec::new();
    let mut pid = std::process::id();
    out.push(pid);
    for _ in 0..32 {
        let path = format!("/proc/{pid}/status");
        let Ok(text) = std::fs::read_to_string(&path) else {
            break;
        };
        let mut ppid = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("PPid:") {
                ppid = rest.trim().parse().ok();
                break;
            }
        }
        let Some(p) = ppid else { break };
        if p <= 1 || out.contains(&p) {
            break;
        }
        out.push(p);
        pid = p;
    }
    out
}

fn niri_window_device_size(pids: &[u32]) -> Option<(f32, f32)> {
    let host = niri_host_display(pids)?;
    let (lw, lh) = host.window_logical?;
    let scale = host.scale.clamp(0.5, 4.0);
    Some((lw * scale, lh * scale))
}

/// niri: window → workspace → output (current mode + physical mm + scale).
fn niri_host_display(pids: &[u32]) -> Option<HostDisplay> {
    let wins = cmd_stdout(&["niri", "msg", "--json", "windows"])?;
    let outs = cmd_stdout(&["niri", "msg", "--json", "outputs"])?;
    let spaces = cmd_stdout(&["niri", "msg", "--json", "workspaces"]).unwrap_or_default();

    let (ws_id, win_w, win_h) = niri_pick_window_ws(&wins, pids)?;
    let out_name =
        niri_workspace_output(&spaces, ws_id).or_else(|| niri_focused_output_name(&outs));
    let mut host = niri_parse_output(&outs, out_name.as_deref())?;
    host.window_logical = Some((win_w, win_h));
    Some(host)
}

fn niri_pick_window_ws(json: &str, pids: &[u32]) -> Option<(u64, f32, f32)> {
    let mut best: Option<(u64, f32, f32)> = None;
    let bytes = json.as_bytes();
    let mut i = 0;
    while i + 8 < bytes.len() {
        let Some(rel) = find_subslice(&bytes[i..], b"\"pid\"") else {
            break;
        };
        i += rel;
        let Some(pid_f) = json_number_after(&json[i..], "\"pid\"") else {
            i += 4;
            continue;
        };
        let pid = pid_f as u32;
        let start = json[..i].rfind('{').unwrap_or(i);
        let slice = &json[start..json.len().min(start + 2000)];
        let ws = json_number_after(slice, "\"workspace_id\"").unwrap_or(0.0) as u64;
        if let Some((w, h)) = json_window_size_pair(slice) {
            if pids.contains(&pid) && w > 32.0 && h > 32.0 {
                return Some((ws, w, h));
            }
            if best.is_none() && w > 32.0 && h > 32.0 {
                best = Some((ws, w, h));
            }
        }
        i += 4;
    }
    best
}

fn niri_workspace_output(spaces: &str, ws_id: u64) -> Option<String> {
    if spaces.is_empty() || ws_id == 0 {
        return None;
    }
    let bytes = spaces.as_bytes();
    let mut i = 0;
    while i + 6 < bytes.len() {
        let Some(rel) = find_subslice(&bytes[i..], b"\"id\"") else {
            break;
        };
        i += rel;
        let start = spaces[..i].rfind('{').unwrap_or(i);
        let slice = &spaces[start..spaces.len().min(start + 400)];
        let id = match json_number_after(slice, "\"id\"") {
            Some(v) => v as u64,
            None => {
                i += 4;
                continue;
            }
        };
        if id == ws_id {
            if let Some(name) = json_string_after(slice, "\"output\"") {
                return Some(name);
            }
        }
        i += 4;
    }
    None
}

fn niri_focused_output_name(outs: &str) -> Option<String> {
    if let Some(text) = cmd_stdout(&["niri", "msg", "--json", "focused-output"]) {
        if let Some(n) = json_string_after(&text, "\"name\"") {
            return Some(n);
        }
    }
    niri_prefer_output_name(outs)
}

fn niri_prefer_output_name(outs: &str) -> Option<String> {
    let mut names = Vec::new();
    let mut i = 0;
    let bytes = outs.as_bytes();
    while i + 4 < bytes.len() {
        if let Some(rel) = find_subslice(&bytes[i..], b"\"name\"") {
            i += rel;
            if let Some(n) = json_string_after(&outs[i..], "\"name\"") {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            i += 4;
        } else {
            break;
        }
    }
    names
        .iter()
        .find(|n| !n.starts_with("eDP"))
        .cloned()
        .or_else(|| names.first().cloned())
}

fn niri_parse_output(outs: &str, prefer: Option<&str>) -> Option<HostDisplay> {
    let mut names = Vec::new();
    if let Some(p) = prefer {
        names.push(p.to_string());
    }
    let mut i = 0;
    let bytes = outs.as_bytes();
    while i + 8 < bytes.len() {
        if let Some(rel) = find_subslice(&bytes[i..], b"\"name\"") {
            i += rel;
            if let Some(n) = json_string_after(&outs[i..], "\"name\"") {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            i += 4;
        } else {
            break;
        }
    }
    for name in &names {
        if let Some(h) = niri_parse_one_output(outs, name) {
            return Some(h);
        }
    }
    None
}

fn niri_parse_one_output(outs: &str, name: &str) -> Option<HostDisplay> {
    let needle = format!("\"name\":\"{name}\"");
    let idx = outs
        .find(&needle)
        .or_else(|| outs.find(&format!("\"name\": \"{name}\"")))?;
    let start = outs[..idx].rfind('{').unwrap_or(idx);
    let slice = &outs[start..outs.len().min(start + 12000)];
    let (phys_w, phys_h) = json_size_array(slice, "\"physical_size\"")?;
    let cur = json_number_after(slice, "\"current_mode\"").unwrap_or(0.0) as usize;
    let (mode_w, mode_h) = niri_mode_at(slice, cur).or_else(|| niri_mode_at(slice, 0))?;
    let scale = json_number_after(slice, "\"scale\"")
        .map(|v| v as f32)
        .unwrap_or(1.0)
        .clamp(0.5, 4.0);
    Some(HostDisplay {
        name: name.to_string(),
        mode_w,
        mode_h,
        phys_w_mm: phys_w,
        phys_h_mm: phys_h,
        scale,
        window_logical: None,
    })
}

fn niri_mode_at(slice: &str, index: usize) -> Option<(u32, u32)> {
    let modes_idx = slice.find("\"modes\"")?;
    let rest = &slice[modes_idx..];
    let arr = rest.find('[')?;
    let mut body = &rest[arr + 1..];
    let mut i = 0usize;
    while let Some(obj_start) = body.find('{') {
        body = &body[obj_start..];
        let end = body.find('}').unwrap_or(body.len().min(200));
        let obj = &body[..end];
        if i == index {
            let w = json_number_after(obj, "\"width\"")? as u32;
            let h = json_number_after(obj, "\"height\"")? as u32;
            if w >= 320 && h >= 200 {
                return Some((w, h));
            }
            return None;
        }
        i += 1;
        body = &body[end.saturating_add(1)..];
        if i > 64 {
            break;
        }
    }
    None
}

fn hypr_window_device_size(pids: &[u32]) -> Option<(f32, f32)> {
    let host = hypr_host_display(pids)?;
    let (lw, lh) = host.window_logical?;
    let scale = host.scale.clamp(0.5, 4.0);
    Some((lw * scale, lh * scale))
}

fn hypr_host_display(pids: &[u32]) -> Option<HostDisplay> {
    let clients = cmd_stdout(&["hyprctl", "clients", "-j"])?;
    let mon = cmd_stdout(&["hyprctl", "monitors", "-j"])?;
    let mut mon_name: Option<String> = None;
    let mut win: Option<(f32, f32)> = None;
    let mut i = 0;
    let bytes = clients.as_bytes();
    while i + 8 < bytes.len() {
        if let Some(rel) = find_subslice(&bytes[i..], b"\"pid\"") {
            i += rel;
            let start = clients[..i].rfind('{').unwrap_or(i);
            let slice = &clients[start..clients.len().min(start + 1500)];
            let pid = match json_number_after(slice, "\"pid\"") {
                Some(v) => v as u32,
                None => {
                    i += 4;
                    continue;
                }
            };
            if pids.contains(&pid) {
                if let Some((w, h)) = json_size_array(slice, "\"size\"") {
                    if w > 32.0 && h > 32.0 {
                        win = Some((w, h));
                        mon_name = json_string_after(slice, "\"monitor\"");
                        break;
                    }
                }
            }
            i += 4;
        } else {
            break;
        }
    }
    let (win_w, win_h) = win?;
    let mut host = hypr_parse_monitor(&mon, mon_name.as_deref())?;
    host.window_logical = Some((win_w, win_h));
    Some(host)
}

fn hypr_parse_monitor(mon: &str, prefer: Option<&str>) -> Option<HostDisplay> {
    let bytes = mon.as_bytes();
    let mut i = 0;
    let mut first: Option<HostDisplay> = None;
    while i + 8 < bytes.len() {
        let Some(rel) = find_subslice(&bytes[i..], b"\"name\"") else {
            break;
        };
        i += rel;
        let start = mon[..i].rfind('{').unwrap_or(i);
        let slice = &mon[start..mon.len().min(start + 2500)];
        let name = match json_string_after(slice, "\"name\"") {
            Some(n) => n,
            None => {
                i += 4;
                continue;
            }
        };
        let mode_w = json_number_after(slice, "\"width\"").unwrap_or(0.0) as u32;
        let mode_h = json_number_after(slice, "\"height\"").unwrap_or(0.0) as u32;
        let phys_w = json_number_after(slice, "\"widthMM\"")
            .or_else(|| json_number_after(slice, "\"width_mm\""))
            .unwrap_or(0.0) as f32;
        let phys_h = json_number_after(slice, "\"heightMM\"")
            .or_else(|| json_number_after(slice, "\"height_mm\""))
            .unwrap_or(0.0) as f32;
        let scale = json_number_after(slice, "\"scale\"")
            .map(|v| v as f32)
            .unwrap_or(1.0)
            .clamp(0.5, 4.0);
        if mode_w >= 320 && mode_h >= 200 {
            let h = HostDisplay {
                name: name.clone(),
                mode_w,
                mode_h,
                phys_w_mm: phys_w,
                phys_h_mm: phys_h,
                scale,
                window_logical: None,
            };
            if prefer.map(|p| p == name).unwrap_or(false) {
                return Some(h);
            }
            if first.is_none() {
                first = Some(h);
            }
        }
        i += 4;
    }
    first
}

fn sway_window_device_size(pids: &[u32]) -> Option<(f32, f32)> {
    let host = sway_host_display(pids)?;
    let (lw, lh) = host.window_logical?;
    let scale = host.scale.clamp(0.5, 4.0);
    Some((lw * scale, lh * scale))
}

fn sway_host_display(pids: &[u32]) -> Option<HostDisplay> {
    let tree = cmd_stdout(&["swaymsg", "-t", "get_tree"])?;
    let outs = cmd_stdout(&["swaymsg", "-t", "get_outputs"])?;
    let mut i = 0;
    let bytes = tree.as_bytes();
    let mut win: Option<(f32, f32)> = None;
    let mut out_name: Option<String> = None;
    while i + 8 < bytes.len() {
        if let Some(rel) = find_subslice(&bytes[i..], b"\"pid\"") {
            i += rel;
            let start = tree[..i].rfind('{').unwrap_or(i);
            let slice = &tree[start..tree.len().min(start + 2000)];
            let pid = match json_number_after(slice, "\"pid\"") {
                Some(v) => v as u32,
                None => {
                    i += 4;
                    continue;
                }
            };
            if pids.contains(&pid) {
                if let Some((w, h)) = json_rect_wh(slice) {
                    if w > 32.0 && h > 32.0 {
                        win = Some((w, h));
                        out_name = json_string_after(slice, "\"output\"");
                        break;
                    }
                }
            }
            i += 4;
        } else {
            break;
        }
    }
    let (win_w, win_h) = win?;
    let mut host = sway_parse_output(&outs, out_name.as_deref())?;
    host.window_logical = Some((win_w, win_h));
    Some(host)
}

fn sway_parse_output(outs: &str, prefer: Option<&str>) -> Option<HostDisplay> {
    let bytes = outs.as_bytes();
    let mut i = 0;
    let mut first: Option<HostDisplay> = None;
    while i + 8 < bytes.len() {
        let Some(rel) = find_subslice(&bytes[i..], b"\"name\"") else {
            break;
        };
        i += rel;
        let start = outs[..i].rfind('{').unwrap_or(i);
        let slice = &outs[start..outs.len().min(start + 4000)];
        let name = match json_string_after(slice, "\"name\"") {
            Some(n) => n,
            None => {
                i += 4;
                continue;
            }
        };
        let active = json_bool_after(slice, "\"active\"").unwrap_or(true);
        if !active {
            i += 4;
            continue;
        }
        let (mode_w, mode_h) = if let Some(cm) = slice.find("\"current_mode\"") {
            let sub = &slice[cm..slice.len().min(cm + 300)];
            let w = json_number_after(sub, "\"width\"").unwrap_or(0.0) as u32;
            let h = json_number_after(sub, "\"height\"").unwrap_or(0.0) as u32;
            (w, h)
        } else {
            (0, 0)
        };
        let phys_w = json_number_after(slice, "\"physical_width\"")
            .or_else(|| json_number_after(slice, "\"physical_width_mm\""))
            .unwrap_or(0.0) as f32;
        let phys_h = json_number_after(slice, "\"physical_height\"")
            .or_else(|| json_number_after(slice, "\"physical_height_mm\""))
            .unwrap_or(0.0) as f32;
        let scale = json_number_after(slice, "\"scale\"")
            .map(|v| v as f32)
            .unwrap_or(1.0)
            .clamp(0.5, 4.0);
        if mode_w >= 320 && mode_h >= 200 {
            let h = HostDisplay {
                name: name.clone(),
                mode_w,
                mode_h,
                phys_w_mm: phys_w,
                phys_h_mm: phys_h,
                scale,
                window_logical: None,
            };
            if prefer.map(|p| p == name).unwrap_or(false) {
                return Some(h);
            }
            if first.is_none() {
                first = Some(h);
            }
        }
        i += 4;
    }
    first
}

/// Compositor report for the output that hosts this process's terminal window.
#[derive(Clone, Debug)]
struct HostDisplay {
    name: String,
    mode_w: u32,
    mode_h: u32,
    phys_w_mm: f32,
    phys_h_mm: f32,
    scale: f32,
    window_logical: Option<(f32, f32)>,
}

impl HostDisplay {
    fn ppi(&self) -> Option<f32> {
        ppi_from_physical_mode(self.phys_w_mm, self.phys_h_mm, self.mode_w, self.mode_h)
    }
}

fn host_display() -> Option<HostDisplay> {
    let pids = process_ancestor_pids();
    if let Some(h) = niri_host_display(&pids) {
        return Some(h);
    }
    if let Some(h) = hypr_host_display(&pids) {
        return Some(h);
    }
    if let Some(h) = sway_host_display(&pids) {
        return Some(h);
    }
    None
}

/// PPI from panel mm size + **current** mode pixels (device pixels / inch).
fn ppi_from_physical_mode(phys_w_mm: f32, phys_h_mm: f32, mode_w: u32, mode_h: u32) -> Option<f32> {
    if phys_w_mm < 50.0 || phys_h_mm < 50.0 || mode_w < 320 || mode_h < 200 {
        return None;
    }
    let ppi_w = mode_w as f32 / (phys_w_mm / 25.4);
    let ppi_h = mode_h as f32 / (phys_h_mm / 25.4);
    valid_ppi((ppi_w + ppi_h) * 0.5)
}

fn cmd_stdout(argv: &[&str]) -> Option<String> {
    let (prog, args) = argv.split_first()?;
    let out = std::process::Command::new(prog)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn json_number_after(s: &str, key: &str) -> Option<f64> {
    let idx = s.find(key)?;
    let rest = s[idx + key.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn json_string_after(s: &str, key: &str) -> Option<String> {
    let idx = s.find(key)?;
    let rest = s[idx + key.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn json_bool_after(s: &str, key: &str) -> Option<bool> {
    let idx = s.find(key)?;
    let rest = s[idx + key.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

#[allow(dead_code)]
fn json_first_f32(s: &str, key: &str) -> Option<f32> {
    json_number_after(s, key).map(|v| v as f32)
}

fn json_window_size_pair(s: &str) -> Option<(f32, f32)> {
    json_size_array(s, "\"window_size\"")
}

fn json_size_array(s: &str, key: &str) -> Option<(f32, f32)> {
    let idx = s.find(key)?;
    let rest = s[idx + key.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('[')?.trim_start();
    let (a, rest) = split_num(rest)?;
    let rest = rest.trim_start().strip_prefix(',')?.trim_start();
    let (b, _) = split_num(rest)?;
    Some((a as f32, b as f32))
}

fn json_rect_wh(s: &str) -> Option<(f32, f32)> {
    let idx = s.find("\"rect\"")?;
    let slice = &s[idx..s.len().min(idx + 200)];
    let w = json_number_after(slice, "\"width\"")? as f32;
    let h = json_number_after(slice, "\"height\"")? as f32;
    Some((w, h))
}

fn split_num(s: &str) -> Option<(f64, &str)> {
    let end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+')
        .unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    let n = s[..end].parse().ok()?;
    Some((n, &s[end..]))
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
    /// Compositor: current mode × physical size for the **host** output.
    Compositor,
    EdidDetailed,
    EdidCm,
    Fallback96,
}

/// Detect display **pixels per inch** (PPI) for ruler sizing.
///
/// Order:
/// 1. `MFD_PPI` — manual ruler calibration (always wins)
/// 2. Compositor host output: **current** mode × physical mm (niri / Hypr / sway)
/// 3. DRM EDID detailed timing size in **mm** + best mode match
/// 4. DRM EDID screen size in **cm** + best mode match
/// 5. Fallback **96** (not ruler-accurate — calibrate with `MFD_PPI`)
///
/// Multi-monitor: uses the output that hosts this terminal window when the
/// compositor reports it. Never takes first connector or first EDID mode alone —
/// those are wrong on ultrawides (preferred ≠ current) and dual-head
/// (laptop eDP PPI ≠ external panel).
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
    let host = host_display();
    if let Some(ref h) = host {
        if let Some(ppi) = h.ppi() {
            return (ppi, PpiSource::Compositor);
        }
    }
    let prefer_conn = host.as_ref().map(|h| h.name.as_str());
    if let Some(ppi) = ppi_from_drm_edid(true, prefer_conn) {
        return (ppi, PpiSource::EdidDetailed);
    }
    if let Some(ppi) = ppi_from_drm_edid(false, prefer_conn) {
        return (ppi, PpiSource::EdidCm);
    }
    (96.0, PpiSource::Fallback96)
}

/// Unified **ruler layout**: one side length drives both framebuffer and viewport.
#[derive(Clone, Debug)]
pub struct PhysicalFace {
    /// Requested edge length (inches).
    pub inches_requested: f32,
    /// PPI used for the calculation (panel device pixels / inch).
    pub ppi: f32,
    pub ppi_source: PpiSource,
    /// Winsize → device correction applied to cell size.
    pub pixel_space: PixelSpace,
    /// Cell size in panel device pixels `(w, h)`.
    pub cell_device: (f32, f32),
    /// Framebuffer side (1∶1), after clamps — device-pixel resolution.
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
    ///
    /// All length math uses **panel device pixels** (EDID PPI space). Cell
    /// sizes from `TIOCGWINSZ` are converted via [`pixel_space`].
    pub fn layout(backend: TermBackend, inches: f32) -> Self {
        let inches = inches.clamp(1.0, 12.0);
        let (ppi, ppi_source) = display_ppi_info();
        let px = pixel_space();
        let (cw, ch) = cell_pixel_size_device();
        let (tc, tr) = terminal_cells();

        // Ideal panel device pixels for N inches.
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

        // Viewport must show the **same** side on glass (device-pixel side).
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
            pixel_space: px,
            cell_device: (cw, ch),
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
///
/// `prefer_detailed`: use detailed-timing mm when present (more accurate than cm).
/// `prefer_connector`: connector suffix such as `HDMI-A-1` (from compositor).
///
/// Mode selection (never blind `modes` line 0 alone):
/// 1. Compositor current mode for the preferred connector (when available)
/// 2. Largest mode listed under sysfs `modes` (native / max density heuristic)
/// 3. Preferred (first) mode as last resort
fn ppi_from_drm_edid(prefer_detailed: bool, prefer_connector: Option<&str>) -> Option<f32> {
    let drm = std::path::Path::new("/sys/class/drm");
    let entries = std::fs::read_dir(drm).ok()?;

    let host = host_display();
    let host_mode = host.as_ref().map(|h| (h.name.as_str(), h.mode_w, h.mode_h));

    let mut candidates: Vec<(i32, f32)> = Vec::new();
    for ent in entries.flatten() {
        let path = ent.path();
        let name = path.file_name()?.to_string_lossy().into_owned();
        if !name.contains('-') {
            continue;
        }
        let conn = drm_connector_suffix(&name);
        let status = std::fs::read_to_string(path.join("status")).ok()?;
        if !status.trim().eq_ignore_ascii_case("connected") {
            continue;
        }
        let edid = std::fs::read(path.join("edid")).ok()?;
        let modes_txt = std::fs::read_to_string(path.join("modes")).ok()?;
        let phys = edid_physical_mm(&edid);
        let (rw, rh) = select_drm_mode(&modes_txt, conn, host_mode, phys)?;

        let ppi = if prefer_detailed {
            edid_ppi_detailed(&edid, rw, rh)
        } else {
            edid_ppi_cm(&edid, rw, rh)
        };
        if let Some(p) = ppi {
            let mut rank = 0i32;
            if prefer_connector
                .map(|p| conn.eq_ignore_ascii_case(p))
                .unwrap_or(false)
            {
                rank += 100;
            }
            if !conn.starts_with("eDP") {
                rank += 10;
            }
            candidates.push((rank, p));
        }
    }
    candidates.sort_by_key(|a| std::cmp::Reverse(a.0));
    candidates.into_iter().next().map(|(_, p)| p)
}

/// `card0-HDMI-A-1` → `HDMI-A-1`
fn drm_connector_suffix(sysfs_name: &str) -> &str {
    if let Some(rest) = sysfs_name.strip_prefix("card") {
        if let Some(idx) = rest.find('-') {
            return &rest[idx + 1..];
        }
    }
    sysfs_name
}

/// Pick pixel mode for PPI: host current → aspect-matched max → first line.
fn select_drm_mode(
    modes_txt: &str,
    conn: &str,
    host_mode: Option<(&str, u32, u32)>,
    phys_mm: Option<(f32, f32)>,
) -> Option<(u32, u32)> {
    if let Some((host_name, mw, mh)) = host_mode {
        if conn.eq_ignore_ascii_case(host_name) && mw >= 320 && mh >= 200 {
            return Some((mw, mh));
        }
    }
    let target_ar = phys_mm.and_then(|(pw, ph)| {
        if pw >= 50.0 && ph >= 50.0 {
            Some(pw / ph)
        } else {
            None
        }
    });
    // Score: lower aspect error is better; then larger area.
    let mut best: Option<(u32, u32, f32, u64)> = None; // w,h,ar_err,area
    let mut first: Option<(u32, u32)> = None;
    for line in modes_txt.lines() {
        if let Some((w, h)) = parse_mode_wh(line) {
            if h == 0 {
                continue;
            }
            if first.is_none() {
                first = Some((w, h));
            }
            let area = w as u64 * h as u64;
            let ar_err = match target_ar {
                Some(tar) => {
                    let ar = w as f32 / h as f32;
                    ((ar / tar) - 1.0).abs()
                }
                None => 0.0,
            };
            let better = match best {
                None => true,
                Some((_, _, e, a)) => {
                    if target_ar.is_some() {
                        ar_err + 1e-6 < e || ((ar_err - e).abs() < 1e-3 && area > a)
                    } else {
                        area > a
                    }
                }
            };
            if better {
                best = Some((w, h, ar_err, area));
            }
        }
    }
    best.map(|(w, h, _, _)| (w, h)).or(first)
}

/// Panel size in mm from EDID detailed descriptors or cm fields.
fn edid_physical_mm(edid: &[u8]) -> Option<(f32, f32)> {
    if edid.len() >= 126 {
        for base in [54, 72, 90, 108] {
            if base + 17 >= edid.len() {
                break;
            }
            if edid[base] == 0 && edid[base + 1] == 0 {
                continue;
            }
            let h_mm = edid[base + 12] as u16 | (((edid[base + 14] as u16) & 0xF0) << 4);
            let v_mm = edid[base + 13] as u16 | (((edid[base + 14] as u16) & 0x0F) << 8);
            if h_mm >= 50 && v_mm >= 50 {
                return Some((h_mm as f32, v_mm as f32));
            }
        }
    }
    if edid.len() >= 0x17 {
        let h_cm = edid[0x15] as f32;
        let v_cm = edid[0x16] as f32;
        if h_cm >= 5.0 && v_cm >= 5.0 {
            return Some((h_cm * 10.0, v_cm * 10.0));
        }
    }
    None
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
    ppi_from_physical_mode(h_cm * 10.0, v_cm * 10.0, mode_w, mode_h)
}

/// Detailed timing descriptors (18-byte blocks at 54,72,90,108) store size in mm.
///
/// Uses the **caller-supplied mode** (current / max) with physical mm from a DTD.
/// Panel size is fixed; mode pixels change with the active video mode.
fn edid_ppi_detailed(edid: &[u8], mode_w: u32, mode_h: u32) -> Option<f32> {
    if edid.len() < 126 {
        return None;
    }
    let mut any_mm: Option<(u16, u16)> = None;
    let mut matched_mm: Option<(u16, u16)> = None;
    for base in [54, 72, 90, 108] {
        if base + 17 >= edid.len() {
            break;
        }
        if edid[base] == 0 && edid[base + 1] == 0 {
            continue;
        }
        let h_act = edid[base + 2] as u32 | (((edid[base + 4] as u32) & 0xF0) << 4);
        let v_act = edid[base + 5] as u32 | (((edid[base + 7] as u32) & 0xF0) << 4);
        let h_mm = edid[base + 12] as u16 | (((edid[base + 14] as u16) & 0xF0) << 4);
        let v_mm = edid[base + 13] as u16 | (((edid[base + 14] as u16) & 0x0F) << 8);
        if h_mm < 50 || v_mm < 50 {
            continue;
        }
        if any_mm.is_none() {
            any_mm = Some((h_mm, v_mm));
        }
        if h_act == mode_w && v_act == mode_h {
            matched_mm = Some((h_mm, v_mm));
            break;
        }
    }
    let (h_mm, v_mm) = matched_mm.or(any_mm)?;
    ppi_from_physical_mode(h_mm as f32, v_mm as f32, mode_w, mode_h)
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

    // Encode base64 into reusable string (no per-frame heap String from encoder).
    sc.b64.clear();
    sc.b64.reserve(sc.rgba.len().div_ceil(3) * 4 + 8);
    b64_encode_into(&sc.rgba, &mut sc.b64);

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

/// Encode base64 into `out` without allocating a temporary String.
fn b64_encode_into(data: &[u8], out: &mut String) {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b64_hello() {
        let mut s = String::new();
        b64_encode_into(b"hello", &mut s);
        assert_eq!(s, "aGVsbG8=");
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

    #[test]
    fn device_cell_scale_recovers_four_inch_face() {
        // Ghostty buffer cell 19×42, compositor corr ~0.763 → device ~14.5×32.1
        // 4" @ 191.25 ppi = 765 device px → ~53×24 cells, on-glass ≈ 4".
        let ppi = 191.25_f32;
        let corr = 0.763_f32;
        let cw = 19.0 * corr;
        let ch = 42.0 * corr;
        let side = 4.0 * ppi;
        let (cols, rows) = cells_for_screen_square(200, 80, cw, ch, side);
        let og = (cols as f32 * cw).min(rows as f32 * ch) / ppi;
        assert!(
            (og - 4.0).abs() < 0.15,
            "on-glass {og:.3}\" want ~4\" cells {cols}×{rows} cell {cw:.1}×{ch:.1}"
        );
        // Without correction, face would be ~corr× smaller.
        let (cols_bad, rows_bad) = cells_for_screen_square(200, 80, 19.0, 42.0, side);
        let og_bad = (cols_bad as f32 * 19.0 * corr).min(rows_bad as f32 * 42.0 * corr) / ppi;
        assert!(
            og_bad < 3.3,
            "uncorrected on-glass should understate (got {og_bad:.2}\")"
        );
    }

    #[test]
    fn json_window_size_parses_niri_shape() {
        let j = r#"[{"pid": 3183128, "layout": {"window_size": [1528, 1017]}}]"#;
        let (w, h) = json_window_size_pair(j).expect("window_size");
        assert!((w - 1528.0).abs() < 0.1 && (h - 1017.0).abs() < 0.1);
        let pid = json_number_after(j, "\"pid\"").unwrap();
        assert_eq!(pid as u32, 3183128);
    }

    #[test]
    fn odyssey_current_mode_ppi_not_preferred() {
        // Samsung Odyssey G91SD: current 5120×1440, phys 1190×340 mm → ~108 PPI.
        // Preferred EDID mode 3840×1080 would wrongly yield ~81 PPI.
        let ppi_cur = ppi_from_physical_mode(1190.0, 340.0, 5120, 1440).expect("current");
        let ppi_pref = ppi_from_physical_mode(1190.0, 340.0, 3840, 1080).expect("preferred");
        assert!((ppi_cur - 108.4).abs() < 1.0, "current ppi={ppi_cur}");
        assert!((ppi_pref - 81.3).abs() < 1.0, "preferred ppi={ppi_pref}");
        let side = 4.0 * ppi_cur;
        assert!((side - 433.7).abs() < 4.0, "4\" side px={side}");
        assert!(ppi_cur > ppi_pref * 1.2);
    }

    #[test]
    fn select_drm_mode_prefers_host_then_aspect() {
        let modes = "3840x1080\n3840x2160\n5120x1440\n1920x1080\n";
        let phys = Some((1190.0_f32, 340.0_f32)); // Odyssey aspect ~3.5
        let (w, h) =
            select_drm_mode(modes, "HDMI-A-1", Some(("HDMI-A-1", 5120, 1440)), phys).unwrap();
        assert_eq!((w, h), (5120, 1440));
        // No host: prefer aspect match to panel (5120×1440), not max area (3840×2160).
        let (w2, h2) = select_drm_mode(modes, "HDMI-A-1", None, phys).unwrap();
        assert_eq!((w2, h2), (5120, 1440), "aspect-matched mode");
        let (w3, h3) =
            select_drm_mode(modes, "eDP-1", Some(("HDMI-A-1", 5120, 1440)), phys).unwrap();
        assert_eq!((w3, h3), (5120, 1440), "host name mismatch → aspect");
    }

    #[test]
    fn drm_connector_suffix_strips_card_prefix() {
        assert_eq!(drm_connector_suffix("card0-HDMI-A-1"), "HDMI-A-1");
        assert_eq!(drm_connector_suffix("card1-eDP-1"), "eDP-1");
    }

    #[test]
    fn niri_mode_index_parses() {
        let slice = r#"{
            "name":"HDMI-A-1",
            "physical_size":[1190,340],
            "current_mode":4,
            "modes":[
                {"width":3840,"height":1080,"refresh_rate":119959},
                {"width":3840,"height":2160,"refresh_rate":59940},
                {"width":3840,"height":2160,"refresh_rate":29970},
                {"width":5120,"height":1440,"refresh_rate":143987},
                {"width":5120,"height":1440,"refresh_rate":119985}
            ],
            "logical":{"scale":1.0}
        }"#;
        let (w, h) = niri_mode_at(slice, 4).expect("mode 4");
        assert_eq!((w, h), (5120, 1440));
        let (w0, h0) = niri_mode_at(slice, 0).unwrap();
        assert_eq!((w0, h0), (3840, 1080));
        let wrapped = format!("{{\"HDMI-A-1\":{slice}}}");
        let host = niri_parse_one_output(&wrapped, "HDMI-A-1").expect("parse");
        assert_eq!(host.mode_w, 5120);
        assert_eq!(host.mode_h, 1440);
        let ppi = host.ppi().unwrap();
        assert!((ppi - 108.4).abs() < 1.0, "ppi={ppi}");
    }
}
