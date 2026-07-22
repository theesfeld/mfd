//! **MFD demo** — jet CMFD formats + **full auto vehicle** pages.
//!
//! Default domain is **auto** (vehicle MFD). Jet remains on Tab / `j`.
//!
//! ```text
//! cargo run --release --bin mfd-demo
//! MFD_DOMAIN=jet cargo run --release --bin mfd-demo
//! MFD_CAMERA=auto cargo run --release --bin mfd-demo
//! MFD_OBD_PORT=/dev/ttyUSB0 cargo run --release --bin mfd-demo
//! ```

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use mfd::auto::{self, AutoPage, DriveMode, GearSelect, VehicleSnapshot};
use mfd::bezel::{BezelEvent, BezelSource, BezelState, KeyboardBezel};
use mfd::font::{draw_text, text_width};
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

    // Default: auto vehicle showcase (override MFD_DOMAIN=jet).
    let mut domain = match std::env::var("MFD_DOMAIN")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jet" | "f16" | "cmfd" => Domain::Jet,
        _ => Domain::Auto,
    };

    let mut fmt_sel = FormatSelect::default();
    let mut jet_fmt = fmt_sel.current();
    let mut auto_page = match std::env::var("MFD_AUTO_PAGE")
        .unwrap_or_default()
        .to_ascii_uppercase()
        .as_str()
    {
        "FUEL" => AutoPage::Fuel,
        "TEMP" | "TEMPS" => AutoPage::Temps,
        "DRV" | "DRIVE" => AutoPage::Drive,
        "LITE" | "LIGHTS" => AutoPage::Lights,
        "TPM" => AutoPage::Tpm,
        "BODY" => AutoPage::Body,
        "CLIM" | "CLIMATE" => AutoPage::Clim,
        "FLIR" | "CAM" => AutoPage::Flir,
        "RNG" | "RANGE" | "COLL" => AutoPage::Collision,
        "ATT" | "ATTITUDE" => AutoPage::Attitude,
        "MAP" | "TOPO" => AutoPage::Map,
        "OBD" => AutoPage::Obd,
        "SET" | "SETUP" => AutoPage::Setup,
        _ => AutoPage::Cluster,
    };
    let mut vehicle = VehicleSnapshot::default();
    let mut color_mode = ColorMode::ColorMfd;
    let mut osb_tick: u32 = 0;

    #[cfg(feature = "obd")]
    let obd_feed = auto::obd_feed::ObdFeed::try_start_from_env();
    #[cfg(feature = "obd")]
    let obd_status = if let Some(ref f) = obd_feed {
        let s = f.status_line();
        eprintln!("OBD: {s}");
        s
    } else {
        eprintln!("OBD: DEMO (set MFD_OBD_PORT or MFD_OBD_REPLAY)");
        "DEMO".into()
    };
    #[cfg(not(feature = "obd"))]
    let obd_status = {
        eprintln!("OBD: off (build --features obd)");
        "OFF".to_string()
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

    eprintln!(
        "start: {} · auto page {}",
        match domain {
            Domain::Auto => "AUTO vehicle MFD",
            Domain::Jet => "JET CMFD",
        },
        auto_page.title()
    );

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
                    eprintln!(
                        "domain → {}",
                        match domain {
                            Domain::Auto => "AUTO",
                            Domain::Jet => "JET",
                        }
                    );
                }
                b'a' | b'A' => {
                    domain = Domain::Auto;
                    eprintln!("domain → AUTO");
                }
                b'j' | b'J' => {
                    domain = Domain::Jet;
                    eprintln!("domain → JET");
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
                b'm' | b'M' if matches!(domain, Domain::Jet) => {
                    osb_tick = osb_tick.wrapping_add(1);
                    let _ = fmt_sel.handle_osb(fmt_sel.active.osb(), osb_tick);
                    jet_fmt = fmt_sel.current();
                }
                // Auto page jump keys (always switch to auto)
                b'1' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Cluster;
                }
                b'2' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Fuel;
                }
                b'3' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Temps;
                }
                b'4' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Drive;
                }
                b'5' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Lights;
                }
                b'6' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Tpm;
                }
                b'7' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Body;
                }
                b'8' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Clim;
                }
                b'9' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Flir;
                }
                b'0' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Collision;
                }
                b'o' | b'O' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Obd;
                }
                b's' | b'S' if matches!(domain, Domain::Auto) => {
                    auto_page = AutoPage::Setup;
                }
                b'u' | b'U' if matches!(domain, Domain::Auto) => {
                    vehicle.speed_unit = vehicle.speed_unit.cycle();
                }
                // Page cycle — do not steal [ ] (real CMFD BRT rocker).
                b'n' | b'N' if matches!(domain, Domain::Auto) => {
                    auto_page = cycle_auto(auto_page, 1);
                }
                b'p' | b'P' if matches!(domain, Domain::Auto) => {
                    auto_page = cycle_auto(auto_page, -1);
                }
                b'h' | b'H' if matches!(domain, Domain::Auto) => {
                    auto_page = AutoPage::Setup;
                }
                // Attitude / map jump
                b'v' | b'V' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Attitude;
                }
                b'x' | b'X' => {
                    domain = Domain::Auto;
                    auto_page = AutoPage::Map;
                }
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
                match domain {
                    Domain::Jet => {
                        osb_tick = osb_tick.wrapping_add(1);
                        match fmt_sel.handle_osb(osb, osb_tick) {
                            FormatSelectAction::Show(f) => jet_fmt = f,
                            FormatSelectAction::OpenMenu { .. } => jet_fmt = Format::Menu,
                            FormatSelectAction::CloseMenu => jet_fmt = fmt_sel.current(),
                            FormatSelectAction::Ignore => {
                                if !fmt_sel.menu_open {
                                    if let Some(f) = Format::from_top_osb(osb, 0) {
                                        fmt_sel.assign(fmt_sel.active, f);
                                        jet_fmt = f;
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

        #[cfg(target_os = "linux")]
        let cam_frame = if matches!(domain, Domain::Auto)
            && matches!(auto_page, AutoPage::Flir | AutoPage::Collision)
        {
            // Grab on FLIR; keep last frame warm for Collision if needed later
            if matches!(auto_page, AutoPage::Flir) && frame_i % 2 == 0 {
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
                jet::draw_format_sel(&mut page, jet_fmt, &pal, &bezel, t, Some(&fmt_sel));
            }
            Domain::Auto => {
                let font_px = page.font_px;
                auto::draw_auto_with_video(
                    &mut page,
                    auto_page,
                    &pal,
                    &bezel,
                    &vehicle,
                    t,
                    cam_frame.as_ref(),
                );
                let feed = if use_obd { "OBD" } else { "DEMO" };
                let cam = if cam_frame.is_some() {
                    "CAM"
                } else if std::env::var_os("MFD_FLIR_PATH").is_some() {
                    "FILE"
                } else {
                    #[cfg(target_os = "linux")]
                    {
                        if camera.is_some() {
                            "CAM?"
                        } else {
                            "SYN"
                        }
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        "SYN"
                    }
                };
                let status = format!(
                    "AUTO {} · {} · {} · n/p · [ ] BRT · Tab jet",
                    auto_page.title(),
                    feed,
                    cam
                );
                draw_demo_status(page.surface, &status, pal.dim, font_px * 0.6);
            }
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
    eprintln!("mfd-demo done · libmfd {ver}");
    Ok(())
}

fn cycle_auto(cur: AutoPage, dir: i32) -> AutoPage {
    let all = AutoPage::ALL;
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
    eprintln!("  mfd-demo  libmfd {ver}");
    eprintln!("  Vehicle MFD (default) + jet CMFD");
    eprintln!("═══════════════════════════════════════════════════════════");
    eprintln!();
    eprintln!("  DOMAIN");
    eprintln!("    Tab / a     AUTO vehicle pages   (default)");
    eprintln!("    j           JET F-16 formats");
    eprintln!();
    eprintln!("  AUTO PAGES  (keys or OSB)");
    eprintln!("    1 CLST   cluster  speed/RPM/gear/throttle");
    eprintln!("    2 FUEL   fuel + battery + load tapes");
    eprintln!("    3 TEMP   oil/coolant/trans/IAT/MAF/EGT");
    eprintln!("    4 DRV    gear P/R/N/D/M · 2H/4H/4L");
    eprintln!("    5 LITE   headlights fog brake turns interior");
    eprintln!("    6 TPM    tire pressures + temps");
    eprintln!("    7 BODY   doors + seat belts");
    eprintln!("    8 CLIM   out/in temp HVAC");
    eprintln!("    9 FLIR   camera / FLIR glass");
    eprintln!("    0 RNG    collision / park ranges");
    eprintln!("    v ATT    attitude ball + heading N/NW/… + degrees");
    eprintln!("    x MAP    schematic line/topo (not full DEM)");
    eprintln!("    o OBD    PID list");
    eprintln!("    s SET    setup / units");
    eprintln!("    n / p    next / previous auto page");
    eprintln!("    u        cycle speed unit MPH/KM/H/KT");
    eprintln!();
    eprintln!("  OSB (auto)");
    eprintln!("    top     CLST FUEL TEMP DRV LITE");
    eprintln!("    right   TPM  BODY CLIM FLIR RNG");
    eprintln!("    left    OBD  SET  ATT  MAP  …");
    eprintln!();
    eprintln!("  BEZEL (real CMFD rockers)");
    eprintln!("    [ ]     BRT brightness −/+   (yes — on real MFD)");
    eprintln!("    ; '     CON contrast −/+");
    eprintln!("    - =     SYM symbology −/+");
    eprintln!("    , .     GAIN −/+");
    eprintln!();
    eprintln!("  JET");
    eprintln!("    OSB 12/13/14 format slots · m Master Menu · g widget QA");
    eprintln!();
    eprintln!("  SENSORS");
    eprintln!("    MFD_CAMERA=/dev/video0|auto");
    eprintln!("    MFD_FLIR_PATH=still.pgm");
    eprintln!("    MFD_OBD_PORT=/dev/ttyUSB0  MFD_OBD_REPLAY=…");
    eprintln!("    MFD_RANGE=2.1,3.0,2.8,1.2");
    eprintln!("    MFD_DOMAIN=auto|jet   MFD_AUTO_PAGE=ATT|MAP|FLIR|…");
    eprintln!("    c color · Esc quit");
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
