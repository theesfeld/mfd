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

use mfd::auto::{
    self, AutoFormatSelect, AutoPage, DemoProbe, FormatSelectAction, GearSelect, VehicleSnapshot,
};
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
    let mut fmt_sel = AutoFormatSelect::default();
    let mut warn_eng = WarningEngine::new();
    let mut fog_ok = true;

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

        // Index walk: arrow keys are CSI sequences starting with ESC
        // (`ESC [ A` …). Treating every 0x1b as quit killed the glass.
        let mut ki = 0usize;
        while ki < keybuf.len() {
            let k = keybuf[ki];
            // ESC sequences (arrows, SS3, bare Esc)
            if k == 0x1b {
                if let Some((consumed, act)) = parse_esc_seq(&keybuf[ki..]) {
                    match act {
                        EscAction::Quit => RUNNING.store(false, Ordering::Relaxed),
                        EscAction::PageNext if boot_done => {
                            let next = cycle_auto(auto_page, 1, &available_pages);
                            goto_format(&mut auto_page, &mut fmt_sel, next, &available_pages);
                        }
                        EscAction::PagePrev if boot_done => {
                            let prev = cycle_auto(auto_page, -1, &available_pages);
                            goto_format(&mut auto_page, &mut fmt_sel, prev, &available_pages);
                        }
                        EscAction::Ignore | EscAction::PageNext | EscAction::PagePrev => {}
                    }
                    ki += consumed;
                    continue;
                }
                // Lone ESC → quit
                RUNNING.store(false, Ordering::Relaxed);
                break;
            }
            match k {
                b'c' | b'C' if boot_done => {
                    color_mode = match color_mode {
                        ColorMode::GreenMono => ColorMode::ColorMfd,
                        ColorMode::ColorMfd => ColorMode::HighVis,
                        ColorMode::HighVis => ColorMode::GreenMono,
                    };
                }
                // During BIT: only Esc / quit (arrows ignored above)
                _ if !boot_done => {
                    bezel_src.push_key_state(k, &bezel);
                }
                // Format change (not OSB option keys):
                //   n/p  cycle · m Master Menu · Tab next format slot
                // Dedicated OSB keys always go to bezel (see KeyboardBezel):
                //   1-5 top options · 6-0 right · qwert bottom · asdfg left
                b'n' | b'N' => {
                    let next = cycle_auto(auto_page, 1, &available_pages);
                    goto_format(&mut auto_page, &mut fmt_sel, next, &available_pages);
                }
                b'p' | b'P' => {
                    let prev = cycle_auto(auto_page, -1, &available_pages);
                    goto_format(&mut auto_page, &mut fmt_sel, prev, &available_pages);
                }
                b'm' | b'M' => {
                    let allow = available_pages.as_slice();
                    if !allow.is_empty() {
                        let _ = fmt_sel.handle_osb(fmt_sel.active.osb(), osb_tick, allow);
                    }
                }
                b'u' | b'U' => vehicle.speed_unit = vehicle.speed_unit.cycle(),
                // All OSB / knob keys — never steal for format jumps.
                // 1-5 = top options (Lights LO/HI/FOG…), 6-0 = right,
                // qwert = bottom format slots + DCLT, asdfg = left BUS/SET/DTC.
                _ => bezel_src.push_key_state(k, &bezel),
            }
            ki += 1;
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
                // After boot, empty available_pages must not mean "all formats".
                let allow = available_pages.as_slice();
                if allow.is_empty() {
                    continue;
                }
                match fmt_sel.handle_osb(osb, osb_tick, allow) {
                    FormatSelectAction::Show(p) => {
                        auto_page = p;
                        continue;
                    }
                    FormatSelectAction::OpenMenu { .. } | FormatSelectAction::CloseMenu => {
                        continue;
                    }
                    FormatSelectAction::Own => {
                        auto_page = AutoPage::Own;
                        continue;
                    }
                    FormatSelectAction::Declutter => continue,
                    FormatSelectAction::Ignore => {}
                }
                if let Some(p) = AutoPage::from_left_support_osb(osb) {
                    if AutoFormatSelect::is_allowed(p, allow) {
                        auto_page = p;
                    }
                    continue;
                }
                match (auto_page, osb) {
                    (
                        AutoPage::Eng
                        | AutoPage::Fuel
                        | AutoPage::Fluid
                        | AutoPage::Elec
                        | AutoPage::Drive
                        | AutoPage::Setup,
                        1 | 2,
                    ) => {
                        vehicle.speed_unit = vehicle.speed_unit.cycle();
                    }
                    (AutoPage::Drive, 3) => vehicle.gear = GearSelect::Park,
                    (AutoPage::Drive, 4) => vehicle.gear = GearSelect::Reverse,
                    (AutoPage::Drive, 5) => vehicle.gear = GearSelect::Drive,
                    (AutoPage::Drive, 6) => vehicle.gear = GearSelect::Neutral,
                    (AutoPage::Drive, 7) => vehicle.gear = GearSelect::Manual,
                    (AutoPage::Lights, 1) => vehicle.light_low = !vehicle.light_low,
                    (AutoPage::Lights, 2) => vehicle.light_high = !vehicle.light_high,
                    (AutoPage::Lights, 3) if fog_ok => vehicle.light_fog = !vehicle.light_fog,
                    (AutoPage::Lights, 4) => vehicle.light_drive = !vehicle.light_drive,
                    (AutoPage::Lights, 5) => vehicle.light_interior = !vehicle.light_interior,
                    (AutoPage::Clim, 1) => vehicle.hvac_ac = !vehicle.hvac_ac,
                    (AutoPage::Clim, 2) => vehicle.hvac_fan = (vehicle.hvac_fan + 0.1).min(1.0),
                    (AutoPage::Clim, 3) => vehicle.hvac_defrost = !vehicle.hvac_defrost,
                    _ => {}
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
            fmt_sel = AutoFormatSelect::from_allowed(&available_pages);
            auto_page = fmt_sel.current();
            if !available_pages.is_empty() && !available_pages.contains(&auto_page) {
                auto_page = available_pages[0];
                fmt_sel.assign(fmt_sel.active, auto_page);
            }
            eprintln!(
                "BIT COMPLETE · {} formats · {} · slots {:?}",
                available_pages.len(),
                caps_now.link,
                fmt_sel.slot_labels()
            );
        }
        fog_ok = caps_now.features.fog_lights;

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
                Some(&fmt_sel),
            );
            let cam = if cam_frame.is_some() { "CAM" } else { "SYN" };
            let aw = if active.is_empty() {
                String::new()
            } else {
                format!(" · {}", active[0].label)
            };
            let link = vehicle.bus_status_short();
            let dcl = match fmt_sel.dclt {
                0 => "D0",
                1 => "D1",
                _ => "D2",
            };
            let status = if fmt_sel.menu_open {
                format!("MENU · {link} · {cam}{aw}")
            } else {
                format!("{} · {link} · {dcl} · {cam}{aw}", auto_page.title())
            };
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

/// Result of consuming an ESC / CSI / SS3 sequence from a key buffer.
#[derive(Clone, Copy, Debug)]
enum EscAction {
    Quit,
    PageNext,
    PagePrev,
    Ignore,
}

/// Parse terminal escape sequences starting at `buf[0] == ESC`.
///
/// Returns `(bytes_consumed, action)`. Arrow keys: CSI `ESC [ A/B/C/D` or
/// SS3 `ESC O A/B/C/D` (application cursor mode). Bare Esc → Quit.
fn parse_esc_seq(buf: &[u8]) -> Option<(usize, EscAction)> {
    if buf.is_empty() || buf[0] != 0x1b {
        return None;
    }
    if buf.len() == 1 {
        return Some((1, EscAction::Quit));
    }
    // CSI: ESC [ … final
    if buf[1] == b'[' {
        let mut j = 2usize;
        while j < buf.len() {
            let b = buf[j];
            // CSI params 0x30–0x3F, intermediates 0x20–0x2F, final 0x40–0x7E
            if (0x40..=0x7e).contains(&b) {
                let act = match b {
                    b'A' | b'D' => EscAction::PagePrev, // Up / Left
                    b'B' | b'C' => EscAction::PageNext, // Down / Right
                    // Home/End/… and other CSI — ignore, do not quit
                    _ => EscAction::Ignore,
                };
                return Some((j + 1, act));
            }
            j += 1;
        }
        // Incomplete CSI in this read — swallow ESC+rest so we don't quit
        return Some((buf.len(), EscAction::Ignore));
    }
    // SS3: ESC O A/B/C/D (common for arrows in app mode)
    if buf[1] == b'O' && buf.len() >= 3 {
        let act = match buf[2] {
            b'A' | b'D' => EscAction::PagePrev,
            b'B' | b'C' => EscAction::PageNext,
            _ => EscAction::Ignore,
        };
        return Some((3, act));
    }
    // ESC + other: ignore multi-byte (Alt-key), do not quit
    // Only pure single-byte Esc (handled above) quits.
    Some((2, EscAction::Ignore))
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

/// Jump to a format only if probe-allowed (or pre-boot ALL). No hollow formats.
fn goto_format(
    auto_page: &mut AutoPage,
    fmt_sel: &mut AutoFormatSelect,
    page: AutoPage,
    available: &[AutoPage],
) {
    if available.is_empty() {
        // Pre-boot: allow only defaults
        *auto_page = page;
        return;
    }
    if !AutoFormatSelect::is_allowed(page, available) {
        return;
    }
    *auto_page = page;
    if !matches!(page, AutoPage::Own) {
        fmt_sel.assign(fmt_sel.active, page);
    }
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
    eprintln!("  DEDICATED OSB KEYS (page options stay on the page)");
    eprintln!("    TOP    1 2 3 4 5     options for current format (e.g. Lights LO/HI/FOG)");
    eprintln!("    RIGHT  6 7 8 9 0     options");
    eprintln!("    BOTTOM q w e r t     OWN · fmtA · fmtB · fmtC · DCLT");
    eprintln!("    LEFT   a s d f g     DTC · · · SET · BUS");
    eprintln!();
    eprintln!("  FORMAT CHANGE");
    eprintln!("    n/p or arrows  cycle ·  m  Master Menu ·  e/r (w/e) switch slots");
    eprintln!("    Press active slot (lit) → Master Menu · GO formats only");
    eprintln!();
    eprintln!("  OTHER  u unit · c color · [ ] BRT · Esc quit");
    eprintln!("  Drive: ./cmfd.sh  ·  MFD_OBD_BT=00:04:3E:96:B8:F1");
    eprintln!();
}

#[cfg(test)]
mod esc_tests {
    use super::*;

    #[test]
    fn arrow_up_is_page_prev_not_quit() {
        let (n, a) = parse_esc_seq(b"\x1b[A").unwrap();
        assert_eq!(n, 3);
        assert!(matches!(a, EscAction::PagePrev));
    }

    #[test]
    fn arrow_down_is_page_next() {
        let (n, a) = parse_esc_seq(b"\x1b[B").unwrap();
        assert_eq!(n, 3);
        assert!(matches!(a, EscAction::PageNext));
    }

    #[test]
    fn bare_esc_quits() {
        let (n, a) = parse_esc_seq(b"\x1b").unwrap();
        assert_eq!(n, 1);
        assert!(matches!(a, EscAction::Quit));
    }

    #[test]
    fn ss3_arrow_right() {
        let (n, a) = parse_esc_seq(b"\x1bOC").unwrap();
        assert_eq!(n, 3);
        assert!(matches!(a, EscAction::PageNext));
    }
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
