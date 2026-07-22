//! Automotive pages — **reuse jet widgets + same bezel**.

use crate::bezel::BezelState;
use crate::geom::Rect;
use crate::page::Page;
use crate::palette::Palette;
use crate::widget::{
    caution_box, content_after_osb, list_menu, numeric_readout, osb_chrome, progress_strip,
    round_gauge, softkey_row, tape_gauge, RoundGaugeOpts, SoftkeyLayout, TapeOpts, TapeOrientation,
};

#[derive(Clone, Debug, Default)]
pub struct ObdSnapshot {
    pub rpm: f32,
    pub speed: f32,
    pub fuel: f32,
    pub coolant: f32,
    pub trans_temp: f32,
    pub battery: f32,
    pub throttle: f32,
    pub load: f32,
    pub dtc_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoPage {
    Cluster,
    Power,
    Temps,
    Obd,
    Setup,
}

impl AutoPage {
    pub fn name(self) -> &'static str {
        match self {
            AutoPage::Cluster => "CLUSTER",
            AutoPage::Power => "POWER",
            AutoPage::Temps => "TEMPS",
            AutoPage::Obd => "OBD",
            AutoPage::Setup => "SETUP",
        }
    }

    /// Top OSB 1–5 select auto page (same plug-in bezel as jet).
    pub fn from_top_osb(osb: u8) -> Option<AutoPage> {
        match osb {
            1 => Some(AutoPage::Cluster),
            2 => Some(AutoPage::Power),
            3 => Some(AutoPage::Temps),
            4 => Some(AutoPage::Obd),
            5 => Some(AutoPage::Setup),
            _ => None,
        }
    }
}

fn chrome(page: &mut Page, pal: &Palette, title: &str, bezel: &BezelState) {
    let b = page.bounds.inset(2);
    let top = ["CLST", "PWR", "TMP", "OBD", "SET"];
    let right = ["BRT", "CON", "SYM", "GAIN", "MENU"];
    let bottom = ["P", "R", "N", "D", "S"];
    let left = ["PAGE", "ACK", "CLR", "DTC", "HOME"];
    osb_chrome(
        page.surface,
        b,
        &top,
        &right,
        &bottom,
        &left,
        page.font_px * 0.7,
        pal.dim,
        bezel.last_osb,
    );
    let c = content_after_osb(b, page.font_px * 0.7);
    page.label_centered(
        c.center().0 as f32,
        c.y as f32 + page.font_px * 0.6,
        title,
        pal.primary,
    );
    page.label_at(
        c.x as f32 + 2.0,
        c.bottom() as f32 - page.font_px * 0.9,
        &format!("BRT{:.0}", bezel.brightness * 100.0),
        pal.dim,
        page.font_px * 0.65,
    );
}

fn content(page: &Page) -> Rect {
    let b = page.bounds.inset(2);
    let c = content_after_osb(b, page.font_px * 0.7);
    Rect::new(
        c.x,
        c.y + (page.font_px as i32) + 8,
        c.w,
        (c.h - (page.font_px as i32) * 2 - 12).max(40),
    )
}

pub fn draw_auto(
    page: &mut Page,
    which: AutoPage,
    pal: &Palette,
    bezel: &BezelState,
    obd: &ObdSnapshot,
) {
    page.clear();
    page.surface.clear(pal.glass);
    page.bezel();
    chrome(page, pal, which.name(), bezel);
    let c = content(page);
    match which {
        AutoPage::Cluster => {
            let tach_w = (c.w as f32 * 0.55) as i32;
            round_gauge(
                page.surface,
                Rect::new(c.x, c.y, tach_w, c.h),
                RoundGaugeOpts {
                    value: obd.rpm,
                    redline: Some(0.78),
                    label: "RPM",
                    color: pal.primary,
                    font_px: page.font_px,
                    ..Default::default()
                },
            );
            let rx = c.x + tach_w + 8;
            let rw = c.w - tach_w - 12;
            numeric_readout(
                page.surface,
                rx as f32 + rw as f32 * 0.5,
                c.y as f32 + page.font_px,
                &format!("SPD {:.0}", obd.speed * 160.0),
                pal.readout,
                page.font_px * 1.1,
            );
            let tape_h = (c.h - (page.font_px as i32) * 3) / 2;
            tape_gauge(
                page.surface,
                Rect::new(rx, c.y + (page.font_px as i32) * 2, rw / 2 - 4, tape_h),
                TapeOpts {
                    orientation: TapeOrientation::Vertical,
                    font_px: page.font_px * 0.75,
                    color: pal.caution,
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
                    font_px: page.font_px * 0.75,
                    color: pal.nav,
                    value: obd.coolant,
                    label: "COOL",
                },
            );
        }
        AutoPage::Power => {
            let tw = c.w / 3 - 6;
            for (i, (lab, val, col)) in [
                ("LOAD", obd.load, pal.primary),
                ("TPS", obd.throttle, pal.nav),
                ("BATT", obd.battery, pal.readout),
            ]
            .iter()
            .enumerate()
            {
                tape_gauge(
                    page.surface,
                    Rect::new(c.x + i as i32 * (tw + 6), c.y, tw, c.h),
                    TapeOpts {
                        orientation: TapeOrientation::Vertical,
                        font_px: page.font_px * 0.8,
                        color: *col,
                        value: *val,
                        label: lab,
                    },
                );
            }
        }
        AutoPage::Temps => {
            let bar_h = (c.h as f32 * 0.28) as i32;
            tape_gauge(
                page.surface,
                Rect::new(c.x, c.y, c.w, bar_h),
                TapeOpts {
                    orientation: TapeOrientation::Horizontal,
                    font_px: page.font_px * 0.85,
                    color: pal.warning,
                    value: (obd.coolant + obd.trans_temp) * 0.5,
                    label: "AVG TEMP",
                },
            );
            let ty = c.y + bar_h + 6;
            let th = c.h - bar_h - 6;
            let tw = c.w / 2 - 8;
            tape_gauge(
                page.surface,
                Rect::new(c.x + 4, ty, tw, th),
                TapeOpts {
                    orientation: TapeOrientation::Vertical,
                    font_px: page.font_px,
                    color: pal.nav,
                    value: obd.coolant,
                    label: "COOLANT",
                },
            );
            tape_gauge(
                page.surface,
                Rect::new(c.x + tw + 12, ty, tw, th),
                TapeOpts {
                    orientation: TapeOrientation::Vertical,
                    font_px: page.font_px,
                    color: pal.caution,
                    value: obd.trans_temp,
                    label: "TRANS",
                },
            );
        }
        AutoPage::Obd => {
            let lines = [
                format!("PID 0C RPM  {:.0}", obd.rpm * 7000.0),
                format!("PID 0D VSS  {:.0}", obd.speed * 160.0),
                format!("PID 2F FUEL {:.0}%", obd.fuel * 100.0),
                format!("PID 05 ECT  {:.0}%", obd.coolant * 100.0),
                format!("PID 11 TPS  {:.0}%", obd.throttle * 100.0),
                format!("DTC COUNT   {}", obd.dtc_count),
            ];
            let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            list_menu(
                page.surface,
                c,
                &refs,
                None,
                page.font_px,
                pal.primary,
                pal.readout,
            );
        }
        AutoPage::Setup => {
            softkey_row(
                page.surface,
                Rect::new(c.x, c.y, c.w, (page.font_px as i32) + 8),
                &["DISP", "UNIT", "PORT", "BZL", "ABOUT"],
                SoftkeyLayout {
                    font_px: page.font_px * 0.75,
                    selected: Some(0),
                },
            );
            list_menu(
                page.surface,
                Rect::new(
                    c.x,
                    c.y + (page.font_px as i32) + 12,
                    c.w,
                    c.h / 2 - (page.font_px as i32),
                ),
                &[
                    "COLOR  MFD",
                    "UNITS  METRIC",
                    "OBD  PORT",
                    "BEZEL  TEST",
                    "ABOUT  MFD",
                ],
                Some(0),
                page.font_px,
                pal.primary,
                pal.readout,
            );
            let bot = Rect::new(c.x + 12, c.y + c.h / 2 + 8, c.w - 24, c.h / 2 - 20);
            caution_box(
                page.surface,
                Rect::new(bot.x, bot.y, bot.w, bot.h / 2 - 4),
                "CFG OK",
                page.font_px,
                pal.primary,
            );
            progress_strip(
                page.surface,
                Rect::new(bot.x, bot.bottom() - 16, bot.w, 12),
                bezel.brightness.clamp(0.05, 1.0),
                pal.nav,
                pal.structure,
            );
            numeric_readout(
                page.surface,
                bot.center().0 as f32,
                bot.bottom() as f32 - 28.0,
                "BRT BAR",
                pal.dim,
                page.font_px * 0.75,
            );
        }
    }
}

pub fn rpm_norm(rpm: f32, redline: f32) -> f32 {
    if redline <= 0.0 {
        0.0
    } else {
        (rpm / redline).clamp(0.0, 1.0)
    }
}
