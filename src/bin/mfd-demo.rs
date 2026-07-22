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

use mfd::auto::{self, AutoPage, DriveMode, GearSelect, VehicleSnapshot};
use mfd::bezel::{BezelEvent, BezelSource, BezelState, KeyboardBezel};
use mfd::frame::FramePacer;
use mfd::jet::{self, Format, FormatSelect, FormatSelectAction};
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
    eprintln!("loaded libmfd {ver} · MLU CMFD 4x4 · jet + auto");
    eprintln!("Tab jet/auto · c color · g widget-QA · Esc quit");
    eprintln!("JET: OSB 12/13/14 format select · m = Master Menu");
    eprintln!("AUTO: top CLST…LITE · right TPM/BODY/CLIM/FLIR/RNG · left OBD/SET");
    eprintln!("CAM: MFD_CAMERA=/dev/video0|auto  FLIR: MFD_FLIR_PATH=still.pgm");
    eprintln!("OBD: MFD_OBD_PORT=/dev/ttyUSB0  or  MFD_OBD_REPLAY=capture.jsonl");

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
    // MLU M1: three format options on OSB 14/13/12; start FCR active.
    let mut fmt_sel = FormatSelect::default();
    let mut jet_fmt = fmt_sel.current();
    let mut auto_page = AutoPage::Cluster;
    let mut vehicle = VehicleSnapshot::default();
    let mut color_mode = ColorMode::ColorMfd;
    let mut osb_tick: u32 = 0;

    #[cfg(feature = "obd")]
    let obd_feed = auto::obd_feed::ObdFeed::try_start_from_env();
    #[cfg(feature = "obd")]
    if let Some(ref f) = obd_feed {
        eprintln!("OBD feed: {}", f.status_line());
    } else {
        eprintln!("OBD feed: demo (set MFD_OBD_PORT or MFD_OBD_REPLAY for live)");
    }
    #[cfg(not(feature = "obd"))]
    eprintln!("OBD feed: disabled (build with --features obd)");

    #[cfg(target_os = "linux")]
    let mut camera = {
        use mfd::V4l2Source;
        let cam = V4l2Source::auto_detect().or_else(V4l2Source::from_env);
        if let Some(ref c) = cam {
            eprintln!("camera: {}", c.device.display());
        } else {
            eprintln!("camera: none (MFD_CAMERA=/dev/video0 or auto)");
        }
        cam
    };

    enter_fullscreen()?;
    let t0 = Instant::now();
    let mut keybuf = Vec::with_capacity(32);
    let mut frame_i = 0u32;

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
                b'g' | b'G' => {
                    domain = Domain::Jet;
                    jet_fmt = Format::Gallery;
                }
                b'm' | b'M' => {
                    // Open Master Menu on active slot (same as press highlighted format OSB).
                    domain = Domain::Jet;
                    osb_tick = osb_tick.wrapping_add(1);
                    let _ = fmt_sel.handle_osb(fmt_sel.active.osb(), osb_tick);
                    jet_fmt = fmt_sel.current();
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
                        osb_tick = osb_tick.wrapping_add(1);
                        // 1) Format select / Master Menu (OSB 12/13/14 + menu picks).
                        match fmt_sel.handle_osb(osb, osb_tick) {
                            FormatSelectAction::Show(f) => jet_fmt = f,
                            FormatSelectAction::OpenMenu { .. } => jet_fmt = Format::Menu,
                            FormatSelectAction::CloseMenu => jet_fmt = fmt_sel.current(),
                            FormatSelectAction::Ignore => {
                                // 2) Page-local OSBs only when not format-select.
                                // Top row and sides are format-specific (rotary / CNTL / etc.).
                                // Demo shortcuts when not on menu:
                                if !fmt_sel.menu_open {
                                    match osb {
                                        1..=5 | 6..=11 | 15..=20 => {
                                            // Page owns these; optional demo: top bank still switches
                                            // via format assign for exploration.
                                            if let Some(f) = Format::from_top_osb(osb, 0) {
                                                fmt_sel.assign(fmt_sel.active, f);
                                                jet_fmt = f;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Domain::Auto => {
                        if let Some(p) = AutoPage::from_top_osb(osb) {
                            auto_page = p;
                        } else if let Some(p) = AutoPage::from_right_osb(osb) {
                            auto_page = p;
                        } else if let Some(p) = AutoPage::from_left_osb(osb) {
                            auto_page = p;
                        } else {
                            match (auto_page, osb) {
                                (AutoPage::Cluster | AutoPage::Setup, 11 | 15) => {
                                    vehicle.speed_unit = vehicle.speed_unit.cycle();
                                }
                                (AutoPage::Drive, 11) => vehicle.gear = GearSelect::Park,
                                (AutoPage::Drive, 12) => vehicle.gear = GearSelect::Reverse,
                                (AutoPage::Drive, 13) => vehicle.gear = GearSelect::Neutral,
                                (AutoPage::Drive, 14) => vehicle.gear = GearSelect::Drive,
                                (AutoPage::Drive, 15) => vehicle.gear = GearSelect::Manual,
                                (AutoPage::Drive, 16) => vehicle.drive = DriveMode::TwoHigh,
                                (AutoPage::Drive, 17) => vehicle.drive = DriveMode::FourHigh,
                                (AutoPage::Drive, 18) => vehicle.drive = DriveMode::FourLow,
                                (AutoPage::Lights, 11) => {
                                    vehicle.light_interior = !vehicle.light_interior
                                }
                                (AutoPage::Lights, 12) => {
                                    vehicle.light_drive = !vehicle.light_drive
                                }
                                (AutoPage::Lights, 13) => vehicle.light_fog = !vehicle.light_fog,
                                (AutoPage::Lights, 14) => vehicle.light_high = !vehicle.light_high,
                                (AutoPage::Lights, 15) => vehicle.light_low = !vehicle.light_low,
                                (AutoPage::Clim, 16) => vehicle.hvac_ac = !vehicle.hvac_ac,
                                (AutoPage::Clim, 17) => {
                                    vehicle.hvac_fan = (vehicle.hvac_fan + 0.1).min(1.0)
                                }
                                (AutoPage::Clim, 18) => {
                                    vehicle.hvac_defrost = !vehicle.hvac_defrost
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        let t = t0.elapsed().as_secs_f32();
        frame_i = frame_i.wrapping_add(1);

        // Vehicle: OBD live if available, else animated demo; keep operator toggles.
        #[cfg(feature = "obd")]
        let use_obd = obd_feed.is_some();
        #[cfg(not(feature = "obd"))]
        let use_obd = false;

        if use_obd {
            #[cfg(feature = "obd")]
            if let Some(ref feed) = obd_feed {
                feed.apply_to(&mut vehicle);
            }
        } else {
            let mut live = auto::demo_vehicle(t);
            live.speed_unit = vehicle.speed_unit;
            live.gear = vehicle.gear;
            live.gear_num = vehicle.gear_num;
            live.drive = vehicle.drive;
            live.light_low = vehicle.light_low;
            live.light_high = vehicle.light_high;
            live.light_drive = vehicle.light_drive;
            live.light_fog = vehicle.light_fog;
            live.light_interior = vehicle.light_interior;
            live.hvac_ac = vehicle.hvac_ac;
            live.hvac_defrost = vehicle.hvac_defrost;
            live.hvac_fan = vehicle.hvac_fan;
            vehicle = live;
        }

        // Camera: grab every few frames to keep present snappy
        #[cfg(target_os = "linux")]
        let cam_frame = if matches!(domain, Domain::Auto) && matches!(auto_page, AutoPage::Flir) {
            if frame_i % 3 == 0 {
                camera.as_mut().and_then(|c| c.grab().cloned())
            } else {
                camera.as_ref().and_then(|c| c.last.clone())
            }
        } else {
            None
        };
        #[cfg(not(target_os = "linux"))]
        let cam_frame: Option<mfd::GreyFrame> = None;

        let pal = Palette::new(color_mode);
        let mut page = Page::new(&mut panel);
        page.font_px = if w.min(h) >= 480 { 14.0 } else { 12.0 };

        match domain {
            Domain::Jet => {
                jet::draw_format_sel(&mut page, jet_fmt, &pal, &bezel, t, Some(&fmt_sel))
            }
            Domain::Auto => {
                auto::draw_auto_with_video(
                    &mut page,
                    auto_page,
                    &pal,
                    &bezel,
                    &vehicle,
                    t,
                    cam_frame.as_ref(),
                );
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
