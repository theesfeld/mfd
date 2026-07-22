//! **MFD demo** — square face, OSB bezel, F-16 formats + auto.
//!
//! Physical reference: F-16 MLU color MFD ≈ **4×4 in (10×10 cm)** square glass.
//! This demo uses a **square** framebuffer + centered terminal viewport.
//!
//! ```text
//! cargo run --release --bin mfd-demo
//! ```

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use mfd::auto::{self, AutoPage, ObdSnapshot};
use mfd::bezel::{BezelEvent, BezelSource, BezelState, KeyboardBezel};
use mfd::frame::FramePacer;
use mfd::jet::{self, Format};
use mfd::page::Page;
use mfd::palette::{ColorMode, Palette};
use mfd::term::{
    detect_backend, enter_fullscreen, leave_fullscreen, present_at_state_scratch,
    square_mfd_pixels, square_mfd_viewport, PresentScratch, RawStdin,
};
use mfd::{engine_version, using_assembly, Surface};

static RUNNING: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Domain {
    Jet,
    Auto,
}

fn main() -> io::Result<()> {
    let ver = engine_version();
    if !using_assembly() {
        eprintln!("error: mfd-demo requires pure-asm libmfd (x86_64)");
        std::process::exit(1);
    }
    eprintln!("loaded libmfd {ver} · square MFD face (~4x4 in class)");
    eprintln!("OSB 1-5 top · 6-0 right · qwert bot · asdfg left · [ ] BRT");
    eprintln!("Tab jet/auto · c color · / bank · Esc quit");

    install_sigint();
    // 30 Hz default keeps Kitty present from queuing into multi-second lag.
    let hz = std::env::var("MFD_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30u32);

    let backend = detect_backend();
    let vp = square_mfd_viewport(0.85);
    let (w, h) = square_mfd_pixels(backend);
    eprintln!(
        "surface {w}x{h}  viewport cells {}x{} @{},{}",
        vp.cols, vp.rows, vp.col, vp.row
    );

    let mut panel = Surface::new(w, h);
    let mut scratch = PresentScratch::default();
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };

    let raw = match RawStdin::enter() {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("warning: raw stdin ({e})");
            None
        }
    };

    let mut bezel_src = KeyboardBezel::new();
    let mut bezel = BezelState::default();
    let mut domain = Domain::Jet;
    // Start on ENG so tapes + round gauges are obvious (not only HSD rings).
    let mut jet_fmt = Format::Eng;
    let mut jet_bank = 0usize;
    let mut auto_page = AutoPage::Cluster;
    let mut color_mode = ColorMode::ColorMfd;

    enter_fullscreen()?;
    let t0 = Instant::now();
    let mut keybuf = Vec::with_capacity(32);

    while RUNNING.load(Ordering::Relaxed) {
        keybuf.clear();
        if let Some(ref raw) = raw {
            raw.read_keys(&mut keybuf)?;
        }

        for &k in &keybuf {
            match k {
                0x1b => RUNNING.store(false, Ordering::Relaxed),
                b'\t' => {
                    domain = match domain {
                        Domain::Jet => Domain::Auto,
                        Domain::Auto => Domain::Jet,
                    };
                }
                b'c' | b'C' => {
                    color_mode = match color_mode {
                        ColorMode::GreenMono => ColorMode::ColorMfd,
                        ColorMode::ColorMfd => ColorMode::HighVis,
                        ColorMode::HighVis => ColorMode::GreenMono,
                    };
                }
                b'/' => jet_bank = jet_bank.wrapping_add(1),
                _ => bezel_src.push_key_state(k, &bezel),
            }
        }
        if !RUNNING.load(Ordering::Relaxed) {
            break;
        }

        for ev in bezel_src.poll() {
            bezel.apply(ev);
            if let BezelEvent::OsbDown(osb) = ev {
                match domain {
                    Domain::Jet => {
                        if let Some(f) = Format::from_top_osb(osb, jet_bank) {
                            jet_fmt = f;
                        } else {
                            match osb {
                                11 => jet_fmt = Format::Cni,
                                12 => jet_fmt = Format::Fuel,
                                13 => jet_fmt = Format::Eng,
                                14 => jet_fmt = Format::Test,
                                15 => jet_fmt = Format::Dte,
                                16 => jet_fmt = Format::Blank,
                                17 => jet_fmt = Format::HudRpt,
                                18 => jet_fmt = Format::Ecm,
                                19 => jet_fmt = Format::Flir,
                                20 => jet_fmt = Format::Had,
                                6 => jet_fmt = Format::FcrGm,
                                7 => jet_fmt = Format::FcrSea,
                                8 => jet_fmt = Format::Stores,
                                9 => jet_fmt = Format::Ufc,
                                10 => jet_fmt = Format::Pfl,
                                _ => {}
                            }
                        }
                    }
                    Domain::Auto => {
                        if let Some(p) = AutoPage::from_top_osb(osb) {
                            auto_page = p;
                        }
                    }
                }
            }
        }

        let t = t0.elapsed().as_secs_f32();
        let pal = Palette::new(color_mode);
        let mut page = Page::new(&mut panel);
        page.font_px = if w.min(h) >= 480 { 14.0 } else { 12.0 };

        match domain {
            Domain::Jet => jet::draw_format(&mut page, jet_fmt, &pal, &bezel, t),
            Domain::Auto => {
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
                auto::draw_auto(&mut page, auto_page, &pal, &bezel, &obd);
            }
        }

        // Real BRT: scale ink after draw.
        panel.apply_brightness(bezel.brightness.clamp(0.05, 1.0));

        present_at_state_scratch(&panel, backend, vp, None, Some(&mut scratch))?;
        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }
    }

    leave_fullscreen()?;
    drop(raw);
    eprintln!("mfd-demo done · libmfd {ver}");
    Ok(())
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
