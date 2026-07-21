//! Live vector demo — terminal windows **and** Linux TTY frame buffer.
//!
//! ```text
//! # Any terminal emulator (Ghostty, Kitty, xterm, …) — default:
//! cargo run --release --bin vge-demo
//!
//! # Force half-block / ascii / kitty present:
//! VGE_TERM=half cargo run --release --bin vge-demo
//! VGE_TERM=ascii cargo run --release --bin vge-demo
//!
//! # Direct video RAM (Linux VT / TTY glass):
//! cargo run --release --bin vge-demo -- --fb
//! # Prefer: Ctrl+Alt+F3 → login → vge-demo --fb
//! ```
//!
//! Quit: `q` / Esc / Ctrl+C.

use std::f32::consts::PI;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use vge::term::{
    detect_backend, enter_fullscreen, leave_fullscreen, present, suggested_surface_size,
    TermBackend,
};
use vge::{Color, Surface, Xform, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, RED, WHITE};

static RUNNING: AtomicBool = AtomicBool::new(true);

/// Draw target without `dyn` (avoids fat-pointer footguns with the asm FFI).
trait Canvas {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn clear(&mut self, c: Color);
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color);
    fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color, t: i32);
    fn circle(&mut self, cx: i32, cy: i32, r: i32, c: Color);
    fn rect_fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color);
    fn line_xf(&mut self, m: &Xform, x0: f32, y0: f32, x1: f32, y1: f32, c: Color);
}

impl Canvas for Surface {
    fn width(&self) -> u32 {
        Surface::width(self)
    }
    fn height(&self) -> u32 {
        Surface::height(self)
    }
    fn clear(&mut self, c: Color) {
        Surface::clear(self, c)
    }
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
        Surface::line(self, x0, y0, x1, y1, c)
    }
    fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color, t: i32) {
        Surface::line_thick(self, x0, y0, x1, y1, c, t)
    }
    fn circle(&mut self, cx: i32, cy: i32, r: i32, c: Color) {
        Surface::circle(self, cx, cy, r, c)
    }
    fn rect_fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
        Surface::rect_fill(self, x0, y0, x1, y1, c)
    }
    fn line_xf(&mut self, m: &Xform, x0: f32, y0: f32, x1: f32, y1: f32, c: Color) {
        Surface::line_xf(self, m, x0, y0, x1, y1, c)
    }
}

#[cfg(target_os = "linux")]
impl Canvas for vge::fb::Framebuffer {
    fn width(&self) -> u32 {
        vge::fb::Framebuffer::width(self)
    }
    fn height(&self) -> u32 {
        vge::fb::Framebuffer::height(self)
    }
    fn clear(&mut self, c: Color) {
        vge::fb::Framebuffer::clear(self, c)
    }
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
        vge::fb::Framebuffer::line(self, x0, y0, x1, y1, c)
    }
    fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color, t: i32) {
        vge::fb::Framebuffer::line_thick(self, x0, y0, x1, y1, c, t)
    }
    fn circle(&mut self, cx: i32, cy: i32, r: i32, c: Color) {
        vge::fb::Framebuffer::circle(self, cx, cy, r, c)
    }
    fn rect_fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
        vge::fb::Framebuffer::rect_fill(self, x0, y0, x1, y1, c)
    }
    fn line_xf(&mut self, m: &Xform, x0: f32, y0: f32, x1: f32, y1: f32, c: Color) {
        vge::fb::Framebuffer::line_xf(self, m, x0, y0, x1, y1, c)
    }
}

#[derive(Clone, Copy, Debug)]
enum PresentMode {
    Framebuffer,
    Terminal(TermBackend),
}

fn main() -> io::Result<()> {
    install_sigint();
    match select_mode() {
        PresentMode::Framebuffer => run_framebuffer(),
        PresentMode::Terminal(backend) => run_terminal(backend),
    }
}

fn select_mode() -> PresentMode {
    let args: Vec<String> = std::env::args().collect();
    let flag_fb = args.iter().any(|a| a == "--fb" || a == "-f");
    let flag_term = args.iter().any(|a| a == "--term" || a == "-t");
    let env = std::env::var("VGE_PRESENT")
        .unwrap_or_default()
        .to_ascii_lowercase();

    // Explicit always wins.
    if flag_term {
        return PresentMode::Terminal(detect_backend());
    }
    if flag_fb || env == "fb" || env == "framebuffer" || env == "tty" {
        return PresentMode::Framebuffer;
    }
    // Auto FB only on a real Linux virtual console — never inside pts/GUI terminals.
    if is_linux_vt() && fb_available() {
        return PresentMode::Framebuffer;
    }
    // Ghostty / Kitty / xterm / every emulator → terminal present path.
    PresentMode::Terminal(detect_backend())
}

fn fb_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        vge::fb::Framebuffer::available()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn is_linux_vt() -> bool {
    let term = std::env::var("TERM").unwrap_or_default();
    if term == "linux" {
        return true;
    }
    #[cfg(unix)]
    {
        unsafe {
            let mut buf = [0i8; 64];
            if libc::ttyname_r(libc::STDIN_FILENO, buf.as_mut_ptr(), buf.len()) == 0 {
                let name = std::ffi::CStr::from_ptr(buf.as_ptr());
                if let Ok(s) = name.to_str() {
                    // /dev/tty1 … /dev/tty63 — not /dev/ttys000 or /dev/pts/N
                    if let Some(rest) = s.strip_prefix("/dev/tty") {
                        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(target_os = "linux")]
fn run_framebuffer() -> io::Result<()> {
    use vge::fb::Framebuffer;

    let mut fb = Framebuffer::open_default().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "open framebuffer failed: {e}\n\
                 Need RW on /dev/fb0 (or VGE_FB).\n\
                 For a clean TTY path: Ctrl+Alt+F3 → login → vge-demo --fb\n\
                 Inside Ghostty/Kitty/xterm use default (no --fb):\n\
                   cargo run --release --bin vge-demo"
            ),
        )
    })?;

    eprintln!(
        "VGE direct FB · {}x{} stride={} · asm={} · video RAM",
        fb.width(),
        fb.height(),
        fb.stride(),
        vge::using_assembly()
    );
    eprintln!("Assembly stores into mmap'd /dev/fb0. Quit: q / Ctrl+C");

    let start = Instant::now();
    let mut frame: u64 = 0;
    let mut last_fps = Instant::now();
    let mut fps_count = 0u32;
    let mut fps = 0u32;

    while RUNNING.load(Ordering::Relaxed) {
        if poll_quit()? {
            break;
        }
        let t = start.elapsed().as_secs_f32();
        // Monomorphized — no dyn. Pixels already on the glass.
        draw_scene(&mut fb, t, fps, 3);
        frame += 1;
        fps_count += 1;
        if last_fps.elapsed() >= Duration::from_secs(1) {
            fps = fps_count;
            fps_count = 0;
            last_fps = Instant::now();
        }
        thread::sleep(Duration::from_millis(16));
    }

    drop(fb);
    let _ = writeln!(
        io::stderr(),
        "vge-demo FB done · frames={frame} · asm={}",
        vge::using_assembly()
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn run_framebuffer() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "direct framebuffer present is Linux-only; use default terminal mode",
    ))
}

fn run_terminal(backend: TermBackend) -> io::Result<()> {
    let (w, h) = suggested_surface_size(backend);
    let mut surf = Surface::new(w, h);

    eprintln!(
        "VGE terminal · backend={backend:?} · surface={w}x{h} · asm={}",
        vge::using_assembly()
    );
    eprintln!("Quit: q / Esc / Ctrl+C");

    enter_fullscreen()?;
    let start = Instant::now();
    let mut frame: u64 = 0;
    let mut last_fps = Instant::now();
    let mut fps_count = 0u32;
    let mut fps = 0u32;

    let be = match backend {
        TermBackend::Kitty => 3,
        TermBackend::HalfBlock => 2,
        TermBackend::Ascii => 1,
    };

    let result = (|| -> io::Result<()> {
        while RUNNING.load(Ordering::Relaxed) {
            if poll_quit()? {
                break;
            }
            let t = start.elapsed().as_secs_f32();
            draw_scene(&mut surf, t, fps, be);
            present(&surf, backend)?;
            frame += 1;
            fps_count += 1;
            if last_fps.elapsed() >= Duration::from_secs(1) {
                fps = fps_count;
                fps_count = 0;
                last_fps = Instant::now();
            }
            thread::sleep(Duration::from_millis(16));
        }
        Ok(())
    })();

    leave_fullscreen()?;
    let _ = writeln!(
        io::stderr(),
        "vge-demo term done · backend={backend:?} · frames={frame}"
    );
    result
}

/// Generic monomorphized draw — `C: Canvas`, never `dyn Canvas`.
fn draw_scene<C: Canvas + ?Sized>(c: &mut C, t: f32, fps: u32, backend_code: i32) {
    let w = c.width() as i32;
    let h = c.height() as i32;
    let cx = w / 2;
    let cy = h / 2;
    c.clear(BLACK);

    let m = (w.min(h) / 40).max(8);
    let bracket = m * 2;
    let th = if w > 1000 { 3 } else { 2 };
    c.line_thick(m, m, m + bracket, m, GREEN_DIM, th);
    c.line_thick(m, m, m, m + bracket, GREEN_DIM, th);
    c.line_thick(w - m, m, w - m - bracket, m, GREEN_DIM, th);
    c.line_thick(w - m, m, w - m, m + bracket, GREEN_DIM, th);
    c.line_thick(m, h - m, m + bracket, h - m, GREEN_DIM, th);
    c.line_thick(m, h - m, m, h - m - bracket, GREEN_DIM, th);
    c.line_thick(w - m, h - m, w - m - bracket, h - m, GREEN_DIM, th);
    c.line_thick(w - m, h - m, w - m, h - m - bracket, GREEN_DIM, th);

    let arm = (w.min(h) as f32) * 0.28;
    let rot = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate(t * 1.2)
        .translate(-(cx as f32), -(cy as f32));
    for i in 0..6 {
        let a = i as f32 * PI / 3.0;
        let x1 = cx as f32 + arm * a.cos();
        let y1 = cy as f32 + arm * a.sin();
        c.line_xf(&rot, cx as f32, cy as f32, x1, y1, GREEN);
    }

    let orbit_r = arm * 0.85;
    let ox = cx as f32 + orbit_r * (t * 2.0).cos();
    let oy = cy as f32 + orbit_r * (t * 2.0).sin();
    c.circle(ox as i32, oy as i32, (arm * 0.12).max(4.0) as i32, CYAN);

    let roll = (t * 0.4).sin() * 12.0;
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

    let g = (w.min(h) / 25).max(6);
    c.line(cx - g * 2, cy, cx - g / 2, cy, GREEN);
    c.line(cx + g / 2, cy, cx + g * 2, cy, GREEN);
    c.line(cx, cy - g, cx, cy - g / 3, GREEN);

    let rcx = w - w / 5;
    let rcy = h - h / 5;
    let rr = (w.min(h) / 8).max(12);
    for ring in 1..=3 {
        c.circle(rcx, rcy, rr * ring / 3, GREEN_DIM);
    }
    let sweep = t * 2.5;
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
        .rotate(t * -1.8)
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

    let label_y = 4;
    c.rect_fill(4, label_y, 4 + (fps.min(120) as i32), label_y + 3, WHITE);
    c.rect_fill(4, label_y + 6, 4 + backend_code * 8, label_y + 9, CYAN);
}

fn poll_quit() -> io::Result<bool> {
    #[cfg(unix)]
    {
        unsafe {
            // Do not read a non-tty pipe (cargo/IDE) as if it were keys.
            if libc::isatty(libc::STDIN_FILENO) == 0 {
                return Ok(false);
            }
            let mut fds = libc::pollfd {
                fd: libc::STDIN_FILENO,
                events: libc::POLLIN,
                revents: 0,
            };
            let n = libc::poll(&mut fds as *mut libc::pollfd, 1, 0);
            if n > 0 && (fds.revents & libc::POLLIN) != 0 {
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
        // libc::sighandler_t is a pointer-sized integer on this target.
        #[allow(unknown_lints, function_casts_as_integer)]
        let handler = on_sigint as *const () as libc::sighandler_t;
        libc::signal(libc::SIGINT, handler);
    }
}
