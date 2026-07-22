//! F-16 CMFD-style **power-on / BIT** glass until capability probe is ready.
//!
//! Visual language matches MLU TEST format: black glass, GO/RDY/NOGO list,
//! progress strip, OFP line — not a marketing splash.

use crate::auto::caps::{BitState, VehicleCaps};
use crate::geom::Rect;
use crate::page::Page;
use crate::palette::Palette;
use crate::widget::{bezel_frame, caution_box, label, label_centered, list_menu, progress_strip};
use crate::VERSION;

/// Draw CMFD BIT / loading page (full face).
pub fn draw_bit_screen(page: &mut Page, pal: &Palette, caps: &VehicleCaps, t: f32) {
    page.clear();
    page.surface.clear(pal.glass);
    let b = page.bounds.inset(2);
    bezel_frame(page.surface, b);
    // Structure ink: outer frame already from bezel_frame

    let fh = page.font_px;
    let cx = b.center().0 as f32;

    // Title block (format-like)
    label_centered(
        page.surface,
        cx,
        b.y as f32 + fh * 0.6,
        "CMFD",
        pal.primary,
        fh * 1.1,
    );
    label_centered(
        page.surface,
        cx,
        b.y as f32 + fh * 1.7,
        &format!("OFP  MFD {}", VERSION),
        pal.dim,
        fh * 0.7,
    );
    label_centered(
        page.surface,
        cx,
        b.y as f32 + fh * 2.6,
        &caps.phase,
        pal.readout,
        fh * 0.85,
    );

    // BIT lines — same language as jet TEST format
    let mut lines: Vec<String> = caps
        .lines
        .iter()
        .map(|l| format!("{:<8} {}", l.name, l.state.label()))
        .collect();
    if lines.is_empty() {
        // Animated placeholder while first lines arrive
        let step = ((t * 2.0) as usize) % 4;
        let dots = ["", ".", "..", "..."][step];
        lines.push(format!("MFDS     RDY{dots}"));
        lines.push("LINK     RDY".into());
        lines.push("PROBE    RDY".into());
    }
    // Pad for stable layout
    while lines.len() < 8 {
        lines.push(String::new());
    }
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let list_top = b.y + (fh * 3.4) as i32;
    let list_h = b.h - (fh * 6.5) as i32;
    list_menu(
        page.surface,
        Rect::new(b.x + 12, list_top, b.w - 24, list_h.max(40)),
        &refs,
        None,
        fh * 0.85,
        pal.primary,
        pal.readout,
    );

    // Progress (LOAD / BIT)
    let prog = caps.progress.clamp(0.0, 1.0);
    let bar_y = b.bottom() - (fh * 2.8) as i32;
    progress_strip(
        page.surface,
        Rect::new(b.x + 20, bar_y, b.w - 40, 12),
        prog,
        pal.nav,
        pal.structure,
    );
    label_centered(
        page.surface,
        cx,
        bar_y as f32 + 16.0,
        &format!("BIT  {:.0}%  ·  {}", prog * 100.0, caps.link),
        pal.dim,
        fh * 0.7,
    );

    if caps.ready {
        caution_box(
            page.surface,
            Rect::new(
                b.x + 24,
                b.bottom() - (fh * 1.6) as i32 - 8,
                b.w - 48,
                (fh * 1.4) as i32,
            ),
            "BIT COMPLETE",
            fh * 0.85,
            pal.primary,
        );
    } else {
        // Blink RDY cue
        if (t * 3.0).sin() > 0.0 {
            label(
                page.surface,
                b.x as f32 + 16.0,
                b.bottom() as f32 - fh * 1.2,
                "INIT",
                pal.caution,
                fh * 0.75,
            );
        }
    }

    // Legend: map BitState colors via trailing labels already GO/RDY/NOGO
    let _ = BitState::Go;
}
