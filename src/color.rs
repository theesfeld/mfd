//! Fighter-glass **symbology colors** (high contrast on black).
//!
//! Typical MFD/HUD palette (not airframe paint):
//! - **Green** — primary mode / normal status / softkeys
//! - **Cyan** — geometry, nav, secondary symbology
//! - **White** — primary readout / highlight
//! - **Amber / yellow** — caution
//! - **Red** — warning / limit / threat
//! - **Magenta** — special cue (some jets)
//!
//! These are library tokens for consistent pages — not classified OEM ROM.

use crate::Color;

/// Pack opaque RGB.
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Fully transparent (unused on black-glass pages).
pub const TRANSPARENT: Color = 0;
/// Instrument glass background.
pub const BLACK: Color = rgb(0, 0, 0);
/// Near-black panel (optional depth).
pub const PANEL: Color = rgb(4, 8, 6);

/// Primary green (softkeys, normal).
pub const GREEN: Color = rgb(0, 255, 70);
/// Dim green (ticks, secondary structure).
pub const GREEN_DIM: Color = rgb(0, 120, 45);
/// Cyan geometry / nav.
pub const CYAN: Color = rgb(40, 220, 255);
/// Amber caution.
pub const AMBER: Color = rgb(255, 200, 40);
/// Yellow cue (some tapes / bugs).
pub const YELLOW: Color = rgb(255, 240, 60);
/// Red warning / redline.
pub const RED: Color = rgb(255, 48, 48);
/// Magenta special.
pub const MAGENTA: Color = rgb(255, 60, 220);
/// White primary text / needle.
pub const WHITE: Color = rgb(235, 255, 240);
/// Cool grey structure.
pub const GREY: Color = rgb(90, 110, 100);

/// Named role for widgets (maps to palette).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Primary,
    Dim,
    Nav,
    Caution,
    Warning,
    Special,
    Readout,
    Structure,
}

impl Ink {
    pub fn color(self) -> Color {
        match self {
            Ink::Primary => GREEN,
            Ink::Dim => GREEN_DIM,
            Ink::Nav => CYAN,
            Ink::Caution => AMBER,
            Ink::Warning => RED,
            Ink::Special => MAGENTA,
            Ink::Readout => WHITE,
            Ink::Structure => GREY,
        }
    }
}
