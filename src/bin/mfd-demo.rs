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
    detect_backend, enter_fullscreen, leave_fullscreen, mfd_face_inches, physical_mfd_layout,
    present_at_state_scratch, PpiSource, PresentScratch, PxSpaceSource, RawStdin,
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
    eprintln!("Tab jet/auto · c color · / bank · g gallery · m menu · Esc quit");
    eprintln!("start: FCR RWS (public F-16 MFD layout)");

    install_sigint();
    // 30 Hz default keeps Kitty present from queuing into multi-second lag.
    let hz = std::env::var("MFD_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30u32);

    let backend = detect_backend();
    let face = physical_mfd_layout(backend, mfd_face_inches());
    let vp = face.viewport;
    let (w, h) = face.surface_size();
    let src = match face.ppi_source {
        PpiSource::Env => "MFD_PPI",
        PpiSource::EdidDetailed => "EDID-mm",
        PpiSource::EdidCm => "EDID-cm",
        PpiSource::Fallback96 => "fallback-96 (set MFD_PPI for ruler accuracy)",
    };
    let pxsrc = match face.pixel_space.source {
        PxSpaceSource::Env => "MFD_PX_SCALE",
        PxSpaceSource::Compositor => "compositor",
        PxSpaceSource::Identity => "identity",
    };
    eprintln!(
        "ruler face {req:.2}\" @ {ppi:.1} ppi ({src})  px×{pxs:.3} ({pxsrc})  cell {cw:.1}×{ch:.1}dev  → {w}×{h}px  cells {}×{}  on-glass {og:.2}\"×{og:.2}\"{clip}",
        vp.cols,
        vp.rows,
        req = face.inches_requested,
        ppi = face.ppi,
        pxs = face.pixel_space.winsize_to_device,
        cw = face.cell_device.0,
        ch = face.cell_device.1,
        og = face.on_glass_in,
        clip = if face.clipped {
            "  [clipped to window — enlarge terminal or lower MFD_FACE_IN]"
        } else {
            ""
        }
    );
    debug_assert_eq!(w, h, "framebuffer must be square");

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
    // Start on FCR RWS (typical left-MFD air-to-air radar format).
    let mut jet_fmt = Format::Fcr;
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
                b'g' | b'G' => {
                    domain = Domain::Jet;
                    jet_fmt = Format::Gallery;
                }
                b'm' | b'M' => {
                    domain = Domain::Jet;
                    jet_fmt = Format::Menu;
                }
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
                            // Real F-16 OSB map (public Master Menu / format select).
                            match osb {
                                6 => jet_fmt = Format::Sms,
                                7 => jet_fmt = Format::Hsd,
                                8 => jet_fmt = Format::Dte,
                                9 => jet_fmt = Format::Test,
                                10 => jet_fmt = Format::Menu,
                                11 => jet_fmt = Format::Gallery,
                                12 => jet_fmt = Format::Sms,
                                13 => jet_fmt = Format::Hsd,
                                14 => jet_fmt = Format::Fcr,
                                15 => jet_fmt = Format::Menu, // SWAP → menu for demo
                                16 => jet_fmt = Format::Flir,
                                17 => jet_fmt = Format::Tfr,
                                18 => jet_fmt = Format::Wpn,
                                19 => jet_fmt = Format::Tgp,
                                20 => jet_fmt = Format::Fcr,
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
