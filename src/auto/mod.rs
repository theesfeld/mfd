//! **Automotive** reuse of MFD widgets + OBD-II oriented stubs.
//!
//! | Aviation widget | Automotive use |
//! |-----------------|----------------|
//! | `tape_gauge` | Fuel, coolant, oil temp, transmission temp |
//! | `round_gauge` | RPM tach, boost, speedo (if round) |
//! | softkeys | Drive mode / page select |
//! | labels | PID readouts |
//!
//! OBD-II here is a **call shape** for values (mode 01 PIDs). Wire real ELM/STN later.

use crate::color::{AMBER, CYAN, GREEN, WHITE};
use crate::geom::Rect;
use crate::page::Page;
use crate::widget::{round_gauge, softkey_row, tape_gauge};
use crate::widget::{RoundGaugeOpts, SoftkeyLayout, TapeOpts, TapeOrientation};

/// Common auto cluster softkeys.
pub const AUTO_PAGES: &[&str] = &["CLUSTER", "POWER", "TEMP", "FUEL", "OBD", "SETUP"];

/// Normalized OBD-style snapshot (0..1 or engineering units as noted).
#[derive(Clone, Debug, Default)]
pub struct ObdSnapshot {
    /// RPM 0..1 (map from 0..redline externally).
    pub rpm: f32,
    /// Vehicle speed 0..1 (0..max_speed).
    pub speed: f32,
    /// Fuel level 0..1.
    pub fuel: f32,
    /// Coolant 0..1.
    pub coolant: f32,
    /// Transmission fluid temp 0..1.
    pub trans_temp: f32,
    /// Battery / system voltage 0..1 (e.g. 10..16 V mapped).
    pub battery: f32,
    /// Throttle position 0..1.
    pub throttle: f32,
    /// Engine load 0..1.
    pub load: f32,
    /// Optional DTC count.
    pub dtc_count: u32,
}

/// Driver cluster page: tach (round) + speed + fuel/cool tapes.
pub fn cluster(page: &mut Page, obd: &ObdSnapshot) {
    page.clear();
    page.bezel();
    let b = page.bounds.inset(4);
    let th = (page.font_px * 1.4) as i32;
    softkey_row(
        page.surface,
        Rect::new(b.x, b.y, b.w, th),
        AUTO_PAGES,
        SoftkeyLayout {
            font_px: page.font_px,
            selected: Some(0),
        },
    );
    let c = Rect::new(b.x, b.y + th + 4, b.w, b.h - 2 * th - 8);
    // Large tach left
    let tach_w = (c.w as f32 * 0.55) as i32;
    round_gauge(
        page.surface,
        Rect::new(c.x, c.y, tach_w, c.h),
        RoundGaugeOpts {
            value: obd.rpm,
            redline: Some(0.78),
            label: "RPM",
            font_px: page.font_px,
            ..Default::default()
        },
    );
    // Right stack: speed readout + tapes
    let rx = c.x + tach_w + 8;
    let rw = c.w - tach_w - 12;
    page.label_centered(
        rx as f32 + rw as f32 * 0.5,
        c.y as f32 + page.font_px,
        &format!("SPD {:.0}", obd.speed * 160.0),
        WHITE,
    );
    let tape_h = (c.h - (page.font_px as i32) * 3) / 2;
    tape_gauge(
        page.surface,
        Rect::new(rx, c.y + (page.font_px as i32) * 2, rw / 2 - 4, tape_h),
        TapeOpts {
            orientation: TapeOrientation::Vertical,
            font_px: page.font_px * 0.8,
            color: AMBER,
            value: obd.fuel,
            label: "FUEL",
        },
    );
    tape_gauge(
        page.surface,
        Rect::new(
            rx + rw / 2,
            c.y + (page.font_px as i32) * 2,
            rw / 2 - 4,
            tape_h,
        ),
        TapeOpts {
            orientation: TapeOrientation::Vertical,
            font_px: page.font_px * 0.8,
            color: CYAN,
            value: obd.coolant,
            label: "COOL",
        },
    );
    softkey_row(
        page.surface,
        Rect::new(b.x, b.bottom() - th, b.w, th),
        &["P", "R", "N", "D", "S", "M"],
        SoftkeyLayout {
            font_px: page.font_px,
            selected: Some(3),
        },
    );
}

/// Powertrain page: load, throttle, battery.
pub fn power(page: &mut Page, obd: &ObdSnapshot) {
    page.clear();
    page.bezel();
    let b = page.bounds.inset(4);
    let th = (page.font_px * 1.4) as i32;
    softkey_row(
        page.surface,
        Rect::new(b.x, b.y, b.w, th),
        AUTO_PAGES,
        SoftkeyLayout {
            font_px: page.font_px,
            selected: Some(1),
        },
    );
    let c = Rect::new(b.x, b.y + th + 4, b.w, b.h - th - 8);
    let tw = c.w / 3 - 6;
    for (i, (lab, val, col)) in [
        ("LOAD", obd.load, GREEN),
        ("TPS", obd.throttle, CYAN),
        ("BATT", obd.battery, WHITE),
    ]
    .iter()
    .enumerate()
    {
        tape_gauge(
            page.surface,
            Rect::new(c.x + i as i32 * (tw + 6), c.y, tw, c.h),
            TapeOpts {
                orientation: TapeOrientation::Vertical,
                font_px: page.font_px * 0.85,
                color: *col,
                value: *val,
                label: lab,
            },
        );
    }
}

/// Temperature page (coolant + transmission).
pub fn temps(page: &mut Page, obd: &ObdSnapshot) {
    page.clear();
    page.bezel();
    let b = page.bounds.inset(4);
    let th = (page.font_px * 1.4) as i32;
    softkey_row(
        page.surface,
        Rect::new(b.x, b.y, b.w, th),
        AUTO_PAGES,
        SoftkeyLayout {
            font_px: page.font_px,
            selected: Some(2),
        },
    );
    let c = Rect::new(b.x, b.y + th + 4, b.w, b.h - th - 8);
    let tw = c.w / 2 - 8;
    tape_gauge(
        page.surface,
        Rect::new(c.x + 4, c.y, tw, c.h),
        TapeOpts {
            orientation: TapeOrientation::Vertical,
            font_px: page.font_px,
            color: CYAN,
            value: obd.coolant,
            label: "COOLANT",
        },
    );
    tape_gauge(
        page.surface,
        Rect::new(c.x + tw + 12, c.y, tw, c.h),
        TapeOpts {
            orientation: TapeOrientation::Vertical,
            font_px: page.font_px,
            color: AMBER,
            value: obd.trans_temp,
            label: "TRANS",
        },
    );
}

/// OBD status page (PID list stub).
pub fn obd_status(page: &mut Page, obd: &ObdSnapshot) {
    page.clear();
    page.bezel();
    let b = page.bounds.inset(4);
    let th = (page.font_px * 1.4) as i32;
    softkey_row(
        page.surface,
        Rect::new(b.x, b.y, b.w, th),
        AUTO_PAGES,
        SoftkeyLayout {
            font_px: page.font_px,
            selected: Some(4),
        },
    );
    let lines = [
        format!("PID 0C RPM  {:.0}", obd.rpm * 7000.0),
        format!("PID 0D VSS  {:.0} km/h", obd.speed * 160.0),
        format!("PID 2F FUEL {:.0}%", obd.fuel * 100.0),
        format!("PID 05 ECT  {:.0}%", obd.coolant * 100.0),
        format!("PID 11 TPS  {:.0}%", obd.throttle * 100.0),
        format!("PID 04 LOAD {:.0}%", obd.load * 100.0),
        format!("DTC COUNT   {}", obd.dtc_count),
    ];
    for (i, line) in lines.iter().enumerate() {
        let y = b.y as f32 + th as f32 + 8.0 + i as f32 * (page.font_px + 4.0);
        page.label(b.x as f32 + 8.0, y, line, GREEN);
    }
}

/// Map engineering RPM to 0..1 for gauges.
pub fn rpm_norm(rpm: f32, redline: f32) -> f32 {
    if redline <= 0.0 {
        return 0.0;
    }
    (rpm / redline).clamp(0.0, 1.0)
}
