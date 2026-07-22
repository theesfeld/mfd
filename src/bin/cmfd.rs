//! **cmfd** — live vehicle color MFD (systems pages, OBD/UDS, capture).
//!
//! This is the **product glass**, not a toy demo. Offline synthetic data only
//! appears when no adapter is configured (`bus_state = SIM`).
//!
//! Jet **formats** are not in this path; widgets remain in the library for later.
//!
//! ```text
//! ./cmfd.sh
//! cargo run --release --bin cmfd
//! MFD_CAMERA=auto cargo run --release --bin cmfd
//! MFD_OBD_BT=00:04:3E:96:B8:F1 cargo run --release --bin cmfd
//! MFD_OBD_REPLAY=docs/odbii-session cargo run --release --bin cmfd
//! ```

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use mfd::auto::{self, AutoPage, DemoProbe, GearSelect, VehicleSnapshot};
use mfd::bezel::{BezelEvent, BezelSource, BezelState, KeyboardBezel};
use mfd::font::{draw_text, text_width};
use mfd::frame::FramePacer;
use mfd::page::Page;
use mfd::palette::{ColorMode, Palette};
use mfd::term::{
    detect_backend, enter_fullscreen, leave_fullscreen, mfd_face_inches, physical_mfd_layout,
    present_at_state_scratch, PpiSource, PresentScratch, PxSpaceSource, RawStdin,
};
use mfd::warn::WarningEngine;
use mfd::{engine_version, using_assembly, Surface};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> io::Result<()> {
    let ver = engine_version();
    if !using_assembly() {
        eprintln!("error: cmfd requires pure-asm libmfd (x86_64)");
        std::process::exit(1);
    }

    print_banner(ver);

    install_sigint();
    let hz = std::env::var("MFD_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30u32);

    let backend = detect_backend();
    let face = physical_mfd_layout(backend, mfd_face_inches());
    let vp = face.viewport;
    let (w, h) = face.surface_size();
    log_ruler(&face, vp.cols, vp.rows, w, h);
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

    // Vehicle CMFD only (jet formats remain in lib for later).
    let mut auto_page = match std::env::var("MFD_AUTO_PAGE")
        .unwrap_or_default()
        .to_ascii_uppercase()
        .as_str()
    {
        "ENG" | "ENGINE" | "CLST" | "CLUSTER" => AutoPage::Eng,
        "FUEL" => AutoPage::Fuel,
        "FLUD" | "FLUID" | "TEMP" | "TEMPS" => AutoPage::Fluid,
        "ELEC" => AutoPage::Elec,
        "DRV" | "DRIVE" => AutoPage::Drive,
        "CHAS" | "TPM" => AutoPage::Chas,
        "BODY" => AutoPage::Body,
        "LITE" | "LIGHTS" => AutoPage::Lights,
        "CLIM" | "CLIMATE" => AutoPage::Clim,
        "FLIR" | "CAM" => AutoPage::Cam,
        "RNG" | "RANGE" | "COLL" => AutoPage::Range,
        "ATT" | "ATTITUDE" => AutoPage::Attitude,
        "MAP" | "TOPO" => AutoPage::Map,
        "DTC" | "FAULT" | "FAULTS" | "CODES" => AutoPage::Faults,
        "BUS" | "OBD" | "DATA" => AutoPage::Bus,
        "OWN" | "OWN SHIP" => AutoPage::Own,
        "SET" | "SETUP" => AutoPage::Setup,
        _ => AutoPage::Eng,
    };
    let mut vehicle = VehicleSnapshot::default();
    let mut color_mode = ColorMode::ColorMfd;
    let mut osb_tick: u32 = 0;
    // Startup BIT until capability probe finishes (adaptive pages).
    let mut demo_probe = DemoProbe::start();
    let mut boot_done = false;
    let mut available_pages: Vec<AutoPage> = Vec::new();
    let mut warn_eng = WarningEngine::new();

    #[cfg(feature = "obd")]
    let obd_feed = mfd::obd::ObdFeed::try_start_from_env();
    #[cfg(feature = "obd")]
    let obd_status = if let Some(ref f) = obd_feed {
        let s = f.status_line();
        eprintln!("OBD: {s}");
        s
    } else {
        eprintln!("OBD: SIM (set MFD_OBD_BT / MFD_OBD_PORT / MFD_OBD_REPLAY for live truck)");
        "SIM".into()
    };
    #[cfg(not(feature = "obd"))]
    let obd_status = {
        eprintln!("OBD: SIM (obd feature off)");
        "SIM".to_string()
    };

    #[cfg(target_os = "linux")]
    let mut camera = {
        use mfd::V4l2Source;
        let cam = V4l2Source::auto_detect().or_else(V4l2Source::from_env);
        if let Some(ref c) = cam {
            eprintln!("CAM: {}", c.device.display());
        } else {
            eprintln!("CAM: none — MFD_CAMERA=/dev/video0|auto  or  MFD_FLIR_PATH=still.pgm");
        }
        cam
    };
    #[cfg(target_os = "linux")]
    let cam_label = camera
        .as_ref()
        .map(|c| format!("CAM {}", c.device.display()))
        .unwrap_or_else(|| "CAM off".into());
    #[cfg(not(target_os = "linux"))]
    let cam_label = "CAM n/a".to_string();

    eprintln!("start · LIVE vehicle CMFD · page {}", auto_page.title());

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
                b'c' | b'C' if boot_done => {
                    color_mode = match color_mode {
                        ColorMode::GreenMono => ColorMode::ColorMfd,
                        ColorMode::ColorMfd => ColorMode::HighVis,
                        ColorMode::HighVis => ColorMode::GreenMono,
                    };
                }
                // During BIT: only Esc / quit
                _ if !boot_done => {
                    if k != 0x1b {
                        bezel_src.push_key_state(k, &bezel);
                    }
                }
                // Systems page jumps (after BIT)
                b'1' => auto_page = AutoPage::Eng,
                b'2' => auto_page = AutoPage::Fuel,
                b'3' => auto_page = AutoPage::Fluid,
                b'4' => auto_page = AutoPage::Elec,
                b'5' => auto_page = AutoPage::Drive,
                b'6' => auto_page = AutoPage::Chas,
                b'7' => auto_page = AutoPage::Body,
                b'8' => auto_page = AutoPage::Lights,
                b'9' => auto_page = AutoPage::Clim,
                b'0' => auto_page = AutoPage::Cam,
                b'r' | b'R' => auto_page = AutoPage::Range,
                b'b' | b'B' => auto_page = AutoPage::Bus,
                b'w' | b'W' => auto_page = AutoPage::Own,
                b'o' | b'O' => auto_page = AutoPage::Own,
                b's' | b'S' => auto_page = AutoPage::Setup,
                b'u' | b'U' => vehicle.speed_unit = vehicle.speed_unit.cycle(),
                b'n' | b'N' => auto_page = cycle_auto(auto_page, 1, &available_pages),
                b'p' | b'P' => auto_page = cycle_auto(auto_page, -1, &available_pages),
                b'h' | b'H' => auto_page = AutoPage::Setup,
                b'v' | b'V' => auto_page = AutoPage::Attitude,
                b'x' | b'X' => auto_page = AutoPage::Map,
                b'f' | b'F' => auto_page = AutoPage::Faults,
                // [ ] ; ' - = , .  → real bezel knobs (BRT CON SYM GAIN)
                _ => bezel_src.push_key_state(k, &bezel),
            }
        }
        if !RUNNING.load(Ordering::Relaxed) {
            break;
        }

        for ev in bezel_src.poll() {
            bezel.apply(ev);
            if let BezelEvent::OsbDown(osb) = ev {
                if !boot_done {
                    continue;
                }
                osb_tick = osb_tick.wrapping_add(1);
                if let Some(p) = AutoPage::from_top_osb(osb) {
                    if available_pages.is_empty() || available_pages.contains(&p) {
                        auto_page = p;
                    }
                } else if let Some(p) = AutoPage::from_right_osb(osb) {
                    if available_pages.is_empty() || available_pages.contains(&p) {
                        auto_page = p;
                    }
                } else if let Some(p) = AutoPage::from_left_osb(osb) {
                    if available_pages.is_empty() || available_pages.contains(&p) {
                        auto_page = p;
                    }
                } else {
                    match (auto_page, osb) {
                        (AutoPage::Eng | AutoPage::Setup, 11 | 15) => {
                            vehicle.speed_unit = vehicle.speed_unit.cycle();
                        }
                        (AutoPage::Drive, 11) => vehicle.gear = GearSelect::Park,
                        (AutoPage::Drive, 12) => vehicle.gear = GearSelect::Reverse,
                        (AutoPage::Drive, 13) => vehicle.gear = GearSelect::Neutral,
                        (AutoPage::Drive, 14) => vehicle.gear = GearSelect::Drive,
                        (AutoPage::Drive, 15) => vehicle.gear = GearSelect::Manual,
                        (AutoPage::Lights, 11) => vehicle.light_interior = !vehicle.light_interior,
                        (AutoPage::Lights, 12) => vehicle.light_drive = !vehicle.light_drive,
                        (AutoPage::Lights, 13) => vehicle.light_fog = !vehicle.light_fog,
                        (AutoPage::Lights, 14) => vehicle.light_high = !vehicle.light_high,
                        (AutoPage::Lights, 15) => vehicle.light_low = !vehicle.light_low,
                        (AutoPage::Clim, 16) => vehicle.hvac_ac = !vehicle.hvac_ac,
                        (AutoPage::Clim, 17) => {
                            vehicle.hvac_fan = (vehicle.hvac_fan + 0.1).min(1.0)
                        }
                        (AutoPage::Clim, 18) => vehicle.hvac_defrost = !vehicle.hvac_defrost,
                        _ => {}
                    }
                }
            }
        }

        let t = t0.elapsed().as_secs_f32();
        frame_i = frame_i.wrapping_add(1);

        #[cfg(feature = "obd")]
        let use_obd = obd_feed.is_some();
        #[cfg(not(feature = "obd"))]
        let use_obd = false;

        let caps_now = {
            #[cfg(feature = "obd")]
            {
                if let Some(ref feed) = obd_feed {
                    feed.caps()
                } else {
                    demo_probe.tick().clone()
                }
            }
            #[cfg(not(feature = "obd"))]
            {
                demo_probe.tick().clone()
            }
        };
        if !boot_done && caps_now.ready {
            boot_done = true;
            available_pages = caps_now.pages();
            if !available_pages.contains(&auto_page) {
                auto_page = available_pages.first().copied().unwrap_or(AutoPage::Eng);
            }
            eprintln!(
                "BIT COMPLETE · {} pages · {}",
                available_pages.len(),
                caps_now.link
            );
        }

        if use_obd {
            #[cfg(feature = "obd")]
            if let Some(ref feed) = obd_feed {
                feed.apply_to(&mut vehicle);
            }
        } else if boot_done {
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

        #[cfg(target_os = "linux")]
        let cam_frame = if boot_done && matches!(auto_page, AutoPage::Cam) {
            if frame_i % 2 == 0 {
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

        if !boot_done {
            auto::draw_bit_screen(&mut page, &pal, &caps_now, t);
        } else {
            let font_px = page.font_px;
            // Offline SIM only: force brief alerts so flash/bingo can be checked
            // without a truck. Live OBD path never invents faults.
            if !use_obd {
                if (t as i32 % 20) < 4 {
                    vehicle.park_brake = true;
                    vehicle.speed_mph = vehicle.speed_mph.max(8.0);
                }
                if vehicle.fuel < 0.20 {
                    vehicle.fuel = vehicle.fuel.min(0.12);
                }
            }
            let active = warn_eng.tick(&vehicle);
            auto::draw_auto_with_video(
                &mut page,
                auto_page,
                &pal,
                &bezel,
                &vehicle,
                t,
                cam_frame.as_ref(),
                Some(&caps_now),
                Some(&active),
            );
            let cam = if cam_frame.is_some() { "CAM" } else { "SYN" };
            let aw = if active.is_empty() {
                String::new()
            } else {
                format!(" · {}", active[0].label)
            };
            // Live BT / bus link on every page (OWN has the full block).
            let link = vehicle.bus_status_short();
            let status = format!("{} · {link} · {}{aw} · o=OWN", auto_page.title(), cam);
            draw_demo_status(page.surface, &status, pal.dim, font_px * 0.55);
        }

        panel.apply_brightness(bezel.brightness.clamp(0.05, 1.0));
        present_at_state_scratch(&panel, backend, vp, None, Some(&mut scratch))?;
        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }
    }

    leave_fullscreen()?;
    drop(raw);
    let _ = (obd_status.as_str(), cam_label.as_str());
    eprintln!("cmfd done · libmfd {ver}");
    Ok(())
}

fn cycle_auto(cur: AutoPage, dir: i32, pages: &[AutoPage]) -> AutoPage {
    let all = if pages.is_empty() {
        AutoPage::ALL
    } else {
        pages
    };
    let i = all.iter().position(|&p| p == cur).unwrap_or(0) as i32;
    let n = all.len() as i32;
    let j = (i + dir).rem_euclid(n) as usize;
    all[j]
}

fn draw_demo_status(s: &mut Surface, text: &str, color: mfd::Color, px: f32) {
    let w = s.width() as f32;
    let h = s.height() as f32;
    let tw = text_width(text, px);
    let x = ((w - tw) * 0.5).max(2.0);
    let y = h - px - 4.0;
    draw_text(s, x, y, text, color, px);
}

fn print_banner(ver: &str) {
    eprintln!("═══════════════════════════════════════════════════════════");
    eprintln!("  cmfd  libmfd {ver}");
    eprintln!("  Vehicle CMFD — LIVE glass · OBD/UDS · display-only");
    eprintln!("═══════════════════════════════════════════════════════════");
    eprintln!();
    eprintln!("  DATA STACK");
    eprintln!("    J1979 OBD-II  ·  UDS/CAN (0x22)  ·  Ford DID / As-Built labels");
    eprintln!();
    eprintln!("  STARTUP");
    eprintln!("    CMFD power-on until capability probe finishes");
    eprintln!();
    eprintln!("  SYSTEMS  n/p  1 ENG 2 FUEL 3 FLUD 4 ELEC 5 DRV 6 CHAS …");
    eprintln!("    b BUS  f DTC  v ATT  x MAP  o OWN  s SET  r RNG");
    eprintln!();
    eprintln!("  LINK");
    eprintln!("    OWN page = Bluetooth MAC · channel · adapter · protocol");
    eprintln!("    Bottom strip = BT LIVE / ERR / SIM on every page");
    eprintln!();
    eprintln!("  WARNINGS (speaker)");
    eprintln!("    BINGO low fuel · ALERT park brake / tire / door");
    eprintln!("    Red flash fields · master caution strip");
    eprintln!("    MFD_AUDIO=0 mute · needs aplay (alsa-utils)");
    eprintln!();
    eprintln!("  BEZEL  [ ] BRT  ·  MFD_OBD_BT=00:04:3E:96:B8:F1");
    eprintln!("  Drive: ./cmfd.sh  ·  c color · Esc quit");
    eprintln!();
}

fn log_ruler(face: &mfd::PhysicalFace, cols: u16, rows: u16, w: u32, h: u32) {
    let src = match face.ppi_source {
        PpiSource::Env => "MFD_PPI",
        PpiSource::EdidDetailed => "EDID-mm",
        PpiSource::EdidCm => "EDID-cm",
        PpiSource::Fallback96 => "fallback-96",
    };
    let pxsrc = match face.pixel_space.source {
        PxSpaceSource::Env => "MFD_PX_SCALE",
        PxSpaceSource::Compositor => "compositor",
        PxSpaceSource::Identity => "identity",
    };
    eprintln!(
        "ruler face {:.2}\" @ {:.1} ppi ({src}) px×{:.3} ({pxsrc}) → {w}×{h}px cells {cols}×{rows} on-glass {:.2}\"{}",
        face.inches_requested,
        face.ppi,
        face.pixel_space.winsize_to_device,
        face.on_glass_in,
        if face.clipped {
            " [clipped]"
        } else {
            ""
        }
    );
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
