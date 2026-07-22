//! Real F-16 **CMFD power-on** glass (public MLU model) for vehicle product.
//!
//! Phases (see `docs/reference/cmfd-power-on.md` + vehicle-cmfd-design.md):
//! 1. Power apply — pure black  
//! 2. Display alive — BLANK content + format-select chrome  
//! 3. Bottom: OWN · blank · blank · blank · DCLT (slots fill after probe)
//!
//! Capability probe runs **off-glass**. No GO/NOGO splash. DTC is a format later.

use crate::auto::caps::VehicleCaps;
use crate::page::Page;
use crate::palette::Palette;
use crate::widget::{bezel_frame, osb_chrome};

/// Draw authentic CMFD power-on face while probe runs.
///
/// `t` — wall time for phase timing.  
/// `caps.progress` — 0..1 probe progress (only used to advance visual phase).
pub fn draw_bit_screen(page: &mut Page, pal: &Palette, caps: &VehicleCaps, t: f32) {
    let p = if caps.progress > 0.02 {
        caps.progress.clamp(0.0, 1.0)
    } else {
        (t / 2.8).clamp(0.0, 1.0)
    };

    page.clear();
    page.surface.clear(pal.glass);

    // ── Phase 1: power apply — black glass ───────────────────────────────
    if p < 0.12 {
        let _ = caps;
        return;
    }

    // ── Phase 2–3: bezel + vehicle format-select chrome, blank content ────
    let b = page.bounds.inset(2);
    bezel_frame(page.surface, b);

    // Top / sides empty until formats ready (unlabeled = no function).
    let top = ["", "", "", "", ""];
    let right = ["", "", "", "", ""];
    let left = ["", "", "", "", ""];
    // Bottom L→R OSB 15..11: OWN · slotA · slotB · slotC · DCLT
    // Slots blank during probe — filled after GO set is known.
    let bottom = ["OWN", "", "", "", "DCLT"];

    let active = if p >= 0.20 { Some(14u8) } else { None };
    osb_chrome(
        page.surface,
        b,
        &top,
        &right,
        &bottom,
        &left,
        page.font_px * 0.65,
        pal.dim,
        active,
    );
    // Content: true blank.
}
