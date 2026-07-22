//! Calligraphic demo — **display list** is the image; pixels are scanout.
//!
//! Model (1970s vector CRT / aircraft HUD stroke generator):
//! 1. Rebuild / update the stroke list from live state
//! 2. `list.refresh(surface, phosphor)` — decay + retrace beam
//! 3. Present the surface once (Kitty / half / FB)
//!
//! ```text
//! cargo run --release --bin vge-demo
//! VGE_HZ=60 cargo run --release --bin vge-demo
//! cargo run --release --bin vge-demo -- --fb
//! ```
//!
//! Quit: `q` / Esc / Ctrl+C.

use std::f32::consts::PI;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vge::frame::FramePacer;
use vge::stroke::DisplayList;
use vge::term::{
    detect_backend, enter_fullscreen, enter_overlay, leave_fullscreen, leave_overlay, present_at,
    surface_size_for_viewport, terminal_cells, TermBackend, Viewport,
};
use vge::{Surface, AMBER, CYAN, GREEN, GREEN_DIM, RED, WHITE};

static RUNNING: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy)]
enum Mode {
    Fb,
    Overlay,
    Fullscreen,
}

fn main() -> io::Result<()> {
    install_sigint();
    let args: Vec<String> = std::env::args().collect();
    let mode = if args.iter().any(|a| a == "--fb" || a == "-f") {
        Mode::Fb
    } else if args.iter().any(|a| a == "--full") {
        Mode::Fullscreen
    } else {
        Mode::Overlay
    };

    let hz = std::env::var("VGE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120u32);
    // Phosphor: default on (storage / refresh vector feel). 0 = hard clear each retrace.
    let decay = match std::env::var("VGE_PHOSPHOR").as_deref() {
        Ok("0") | Ok("off") => 0u32,
        Ok(s) if s.parse::<u32>().is_ok() => s.parse().unwrap_or(200),
        _ => 200u32,
    };

    match mode {
        Mode::Fb => run_fb(hz, decay),
        Mode::Overlay => run_overlay(hz, decay),
        Mode::Fullscreen => run_full(hz, decay),
    }
}

#[cfg(target_os = "linux")]
fn run_fb(hz: u32, decay: u32) -> io::Result<()> {
    use vge::fb::Framebuffer;
    let mut fb = Framebuffer::open_default()
        .map_err(|e| io::Error::new(e.kind(), format!("FB open failed: {e}")))?;
    let mut scanout = Surface::new(fb.width(), fb.height());
    let mut list = DisplayList::with_capacity(512);
    eprintln!(
        "VGE stroke · FB {}x{} · list refresh · lock={} Hz · asm={}",
        fb.width(),
        fb.height(),
        hz_label(hz),
        vge::using_assembly()
    );
    loop_stroke(
        &mut list,
        &mut scanout,
        hz,
        decay,
        |s| {
            fb.present_from(s);
            Ok(())
        },
        None,
    )
}

#[cfg(not(target_os = "linux"))]
fn run_fb(_: u32, _: u32) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "FB is Linux-only",
    ))
}

fn run_overlay(hz: u32, decay: u32) -> io::Result<()> {
    let backend = detect_backend();
    let vp = Viewport::centered_frac(0.70, 0.68);
    let (w, h) = surface_size_for_viewport(backend, vp);
    let mut scanout = Surface::new(w, h);
    let mut list = DisplayList::with_capacity(512);

    enter_overlay()?;
    paint_chrome(vp, backend, w, h, hz)?;
    eprintln!(
        "VGE stroke · overlay {backend:?} · {w}x{h} · cells {}×{} · lock={} Hz · asm={}",
        vp.cols,
        vp.rows,
        hz_label(hz),
        vge::using_assembly()
    );

    let result = loop_stroke(
        &mut list,
        &mut scanout,
        hz,
        decay,
        |s| present_at(s, backend, vp),
        Some(vp),
    );
    leave_overlay()?;
    result
}

fn run_full(hz: u32, decay: u32) -> io::Result<()> {
    let backend = detect_backend();
    let vp = Viewport::full_terminal();
    let (w, h) = surface_size_for_viewport(backend, vp);
    let mut scanout = Surface::new(w, h);
    let mut list = DisplayList::with_capacity(512);
    enter_fullscreen()?;
    let result = loop_stroke(
        &mut list,
        &mut scanout,
        hz,
        decay,
        |s| present_at(s, backend, vp),
        None,
    );
    leave_fullscreen()?;
    result
}

fn hz_label(hz: u32) -> String {
    if hz == 0 {
        "off".into()
    } else {
        hz.to_string()
    }
}

fn paint_chrome(vp: Viewport, backend: TermBackend, w: u32, h: u32, hz: u32) -> io::Result<()> {
    let (_tc, tr) = terminal_cells();
    let mut out = io::stdout().lock();
    write!(out, "\x1b[H\x1b[2J")?;
    write!(
        out,
        "\x1b[1;1H\x1b[32m vge stroke list \x1b[0m· {backend:?} · scanout {w}x{h} · cells {}×{} · refresh {} Hz · q quit",
        vp.cols,
        vp.rows,
        hz_label(hz)
    )?;
    let r0 = vp.row + 1;
    let c0 = vp.col + 1;
    if r0 > 1 {
        write!(
            out,
            "\x1b[{};{}H\x1b[90m┌{}┐\x1b[0m",
            r0.saturating_sub(1),
            c0,
            "─".repeat(vp.cols as usize)
        )?;
    }
    let bottom = r0 + vp.rows;
    if bottom < tr {
        write!(
            out,
            "\x1b[{};{}H\x1b[90m└{}┘\x1b[0m",
            bottom,
            c0,
            "─".repeat(vp.cols as usize)
        )?;
    }
    out.flush()
}

fn loop_stroke(
    list: &mut DisplayList,
    scanout: &mut Surface,
    hz: u32,
    decay: u32,
    mut present_fn: impl FnMut(&Surface) -> io::Result<()>,
    status_vp: Option<Viewport>,
) -> io::Result<()> {
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };
    let t0 = Instant::now();
    let mut frame = 0u64;
    let mut last_status = Instant::now();
    let mut sum_build = Duration::ZERO;
    let mut sum_beam = Duration::ZERO;
    let mut sum_present = Duration::ZERO;
    let mut n_acc = 0u32;

    while RUNNING.load(Ordering::Relaxed) {
        if poll_quit()? {
            break;
        }

        let t = t0.elapsed().as_secs_f32();
        let w = scanout.width() as i32;
        let h = scanout.height() as i32;

        // 1) Rebuild display list from live state (HUD computer).
        let tb = Instant::now();
        list.clear();
        build_hud(list, w, h, t);
        let build_d = tb.elapsed();

        // 2) Beam refresh: phosphor + stroke list into scanout buffer.
        let tr = Instant::now();
        list.refresh(scanout, decay);
        let beam_d = tr.elapsed();

        // 3) Host present (scanout only — not the source of truth).
        let tp = Instant::now();
        present_fn(scanout)?;
        let present_d = tp.elapsed();

        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }

        sum_build += build_d;
        sum_beam += beam_d;
        sum_present += present_d;
        n_acc += 1;
        frame += 1;

        if last_status.elapsed() >= Duration::from_millis(200) {
            let n = n_acc.max(1);
            let b_us = (sum_build / n).as_micros();
            let m_us = (sum_beam / n).as_micros();
            let p_us = (sum_present / n).as_micros();
            let (fps, max_us) = if let Some(p) = pacer.as_ref() {
                (p.fps, p.max_us)
            } else {
                let secs = last_status.elapsed().as_secs_f32().max(0.001);
                (n_acc as f32 / secs, 0)
            };
            if let Some(vp) = status_vp {
                let (_tc, tr) = terminal_cells();
                let row = (vp.row + vp.rows + 1).min(tr);
                let mut out = io::stdout().lock();
                write!(
                    out,
                    "\x1b[{row};1H\x1b[K\x1b[32mstrokes={}  build={b_us}µs  beam={m_us}µs  present={p_us}µs  fps={fps:.0}  max_frame={max_us}µs\x1b[0m",
                    list.len()
                )?;
                out.flush()?;
            }
            sum_build = Duration::ZERO;
            sum_beam = Duration::ZERO;
            sum_present = Duration::ZERO;
            n_acc = 0;
            last_status = Instant::now();
        }
        let _ = frame;
    }
    eprintln!();
    Ok(())
}

/// Compose the calligraphic picture (beam commands only).
fn build_hud(list: &mut DisplayList, w: i32, h: i32, t: f32) {
    let cx = w / 2;
    let cy = h / 2;
    let m = (w.min(h) / 40).max(6);
    let bracket = m * 2;
    let th = if w > 800 { 2 } else { 1 };

    list.set_color(GREEN_DIM);
    list.line_thick(m, m, m + bracket, m, th);
    list.line_thick(m, m, m, m + bracket, th);
    list.line_thick(w - m, m, w - m - bracket, m, th);
    list.line_thick(w - m, m, w - m, m + bracket, th);
    list.line_thick(m, h - m, m + bracket, h - m, th);
    list.line_thick(m, h - m, m, h - m - bracket, th);
    list.line_thick(w - m, h - m, w - m - bracket, h - m, th);
    list.line_thick(w - m, h - m, w - m, h - m - bracket, th);

    let arm = (w.min(h) as f32) * 0.28;
    let ang = t * 1.35;
    list.set_color(GREEN);
    for i in 0..6 {
        let a = ang + i as f32 * PI / 3.0;
        list.line(
            cx,
            cy,
            cx + (arm * a.cos()) as i32,
            cy + (arm * a.sin()) as i32,
        );
    }

    let orbit_r = arm * 0.85;
    let ox = cx + (orbit_r * (t * 2.15).cos()) as i32;
    let oy = cy + (orbit_r * (t * 2.15).sin()) as i32;
    list.set_color(CYAN);
    list.circle(ox, oy, (arm * 0.12).max(3.0) as i32);

    // Pitch ladder (rotated by synthetic roll).
    let roll = (t * 0.55).sin() * 14.0f32;
    let (rs, rc) = (roll.to_radians().sin(), roll.to_radians().cos());
    list.set_color(GREEN);
    for step in -3..=3 {
        if step == 0 {
            continue;
        }
        let y_off = step as f32 * (h as f32 * 0.06);
        let half = w as f32 * 0.12;
        let gap = w as f32 * 0.04;
        if step < 0 {
            list.set_color(GREEN_DIM);
        } else {
            list.set_color(GREEN);
        }
        for (xa, xb) in [(-half, -gap), (gap, half)] {
            let (x0, y0) = rot(xa, y_off, rc, rs);
            let (x1, y1) = rot(xb, y_off, rc, rs);
            list.line(
                cx + x0 as i32,
                cy + y0 as i32,
                cx + x1 as i32,
                cy + y1 as i32,
            );
        }
    }
    list.set_color(AMBER);
    {
        let half = w as f32 * 0.2;
        let (x0, y0) = rot(-half, 0.0, rc, rs);
        let (x1, y1) = rot(half, 0.0, rc, rs);
        list.line(
            cx + x0 as i32,
            cy + y0 as i32,
            cx + x1 as i32,
            cy + y1 as i32,
        );
    }

    list.set_color(GREEN);
    let g = (w.min(h) / 25).max(4);
    list.line(cx - g * 2, cy, cx - g / 2, cy);
    list.line(cx + g / 2, cy, cx + g * 2, cy);
    list.line(cx, cy - g, cx, cy - g / 3);

    let rcx = w - w / 5;
    let rcy = h - h / 5;
    let rr = (w.min(h) / 8).max(10);
    list.set_color(GREEN_DIM);
    for ring in 1..=3 {
        list.circle(rcx, rcy, rr * ring / 3);
    }
    list.set_color(GREEN);
    let sweep = t * 2.7;
    list.line_thick(
        rcx,
        rcy,
        rcx + (rr as f32 * sweep.cos()) as i32,
        rcy + (rr as f32 * sweep.sin()) as i32,
        th,
    );

    // Spinning square as polyline.
    let sq = (w.min(h) as f32) * 0.08;
    let bx = w as f32 * 0.18;
    let by = h as f32 * 0.22;
    let sa = t * -2.0;
    let (ss, sc) = (sa.sin(), sa.cos());
    let corners = [(-sq, -sq), (sq, -sq), (sq, sq), (-sq, sq), (-sq, -sq)];
    list.set_color(RED);
    let mut pts = [(0i32, 0i32); 5];
    for (i, (px, py)) in corners.iter().enumerate() {
        let x = bx + px * sc - py * ss;
        let y = by + px * ss + py * sc;
        pts[i] = (x as i32, y as i32);
    }
    list.polyline(&pts);

    list.set_color(WHITE);
    list.line_thick(4, 4, 40, 4, 2);
}

fn rot(x: f32, y: f32, c: f32, s: f32) -> (f32, f32) {
    (x * c - y * s, x * s + y * c)
}

fn poll_quit() -> io::Result<bool> {
    #[cfg(unix)]
    {
        unsafe {
            if libc::isatty(libc::STDIN_FILENO) == 0 {
                return Ok(false);
            }
            let mut fds = libc::pollfd {
                fd: libc::STDIN_FILENO,
                events: libc::POLLIN,
                revents: 0,
            };
            if libc::poll(&mut fds as *mut libc::pollfd, 1, 0) > 0
                && (fds.revents & libc::POLLIN) != 0
            {
                let mut buf = [0u8; 16];
                let r = libc::read(
                    libc::STDIN_FILENO,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                );
                if r > 0 {
                    for &b in &buf[..r as usize] {
                        if b == b'q' || b == b'Q' || b == 0x1b {
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}

fn install_sigint() {
    #[cfg(unix)]
    unsafe {
        extern "C" fn on_sigint(_: libc::c_int) {
            RUNNING.store(false, Ordering::Relaxed);
        }
        #[allow(unknown_lints, function_casts_as_integer)]
        let handler = on_sigint as *const () as libc::sighandler_t;
        libc::signal(libc::SIGINT, handler);
    }
}
