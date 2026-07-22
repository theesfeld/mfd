//! Real F-16 **CMFD power-on** glass (public MLU / training model).
//!
//! Phases (see `docs/reference/cmfd-power-on.md`):
//! 1. Power apply — pure black  
//! 2. Display alive — BLANK face (empty content) + MLU OSB chrome  
//! 3. Format select labels — SWAP · FCR · HSD · SMS · DCLT (training default)
//!
//! Capability probe runs in the background. This is **not** a invented BIT
//! checklist splash. TEST/BIT format is a separate selectable page, not cold power.

use crate::auto::caps::VehicleCaps;
use crate::jet::FormatSelect;
use crate::page::Page;
use crate::palette::Palette;
use crate::widget::{bezel_frame, osb_chrome};

/// Training-default format select (left CMFD NAV/A-A style).
fn power_on_format_select() -> FormatSelect {
    FormatSelect::default() // FCR / HSD / SMS on OSB 14/13/12
}

/// Draw authentic CMFD power-on face while probe runs.
///
/// `t` — wall time for phase timing.  
/// `caps.progress` — 0..1 probe progress (only used to advance visual phase).
pub fn draw_bit_screen(page: &mut Page, pal: &Palette, caps: &VehicleCaps, t: f32) {
    // Prefer probe progress; if zero, use wall clock so demo still animates.
    let p = if caps.progress > 0.02 {
        caps.progress.clamp(0.0, 1.0)
    } else {
        (t / 2.8).clamp(0.0, 1.0)
    };

    page.clear();
    page.surface.clear(pal.glass);

    // ── Phase 1: power apply — black glass, no legends ───────────────────
    if p < 0.12 {
        // Pure black face (real LCD just powered).
        let _ = caps;
        return;
    }

    // ── Phase 2–3: bezel + OSB chrome, empty content (BLANK format) ───────
    let b = page.bounds.inset(2);
    bezel_frame(page.surface, b);

    let sel = power_on_format_select();
    // Bottom L→R = OSB 15..11: SWAP · FCR · HSD · SMS · DCLT
    // Top / sides empty on blank power-on face (no page softkeys yet).
    let top = ["", "", "", "", ""];
    let right = ["", "", "", "", ""];
    let left = ["", "", "", "", ""];
    let [a, b_lab, c] = sel.slot_labels();
    let bottom = ["SWAP", a, b_lab, c, "DCLT"];

    // Highlight active format slot (OSB 14) once "alive"
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

    // Content: true blank — no center text, no progress bar, no OFP splash.
    // Real BLANK format has empty glass; format names live on OSB 12/13/14.
}
