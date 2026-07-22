//! Live vector demo — **smooth** locked display rate + wall-clock motion.
//!
//! High uncapped FPS floods the terminal and looks choppy. Default is a
//! phase-locked 120 Hz present with animation time from the wall clock.
//!
//! ```text
//! cargo run --release --bin vge-demo                 # smooth 120 Hz overlay
//! VGE_HZ=60 cargo run --release --bin vge-demo       # 60 Hz
//! VGE_HZ=0 cargo run --release --bin vge-demo        # uncapped (often choppier)
//! cargo run --release --bin vge-demo -- --fb
//! VGE_EFFECTS=glow,radar cargo run --release --bin vge-demo
//! ```
//!
//! Quit: `q` / Esc / Ctrl+C.

use std::f32::consts::PI;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vge::frame::FramePacer;
use vge::term::{
    detect_backend, enter_fullscreen, enter_overlay, leave_fullscreen, leave_overlay, present_at,
    surface_size_for_viewport, terminal_cells, TermBackend, Viewport,
};
use vge::{Surface, Xform, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, RED, WHITE};

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

    // Default 120 Hz locked. Uncapped (0) often looks worse in terminals.
    let hz = std::env::var("VGE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120u32);
    let effects = std::env::var("VGE_EFFECTS").unwrap_or_default();
    // Slight phosphor hides micro-jitter on vector strokes (classic CRT).
    // Default ON for terminal modes; set VGE_PHOSPHOR=0 to disable.
    let phosphor = match std::env::var("VGE_PHOSPHOR").as_deref() {
        Ok("0") | Ok("off") | Ok("false") => false,
        Ok(_) => true,
        Err(_) => {
            args.iter().any(|a| a == "--phosphor")
                || matches!(mode, Mode::Overlay | Mode::Fullscreen)
        }
    };
    let decay = std::env::var("VGE_DECAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200u32);

    match mode {
        Mode::Fb => run_fb(hz, phosphor, decay, &effects),
        Mode::Overlay => run_overlay(hz, phosphor, decay, &effects),
        Mode::Fullscreen => run_full(hz, phosphor, decay, &effects),
    }
}

#[cfg(target_os = "linux")]
fn run_fb(hz: u32, phosphor: bool, decay: u32, effects: &str) -> io::Result<()> {
    use vge::fb::Framebuffer;
    let mut fb = Framebuffer::open_default()
        .map_err(|e| io::Error::new(e.kind(), format!("FB open failed: {e}")))?;
    let mut back = Surface::new(fb.width(), fb.height());
    eprintln!(
        "VGE FB · {}x{} · lock={} Hz · asm={} · phosphor={}",
        fb.width(),
        fb.height(),
        if hz == 0 {
            "off".into()
        } else {
            hz.to_string()
        },
        vge::using_assembly(),
        phosphor
    );
    loop_draw(
        &mut back,
        hz,
        phosphor,
        decay,
        effects,
        |s| {
            fb.present_from(s);
            Ok(())
        },
        None,
    )
}

#[cfg(not(target_os = "linux"))]
fn run_fb(_: u32, _: bool, _: u32, _: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "FB is Linux-only",
    ))
}

fn run_overlay(hz: u32, phosphor: bool, decay: u32, effects: &str) -> io::Result<()> {
    let backend = detect_backend();
    let vp = Viewport::centered_frac(0.70, 0.68);
    let (w, h) = surface_size_for_viewport(backend, vp);
    let mut back = Surface::new(w, h);

    enter_overlay()?;
    paint_chrome(vp, backend, w, h, hz)?;

    eprintln!(
        "VGE overlay · {backend:?} · {w}x{h} · viewport {}x{} · lock={} Hz · asm={}",
        vp.cols,
        vp.rows,
        if hz == 0 {
            "off".into()
        } else {
            hz.to_string()
        },
        vge::using_assembly()
    );

    let result = loop_draw(
        &mut back,
        hz,
        phosphor,
        decay,
        effects,
        |s| present_at(s, backend, vp),
        Some(vp),
    );
    leave_overlay()?;
    result
}

fn run_full(hz: u32, phosphor: bool, decay: u32, effects: &str) -> io::Result<()> {
    let backend = detect_backend();
    let vp = Viewport::full_terminal();
    let (w, h) = surface_size_for_viewport(backend, vp);
    let mut back = Surface::new(w, h);
    enter_fullscreen()?;
    let result = loop_draw(
        &mut back,
        hz,
        phosphor,
        decay,
        effects,
        |s| present_at(s, backend, vp),
        None,
    );
    leave_fullscreen()?;
    result
}

fn paint_chrome(vp: Viewport, backend: TermBackend, w: u32, h: u32, hz: u32) -> io::Result<()> {
    let (_tc, tr) = terminal_cells();
    let mut out = io::stdout().lock();
    write!(out, "\x1b[H\x1b[2J")?;
    write!(
        out,
        "\x1b[1;1H\x1b[32m vge \x1b[0m· {backend:?} · {w}x{h} px · cells {}×{} · lock {} Hz · q quit",
        vp.cols,
        vp.rows,
        if hz == 0 { "off".to_string() } else { hz.to_string() }
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

fn loop_draw(
    back: &mut Surface,
    hz: u32,
    phosphor: bool,
    decay: u32,
    effects: &str,
    mut present_fn: impl FnMut(&Surface) -> io::Result<()>,
    status_vp: Option<Viewport>,
) -> io::Result<()> {
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };
    // Wall-clock animation origin (independent of frame index).
    let t0 = Instant::now();
    let mut frame = 0u64;
    let mut last_status = Instant::now();
    let mut sum_draw = Duration::ZERO;
    let mut sum_present = Duration::ZERO;
    let mut n_acc = 0u32;

    while RUNNING.load(Ordering::Relaxed) {
        if poll_quit()? {
            break;
        }

        // Continuous time — smooth rotation even if a frame is dropped.
        let t = t0.elapsed().as_secs_f32();

        let td0 = Instant::now();
        if phosphor {
            back.decay(decay);
        } else {
            back.clear(BLACK);
        }
        draw_scene(back, t);
        apply_effects(back, effects, t);
        let draw_d = td0.elapsed();

        let tp0 = Instant::now();
        present_fn(back)?;
        let present_d = tp0.elapsed();

        // Phase-lock AFTER present so the glass updates on an even grid.
        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }

        sum_draw += draw_d;
        sum_present += present_d;
        n_acc += 1;
        frame += 1;

        if last_status.elapsed() >= Duration::from_millis(200) {
            let n = n_acc.max(1);
            let d_us = (sum_draw / n).as_micros();
            let p_us = (sum_present / n).as_micros();
            let (fps, mean_us, max_us) = if let Some(p) = pacer.as_ref() {
                (p.fps, p.mean_us, p.max_us)
            } else {
                let secs = last_status.elapsed().as_secs_f32().max(0.001);
                (
                    n_acc as f32 / secs,
                    (sum_draw + sum_present).as_micros() as u32 / n,
                    0,
                )
            };

            if let Some(vp) = status_vp {
                let (_tc, tr) = terminal_cells();
                let row = (vp.row + vp.rows + 1).min(tr);
                let mut out = io::stdout().lock();
                write!(
                    out,
                    "\x1b[{row};1H\x1b[K\x1b[32mdraw={d_us}µs  present={p_us}µs  fps={fps:.0}  frame_us mean={mean_us} max={max_us}  (smooth=even max, not max fps)\x1b[0m"
                )?;
                out.flush()?;
            } else if frame % 30 == 0 {
                eprint!(
                    "\r  draw={d_us}µs present={p_us}µs fps={fps:.0} mean={mean_us} max={max_us}µs   "
                );
                let _ = io::stderr().flush();
            }
            sum_draw = Duration::ZERO;
            sum_present = Duration::ZERO;
            n_acc = 0;
            last_status = Instant::now();
        }
    }
    eprintln!();
    Ok(())
}

fn apply_effects(s: &mut Surface, spec: &str, t: f32) {
    if spec.is_empty() {
        return;
    }
    for part in spec.split(',') {
        match part.trim().to_ascii_lowercase().as_str() {
            "glow" => vge::effects::glow(s, 2, 36),
            "bloom" => vge::effects::bloom(s, 48, 1),
            "radar" => {
                let cx = s.width() as i32 / 2;
                let cy = s.height() as i32 / 2;
                vge::effects::radar_fade(s, cx, cy, t * 2.0, 0.85);
            }
            "scan" | "scanlines" => vge::effects::scanlines(s, 180),
            _ => {}
        }
    }
}

fn draw_scene(c: &mut Surface, t: f32) {
    let w = c.width() as i32;
    let h = c.height() as i32;
    let cx = w / 2;
    let cy = h / 2;

    let m = (w.min(h) / 40).max(6);
    let bracket = m * 2;
    let th = if w > 800 { 2 } else { 1 };
    c.line_thick(m, m, m + bracket, m, GREEN_DIM, th);
    c.line_thick(m, m, m, m + bracket, GREEN_DIM, th);
    c.line_thick(w - m, m, w - m - bracket, m, GREEN_DIM, th);
    c.line_thick(w - m, m, w - m, m + bracket, GREEN_DIM, th);
    c.line_thick(m, h - m, m + bracket, h - m, GREEN_DIM, th);
    c.line_thick(m, h - m, m, h - m - bracket, GREEN_DIM, th);
    c.line_thick(w - m, h - m, w - m - bracket, h - m, GREEN_DIM, th);
    c.line_thick(w - m, h - m, w - m, h - m - bracket, GREEN_DIM, th);

    let arm = (w.min(h) as f32) * 0.28;
    // Continuous angular rates (rad/s) — wall-clock `t` makes this butter-smooth.
    let rot = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate(t * 1.35)
        .translate(-(cx as f32), -(cy as f32));
    for i in 0..6 {
        let a = i as f32 * PI / 3.0;
        c.line_xf(
            &rot,
            cx as f32,
            cy as f32,
            cx as f32 + arm * a.cos(),
            cy as f32 + arm * a.sin(),
            GREEN,
        );
    }

    let orbit_r = arm * 0.85;
    c.circle(
        (cx as f32 + orbit_r * (t * 2.15).cos()) as i32,
        (cy as f32 + orbit_r * (t * 2.15).sin()) as i32,
        (arm * 0.12).max(3.0) as i32,
        CYAN,
    );

    let roll = (t * 0.55).sin() * 14.0;
    let ladder = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate_deg(roll)
        .translate(-(cx as f32), -(cy as f32));
    for step in -3..=3 {
        if step == 0 {
            continue;
        }
        let y = cy as f32 + step as f32 * (h as f32 * 0.06);
        let half = w as f32 * 0.12;
        let gap = w as f32 * 0.04;
        let col = if step > 0 { GREEN } else { GREEN_DIM };
        c.line_xf(&ladder, cx as f32 - half, y, cx as f32 - gap, y, col);
        c.line_xf(&ladder, cx as f32 + gap, y, cx as f32 + half, y, col);
    }
    c.line_xf(
        &ladder,
        cx as f32 - w as f32 * 0.2,
        cy as f32,
        cx as f32 + w as f32 * 0.2,
        cy as f32,
        AMBER,
    );

    let g = (w.min(h) / 25).max(4);
    c.line(cx - g * 2, cy, cx - g / 2, cy, GREEN);
    c.line(cx + g / 2, cy, cx + g * 2, cy, GREEN);
    c.line(cx, cy - g, cx, cy - g / 3, GREEN);

    let rcx = w - w / 5;
    let rcy = h - h / 5;
    let rr = (w.min(h) / 8).max(10);
    for ring in 1..=3 {
        c.circle(rcx, rcy, rr * ring / 3, GREEN_DIM);
    }
    let sweep = t * 2.7;
    c.line_thick(
        rcx,
        rcy,
        rcx + (rr as f32 * sweep.cos()) as i32,
        rcy + (rr as f32 * sweep.sin()) as i32,
        GREEN,
        th,
    );

    let sq = (w.min(h) as f32) * 0.08;
    let bx = w as f32 * 0.18;
    let by = h as f32 * 0.22;
    let box_xf = Xform::identity()
        .translate(bx, by)
        .rotate(t * -2.0)
        .translate(-bx, -by);
    let corners = [
        (bx - sq, by - sq),
        (bx + sq, by - sq),
        (bx + sq, by + sq),
        (bx - sq, by + sq),
        (bx - sq, by - sq),
    ];
    for win in corners.windows(2) {
        c.line_xf(&box_xf, win[0].0, win[0].1, win[1].0, win[1].1, RED);
    }
    c.rect_fill(4, 4, 40, 6, WHITE);
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
