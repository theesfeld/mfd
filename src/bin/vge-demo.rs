//! **Demo only** — loads pure-asm **libvge** and drives a terminal overlay.
//!
//! The engine is not Rust. Rust only:
//! 1. links `libvge` (see `build.rs` / `make`)
//! 2. holds a stroke list + terminal present glue
//! 3. calls `vge_*` through thin FFI wrappers
//!
//! ```text
//! make                          # build libvge.a (optional but preferred)
//! cargo run --release --bin vge-demo
//! VGE_WIDTH=2 cargo run --release --bin vge-demo
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
use vge::{
    engine_version, using_assembly, Surface, AMBER, CYAN, GREEN, GREEN_DIM, RED, WHITE,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> io::Result<()> {
    // Prove we are on the asm library, not a Rust reimplementation.
    let ver = engine_version();
    if !using_assembly() {
        eprintln!("error: demo requires pure-asm libvge (x86_64)");
        std::process::exit(1);
    }
    eprintln!("loaded libvge {ver} (assembly)");

    install_sigint();
    let hz = std::env::var("VGE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120u32);
    let width = std::env::var("VGE_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1i32)
        .max(1);

    let backend = detect_backend();
    let (tc, tr) = terminal_cells();
    let vp = Viewport {
        col: 0,
        row: 1,
        cols: tc.max(1),
        rows: tr.saturating_sub(2).max(1),
    };
    let (w, h) = surface_size_for_viewport(backend, vp);

    // Pixel buffer owned here; all drawing goes into libvge via Surface/DisplayList.
    let mut scanout = Surface::new(w, h);
    let mut list = DisplayList::with_capacity(512);
    let mut ostate = OverlayState::new();
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };

    enter_overlay()?;
    {
        let mut out = io::stdout().lock();
        write!(
            out,
            "\x1b[1;1H\x1b[2K\x1b[32mlibvge\x1b[0m {ver} · demo loads asm lib · {backend:?} · {w}x{h} · width={width} · q quit"
        )?;
        out.flush()?;
    }

    let t0 = Instant::now();
    let mut last_status = Instant::now();
    let mut n = 0u32;
    let mut beam_sum = Duration::ZERO;
    let mut present_sum = Duration::ZERO;

    while RUNNING.load(Ordering::Relaxed) {
        if poll_quit()? {
            break;
        }
        let t = t0.elapsed().as_secs_f32();

        // Build display list in Rust (demo logic only).
        list.clear();
        list.set_width(width);
        build_hud(&mut list, w as i32, h as i32, t);

        // libvge: transparent clear + stroke (all plot/line/circle = asm).
        let tb = Instant::now();
        list.refresh(&mut scanout);
        beam_sum += tb.elapsed();

        // Terminal glue (not the engine).
        let tp = Instant::now();
        present_at_state(&scanout, backend, vp, Some(&mut ostate))?;
        present_sum += tp.elapsed();

        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }

        n += 1;
        if last_status.elapsed() >= Duration::from_millis(250) {
            let d = (beam_sum / n.max(1)).as_micros();
            let p = (present_sum / n.max(1)).as_micros();
            let fps = pacer.as_ref().map(|x| x.fps).unwrap_or(0.0);
            let mut out = io::stdout().lock();
            write!(
                out,
                "\x1b[1;1H\x1b[2K\x1b[32mlibvge\x1b[0m {ver} · strokes={} · beam={d}µs present={p}µs fps={fps:.0} · q quit",
                list.len()
            )?;
            out.flush()?;
            n = 0;
            beam_sum = Duration::ZERO;
            present_sum = Duration::ZERO;
            last_status = Instant::now();
        }
    }

    leave_overlay()?;
    eprintln!("demo done · libvge {ver}");
    Ok(())
}

/// Demo scene only — emits stroke commands; libvge executes the beam.
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

    list.set_color(CYAN);
    let or = arm * 0.85;
    list.circle(
        cx + (or * (t * 2.15).cos()) as i32,
        cy + (or * (t * 2.15).sin()) as i32,
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
            list.line(cx + x0 as i32, cy + y0 as i32, cx + x1 as i32, cy + y1 as i32);
        }
    }
    list.set_color(AMBER);
    {
        let half = w as f32 * 0.2;
        let (x0, y0) = rot(-half, 0.0, rc, rs);
        let (x1, y1) = rot(half, 0.0, rc, rs);
        list.line(cx + x0 as i32, cy + y0 as i32, cx + x1 as i32, cy + y1 as i32);
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
    list.set_color(RED);
    let mut pts = [(0i32, 0i32); 5];
    for (i, &(px, py)) in [(-sq, -sq), (sq, -sq), (sq, sq), (-sq, sq), (-sq, -sq)]
        .iter()
        .enumerate()
    {
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
    unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 0 {
            return Ok(false);
        }
        let mut fds = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        if libc::poll(&mut fds as *mut _, 1, 0) > 0 && (fds.revents & libc::POLLIN) != 0 {
            let mut buf = [0u8; 16];
            let r = libc::read(
                libc::STDIN_FILENO,
                buf.as_mut_ptr() as *mut _,
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
