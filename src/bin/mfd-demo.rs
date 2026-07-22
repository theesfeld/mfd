//! **MFD demo** — multi-page fighter + automotive cluster in the terminal.
//!
//! Keys: `1`–`8` jet pages · `a` automotive cluster · `q` quit.
//!
//! ```text
//! cargo run --release --bin mfd-demo
//! MFD_TERM=kitty cargo run --release --bin mfd-demo
//! ```

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use mfd::auto::{self, ObdSnapshot};
use mfd::frame::FramePacer;
use mfd::jet;
use mfd::page::Page;
use mfd::term::{
    detect_backend, enter_fullscreen, leave_fullscreen, present_at, surface_size_for_viewport,
    terminal_cells, Viewport,
};
use mfd::{engine_version, using_assembly, Surface};

static RUNNING: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Sms,
    Hsd,
    Tgp,
    Fcr,
    Eng,
    Fuel,
    Dte,
    Test,
    AutoCluster,
    AutoPower,
    AutoTemps,
    AutoObd,
}

fn main() -> io::Result<()> {
    let ver = engine_version();
    if !using_assembly() {
        eprintln!("error: mfd-demo requires pure-asm libmfd (x86_64)");
        std::process::exit(1);
    }
    eprintln!("loaded libmfd {ver} · MFD multi-page demo");

    install_sigint();
    let hz = std::env::var("MFD_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60u32);

    let backend = detect_backend();
    let (tc, tr) = terminal_cells();
    let vp = Viewport {
        col: 0,
        row: 0,
        cols: tc.max(1),
        rows: tr.max(1),
    };
    let (w, h) = surface_size_for_viewport(backend, vp);
    let mut panel = Surface::new(w, h);
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };

    let mut screen = Screen::Hsd;
    enter_fullscreen()?;
    let t0 = Instant::now();

    while RUNNING.load(Ordering::Relaxed) {
        if let Some(s) = poll_key()? {
            match s {
                b'q' | b'Q' | 0x1b => break,
                b'1' => screen = Screen::Sms,
                b'2' => screen = Screen::Hsd,
                b'3' => screen = Screen::Tgp,
                b'4' => screen = Screen::Fcr,
                b'5' => screen = Screen::Eng,
                b'6' => screen = Screen::Fuel,
                b'7' => screen = Screen::Dte,
                b'8' => screen = Screen::Test,
                b'a' | b'A' => screen = Screen::AutoCluster,
                b's' | b'S' => screen = Screen::AutoPower,
                b'd' | b'D' => screen = Screen::AutoTemps,
                b'f' | b'F' => screen = Screen::AutoObd,
                _ => {}
            }
        }

        let t = t0.elapsed().as_secs_f32();
        let mut page = Page::new(&mut panel);
        page.font_px = if w.min(h) >= 700 { 16.0 } else { 13.0 };

        match screen {
            Screen::Sms => jet::sms(&mut page, ((t * 0.7) as usize) % 9, t.sin() > 0.0),
            Screen::Hsd => jet::hsd(&mut page, (t * 25.0) % 360.0, 20.0 + 15.0 * t.sin().abs()),
            Screen::Tgp => jet::tgp(
                &mut page,
                0.5 + 0.3 * (t * 0.8).sin(),
                0.5 + 0.25 * (t * 0.6).cos(),
                t.sin() > 0.7,
            ),
            Screen::Fcr => jet::fcr(
                &mut page,
                0.5 + 0.4 * (t * 0.5).sin(),
                0.3 + 0.4 * (t * 0.35).cos().abs(),
            ),
            Screen::Eng => jet::eng(
                &mut page,
                0.55 + 0.25 * (t * 0.7).sin(),
                0.4 + 0.2 * (t * 0.5).cos(),
                0.45 + 0.1 * (t * 0.3).sin(),
                0.5 + 0.15 * (t * 0.4).cos(),
            ),
            Screen::Fuel => jet::fuel(
                &mut page,
                0.7 + 0.1 * (t * 0.1).cos(),
                0.55 + 0.08 * (t * 0.12).sin(),
                0.3 + 0.05 * (t * 0.08).cos(),
            ),
            Screen::Dte => jet::dte(
                &mut page,
                &[
                    "LOAD 1  READY",
                    "LOAD 2  READY",
                    "WP LIST  12",
                    "DTC  MOUNTED",
                    "COMM  OK",
                ],
            ),
            Screen::Test => jet::test(&mut page, true),
            Screen::AutoCluster | Screen::AutoPower | Screen::AutoTemps | Screen::AutoObd => {
                let obd = ObdSnapshot {
                    rpm: 0.2 + 0.55 * (0.5 + 0.5 * (t * 0.6).sin()),
                    speed: 0.3 + 0.4 * (0.5 + 0.5 * (t * 0.35).sin()),
                    fuel: 0.62 + 0.08 * (t * 0.1).cos(),
                    coolant: 0.5 + 0.1 * (t * 0.15).sin(),
                    trans_temp: 0.4 + 0.12 * (t * 0.2).cos(),
                    battery: 0.55 + 0.05 * (t * 0.25).sin(),
                    throttle: 0.3 + 0.4 * (0.5 + 0.5 * (t * 0.8).sin()),
                    load: 0.35 + 0.3 * (0.5 + 0.5 * (t * 0.55).cos()),
                    dtc_count: 0,
                };
                match screen {
                    Screen::AutoCluster => auto::cluster(&mut page, &obd),
                    Screen::AutoPower => auto::power(&mut page, &obd),
                    Screen::AutoTemps => auto::temps(&mut page, &obd),
                    Screen::AutoObd => auto::obd_status(&mut page, &obd),
                    _ => {}
                }
            }
        }

        present_at(&panel, backend, vp)?;
        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }
    }

    leave_fullscreen()?;
    eprintln!("mfd-demo done · libmfd {ver}");
    Ok(())
}

fn poll_key() -> io::Result<Option<u8>> {
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
