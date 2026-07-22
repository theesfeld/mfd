//! Calligraphic overlay demo — strokes on top of the terminal.
//!
//! No black panel. No phosphor trail. Crisp lines. Width is controllable.
//! Transparent scanout; present paints beam pixels only.
//!
//! ```text
//! cargo run --release --bin vge-demo
//! VGE_WIDTH=3 cargo run --release --bin vge-demo   # stroke width in px
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
    detect_backend, enter_overlay, leave_overlay, present_at_state, surface_size_for_viewport,
    terminal_cells, OverlayState, Viewport,
};
use vge::{Surface, AMBER, CYAN, GREEN, GREEN_DIM, RED, WHITE};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> io::Result<()> {
    install_sigint();
    let args: Vec<String> = std::env::args().collect();
    let fb = args.iter().any(|a| a == "--fb" || a == "-f");
    let hz = std::env::var("VGE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120u32);
    let width = std::env::var("VGE_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1i32)
        .max(1);

    if fb {
        run_fb(hz, width)
    } else {
        run_overlay(hz, width)
    }
}

#[cfg(target_os = "linux")]
fn run_fb(hz: u32, width: i32) -> io::Result<()> {
    use vge::fb::Framebuffer;
    let mut fb = Framebuffer::open_default()
        .map_err(|e| io::Error::new(e.kind(), format!("FB open failed: {e}")))?;
    // FB is a full glass — black clear is fine for dedicated mode.
    let mut scanout = Surface::new(fb.width(), fb.height());
    let mut list = DisplayList::with_capacity(512);
    eprintln!(
        "VGE stroke · FB {}x{} · width={width}px · {} Hz · asm={}",
        fb.width(),
        fb.height(),
        hz,
        vge::using_assembly()
    );
    loop_stroke(
        &mut list,
        &mut scanout,
        hz,
        width,
        true,
        |s, _| {
            // Opaque black base for dedicated FB, then strokes already in s.
            // For FB we clear black in refresh_fb path.
            fb.present_from(s);
            Ok(())
        },
        None,
        None,
    )
}

#[cfg(not(target_os = "linux"))]
fn run_fb(_: u32, _: i32) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "FB is Linux-only",
    ))
}

fn run_overlay(hz: u32, width: i32) -> io::Result<()> {
    let backend = detect_backend();
    // Full terminal area — strokes float over whatever is already on screen.
    let (tc, tr) = terminal_cells();
    let vp = Viewport {
        col: 0,
        row: 1, // leave one status line
        cols: tc.max(1),
        rows: tr.saturating_sub(2).max(1),
    };
    let (w, h) = surface_size_for_viewport(backend, vp);
    let mut scanout = Surface::new(w, h);
    let mut list = DisplayList::with_capacity(512);
    let mut ostate = OverlayState::new();

    enter_overlay()?;
    // Do NOT clear the screen. Terminal content stays. Status on line 1 only.
    {
        let mut out = io::stdout().lock();
        write!(
            out,
            "\x1b[1;1H\x1b[2K\x1b[32mvge\x1b[0m stroke overlay · {backend:?} · {w}x{h} · width={width}px · {} Hz · q quit",
            hz
        )?;
        out.flush()?;
    }

    eprintln!(
        "VGE stroke · transparent overlay · {backend:?} · {w}x{h} · width={width} · asm={}",
        vge::using_assembly()
    );

    let result = loop_stroke(
        &mut list,
        &mut scanout,
        hz,
        width,
        false,
        |s, st| present_at_state(s, backend, vp, st),
        Some(vp),
        Some(&mut ostate),
    );
    leave_overlay()?;
    result
}

#[allow(clippy::too_many_arguments)]
fn loop_stroke(
    list: &mut DisplayList,
    scanout: &mut Surface,
    hz: u32,
    width: i32,
    fb_black: bool,
    mut present_fn: impl FnMut(&Surface, Option<&mut OverlayState>) -> io::Result<()>,
    status_vp: Option<Viewport>,
    mut ostate: Option<&mut OverlayState>,
) -> io::Result<()> {
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };
    let t0 = Instant::now();
    let mut last_status = Instant::now();
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

        list.clear();
        list.set_width(width);
        build_hud(list, w, h, t);

        let tr = Instant::now();
        if fb_black {
            scanout.clear(vge::BLACK);
            list.stroke(scanout);
        } else {
            // Transparent clear + crisp strokes only.
            list.refresh(scanout);
        }
        let beam_d = tr.elapsed();

        let tp = Instant::now();
        present_fn(scanout, ostate.as_deref_mut())?;
        let present_d = tp.elapsed();

        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }

        sum_beam += beam_d;
        sum_present += present_d;
        n_acc += 1;

        if last_status.elapsed() >= Duration::from_millis(250) {
            let n = n_acc.max(1);
            let m_us = (sum_beam / n).as_micros();
            let p_us = (sum_present / n).as_micros();
            let fps = pacer.as_ref().map(|p| p.fps).unwrap_or(0.0);
            let max_us = pacer.as_ref().map(|p| p.max_us).unwrap_or(0);
            let mut out = io::stdout().lock();
            write!(
                out,
                "\x1b[1;1H\x1b[2K\x1b[32mvge\x1b[0m strokes={} width={width} beam={m_us}µs present={p_us}µs fps={fps:.0} max={max_us}µs · q quit",
                list.len()
            )?;
            out.flush()?;
            let _ = status_vp;
            sum_beam = Duration::ZERO;
            sum_present = Duration::ZERO;
            n_acc = 0;
            last_status = Instant::now();
        }
    }
    eprintln!();
    Ok(())
}

fn build_hud(list: &mut DisplayList, w: i32, h: i32, t: f32) {
    let cx = w / 2;
    let cy = h / 2;
    let m = (w.min(h) / 40).max(6);
    let bracket = m * 2;

    list.set_color(GREEN_DIM);
    list.line(m, m, m + bracket, m);
    list.line(m, m, m, m + bracket);
    list.line(w - m, m, w - m - bracket, m);
    list.line(w - m, m, w - m, m + bracket);
    list.line(m, h - m, m + bracket, h - m);
    list.line(m, h - m, m, h - m - bracket);
    list.line(w - m, h - m, w - m - bracket, h - m);
    list.line(w - m, h - m, w - m, h - m - bracket);

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
    list.set_color(CYAN);
    list.circle(
        cx + (orbit_r * (t * 2.15).cos()) as i32,
        cy + (orbit_r * (t * 2.15).sin()) as i32,
        (arm * 0.12).max(3.0) as i32,
    );

    let roll = (t * 0.55).sin() * 14.0f32;
    let (rs, rc) = (roll.to_radians().sin(), roll.to_radians().cos());
    for step in -3..=3 {
        if step == 0 {
            continue;
        }
        let y_off = step as f32 * (h as f32 * 0.06);
        let half = w as f32 * 0.12;
        let gap = w as f32 * 0.04;
        list.set_color(if step < 0 { GREEN_DIM } else { GREEN });
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
    list.line(
        rcx,
        rcy,
        rcx + (rr as f32 * sweep.cos()) as i32,
        rcy + (rr as f32 * sweep.sin()) as i32,
    );

    let sq = (w.min(h) as f32) * 0.08;
    let bx = w as f32 * 0.18;
    let by = h as f32 * 0.22;
    let sa = t * -2.0;
    let (ss, sc) = (sa.sin(), sa.cos());
    let corners = [(-sq, -sq), (sq, -sq), (sq, sq), (-sq, sq), (-sq, -sq)];
    list.set_color(RED);
    let mut pts = [(0i32, 0i32); 5];
    for (i, (px, py)) in corners.iter().enumerate() {
        pts[i] = (
            (bx + px * sc - py * ss) as i32,
            (by + px * ss + py * sc) as i32,
        );
    }
    list.polyline(&pts);

    list.set_color(WHITE);
    list.line(4, 4, 40, 4);
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
