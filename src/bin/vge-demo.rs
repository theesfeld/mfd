//! **Demo only** — loads pure-asm **libvge** and draws instrument vectors.
//!
//! This is not a framebuffer game loop. Each frame is:
//!   clear → call several draw functions → present
//!
//! First real use: **needle gauges** and **tape gauges**. The demo shows
//! multiple draw types at once (line_aa, line_thick, circle, rect_fill,
//! polyline, line_xf + rotate).
//!
//! ```text
//! make
//! cargo run --release --bin vge-demo
//! VGE_TTL=10 cargo run --release --bin vge-demo   # optional needle trail
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
    engine_version, using_assembly, Surface, Xform, AMBER, CYAN, GREEN, GREEN_DIM, RED, WHITE,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> io::Result<()> {
    let ver = engine_version();
    if !using_assembly() {
        eprintln!("error: demo requires pure-asm libvge (x86_64)");
        std::process::exit(1);
    }
    eprintln!("loaded libvge {ver} (assembly) · gauge draw demo");

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
    // Optional trail on the moving needle tip only (library lifespan).
    let needle_ttl = std::env::var("VGE_TTL")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n: &u32| n > 0);

    let backend = detect_backend();
    let (tc, tr) = terminal_cells();
    let vp = Viewport {
        col: 0,
        row: 1,
        cols: tc.max(1),
        rows: tr.saturating_sub(2).max(1),
    };
    let (w, h) = surface_size_for_viewport(backend, vp);

    let mut scanout = Surface::new(w, h);
    let mut trail = DisplayList::with_capacity(128);
    let mut ostate = OverlayState::new();
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };

    let mode = match needle_ttl {
        Some(n) => format!("ttl={n}"),
        None => "draw".to_string(),
    };

    enter_overlay()?;
    {
        let mut out = io::stdout().lock();
        write!(
            out,
            "\x1b[1;1H\x1b[2K\x1b[32mlibvge\x1b[0m {ver} · gauges · {mode} · {backend:?} · {w}x{h} · q quit"
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

        // Instrument values (smooth wall-clock motion).
        let rpm = 0.5 + 0.45 * (t * 0.7).sin();
        let oil = 0.35 + 0.25 * (t * 0.4).cos();
        let speed = 0.5 + 0.4 * (t * 0.55).sin();
        let fuel = 0.7 + 0.2 * (t * 0.25).cos();
        let spin = t * 1.1;

        let tb = Instant::now();
        // Clear once. Then issue independent draws — not a scene graph.
        scanout.clear_transparent();

        let margin = (w.min(h) as f32 * 0.06).max(12.0) as i32;
        let col_w = (w as i32 - margin * 3) / 2;
        let row_h = (h as i32 - margin * 3) / 2;

        // Top-left: RPM needle (circle + ticks + thick needle + AA tip)
        draw_needle_gauge(
            &mut scanout,
            margin + col_w / 2,
            margin + row_h / 2,
            (col_w.min(row_h) / 2 - 4).max(20),
            rpm,
            GREEN,
            "RPM",
            width,
        );

        // Top-right: oil needle (second gauge; cyan)
        draw_needle_gauge(
            &mut scanout,
            margin * 2 + col_w + col_w / 2,
            margin + row_h / 2,
            (col_w.min(row_h) / 2 - 4).max(20),
            oil,
            CYAN,
            "OIL",
            width,
        );

        // Bottom-left: vertical tape (speed)
        draw_tape_gauge(
            &mut scanout,
            margin,
            margin * 2 + row_h,
            col_w,
            row_h,
            speed,
            true,
            AMBER,
            "SPD",
            width,
        );

        // Bottom-right: horizontal tape (fuel)
        draw_tape_gauge(
            &mut scanout,
            margin * 2 + col_w,
            margin * 2 + row_h,
            col_w,
            row_h,
            fuel,
            false,
            GREEN,
            "FUEL",
            width,
        );

        // Fun: small rotated diamond via vge_line_xf + vge_xform_rotate
        draw_spin_mark(
            &mut scanout,
            w as i32 / 2,
            h as i32 / 2,
            (w.min(h) as f32 * 0.04).max(8.0),
            spin,
            WHITE,
            width,
        );

        // Optional: needle tip trails (library lifespan on a short list only)
        if let Some(ttl) = needle_ttl {
            trail.tick();
            trail.set_lifespan(ttl);
            trail.set_width(width.max(1));
            let r = (col_w.min(row_h) / 2 - 4).max(20) as f32;
            let ang = needle_angle(rpm);
            let cx = (margin + col_w / 2) as f32;
            let cy = (margin + row_h / 2) as f32;
            let tip_x = cx + (r * 0.88) * ang.cos();
            let tip_y = cy + (r * 0.88) * ang.sin();
            trail.set_color(RED);
            trail.line(
                tip_x as i32 - 2,
                tip_y as i32,
                tip_x as i32 + 2,
                tip_y as i32,
            );
            trail.stroke_life(&mut scanout, true);
        }

        beam_sum += tb.elapsed();

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
                "\x1b[1;1H\x1b[2K\x1b[32mlibvge\x1b[0m {ver} · gauges · {mode} · beam={d}µs present={p}µs fps={fps:.0} · q quit"
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

/// Map 0..1 value to needle angle (left-stop through top to right-stop).
/// About −210° … +30° in screen coords (y down).
fn needle_angle(value01: f32) -> f32 {
    let v = value01.clamp(0.0, 1.0);
    let start = -PI * 0.75; // −135° from +x ≈ upper-left sweep start
    let sweep = PI * 1.5; // 270° arc
    start + v * sweep
}

/// Needle gauge: `circle` bezel, `line`/`line_aa` ticks, `line_thick` needle,
/// hub `circle`. Value in 0..1.
#[allow(clippy::too_many_arguments)]
fn draw_needle_gauge(
    s: &mut Surface,
    cx: i32,
    cy: i32,
    r: i32,
    value01: f32,
    color: u32,
    _label: &str,
    stroke_w: i32,
) {
    let r = r.max(12);
    // Bezel — circle outline (libvge vge_circle)
    s.circle(cx, cy, r, GREEN_DIM);
    if r > 16 {
        s.circle(cx, cy, r - 3, GREEN_DIM);
    }

    // Major ticks — AA hairlines
    let n_ticks = 9;
    for i in 0..n_ticks {
        let v = i as f32 / (n_ticks - 1) as f32;
        let a = needle_angle(v);
        let (cos, sin) = (a.cos(), a.sin());
        let outer = r as f32;
        let inner = r as f32 * if i % 2 == 0 { 0.78 } else { 0.86 };
        s.line_aa(
            cx + (outer * cos) as i32,
            cy + (outer * sin) as i32,
            cx + (inner * cos) as i32,
            cy + (inner * sin) as i32,
            GREEN_DIM,
        );
    }

    // Needle body — thick stroke (vge_line_thick)
    let a = needle_angle(value01);
    let (cos, sin) = (a.cos(), a.sin());
    let tip = r as f32 * 0.88;
    let tail = r as f32 * 0.15;
    let x0 = cx + (-tail * cos) as i32;
    let y0 = cy + (-tail * sin) as i32;
    let x1 = cx + (tip * cos) as i32;
    let y1 = cy + (tip * sin) as i32;
    let tw = stroke_w.clamp(2, 5);
    s.line_thick(x0, y0, x1, y1, color, tw);
    // Crisp tip hairline over the thick body
    s.line_aa(x0, y0, x1, y1, color);

    // Hub
    s.circle(cx, cy, (r / 12).max(2), color);
    s.circle(cx, cy, (r / 20).max(1), WHITE);
}

/// Tape gauge: frame with `line`/`polyline`, optional thin `rect_fill` band,
/// moving index mark. `vertical == true` → value rises upward.
#[allow(clippy::too_many_arguments)]
fn draw_tape_gauge(
    s: &mut Surface,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    value01: f32,
    vertical: bool,
    color: u32,
    _label: &str,
    stroke_w: i32,
) {
    let v = value01.clamp(0.0, 1.0);
    let x1 = x + w;
    let y1 = y + h;

    // Frame — polyline closed box (vge_polyline)
    s.polyline(&[(x, y), (x1, y), (x1, y1), (x, y1), (x, y)], GREEN_DIM);

    // Inner rail ticks
    if vertical {
        let mid_x = x + w / 2;
        let n = 11;
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let yy = y1 - ((h as f32) * t) as i32;
            let half = if i % 5 == 0 { w / 5 } else { w / 10 };
            s.line_aa(mid_x - half, yy, mid_x + half, yy, GREEN_DIM);
        }
        // Filled value band (vge_rect_fill) — dim strip under the index
        let fill_h = ((h as f32) * v) as i32;
        if fill_h > 1 {
            let band = (w / 8).max(2);
            s.rect_fill(mid_x - band, y1 - fill_h, mid_x + band, y1 - 1, GREEN_DIM);
        }
        // Index chevron — polyline
        let iy = y1 - ((h as f32) * v) as i32;
        let arm = (w / 4).max(6);
        s.polyline(
            &[
                (mid_x - arm, iy),
                (mid_x, iy),
                (mid_x - arm / 2, iy - arm / 3),
                (mid_x - arm / 2, iy + arm / 3),
                (mid_x, iy),
            ],
            color,
        );
        // Index bar — thick
        s.line_thick(mid_x - arm, iy, mid_x + arm, iy, color, stroke_w.max(2));
    } else {
        let mid_y = y + h / 2;
        let n = 11;
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let xx = x + ((w as f32) * t) as i32;
            let half = if i % 5 == 0 { h / 5 } else { h / 10 };
            s.line_aa(xx, mid_y - half, xx, mid_y + half, GREEN_DIM);
        }
        let fill_w = ((w as f32) * v) as i32;
        if fill_w > 1 {
            let band = (h / 8).max(2);
            s.rect_fill(x + 1, mid_y - band, x + fill_w, mid_y + band, GREEN_DIM);
        }
        let ix = x + ((w as f32) * v) as i32;
        let arm = (h / 4).max(6);
        s.polyline(
            &[
                (ix, mid_y - arm),
                (ix, mid_y),
                (ix - arm / 3, mid_y - arm / 2),
                (ix + arm / 3, mid_y - arm / 2),
                (ix, mid_y),
            ],
            color,
        );
        s.line_thick(ix, mid_y - arm, ix, mid_y + arm, color, stroke_w.max(2));
    }
}

/// Rotating diamond using **vge_xform_rotate** + **vge_line_xf**.
fn draw_spin_mark(
    s: &mut Surface,
    cx: i32,
    cy: i32,
    size: f32,
    angle: f32,
    color: u32,
    stroke_w: i32,
) {
    // Translate to center, then rotate (asm xform path).
    let m = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate(angle);

    let a = size;
    // Four edges of a diamond in local space → line_xf
    let edges = [
        (-a, 0.0, 0.0, -a),
        (0.0, -a, a, 0.0),
        (a, 0.0, 0.0, a),
        (0.0, a, -a, 0.0),
    ];
    for (x0, y0, x1, y1) in edges {
        s.line_xf(&m, x0, y0, x1, y1, color);
    }
    // Cross-hair (Bresenham fast + thick accent) so several draw kinds show up.
    if stroke_w > 1 {
        s.line_thick(cx - 2, cy, cx + 2, cy, color, stroke_w);
    } else {
        s.line_fast(cx - 3, cy, cx + 3, cy, color);
        s.line_fast(cx, cy - 3, cx, cy + 3, color);
    }
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
            let r = libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, buf.len());
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
