//! MFD **color modes**.
//!
//! **ColorMfd** roles follow MLU M1 Pilot’s Guide **Table 1-1** / Figs 1-17–1-18:
//! cyan safety/bullseye cursors; white ownship/nav/text; yellow tracks/bug;
//! green default/rings; red threat/warn; black glass.
//! See `docs/reference/mlu-m1-cmfd.md`.

use crate::color::{rgb, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, MAGENTA, RED, WHITE, YELLOW};
use crate::Color;

/// Selectable display color set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Classic monochrome green (pre-color / night-simple).
    #[default]
    GreenMono,
    /// MLU color CMFD palette (Table 1-1).
    ColorMfd,
    /// High-visibility (yellow-dominant legends).
    HighVis,
}

/// Resolved ink roles for drawing.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub mode: ColorMode,
    /// Glass background (black).
    pub glass: Color,
    /// Default symbology / softkeys (green).
    pub primary: Color,
    /// Dim structure (dim green).
    pub dim: Color,
    /// Safety cursors, bullseye geometry (cyan).
    pub nav: Color,
    /// Caution / tracks when amber preferred.
    pub caution: Color,
    /// Warning / threat / redline (red).
    pub warning: Color,
    /// Special cue (magenta — rare on MLU table; kept for modes).
    pub special: Color,
    /// Ownship data, STPT, routes, primary text (white).
    pub readout: Color,
    /// Grid / non-colorized structure.
    pub structure: Color,
    /// Radar tracks / bugged target (yellow — Table 1-1 FCR).
    pub track: Color,
}

impl Palette {
    pub fn new(mode: ColorMode) -> Self {
        match mode {
            ColorMode::GreenMono => Self {
                mode,
                glass: BLACK,
                primary: GREEN,
                dim: GREEN_DIM,
                nav: GREEN,
                caution: GREEN,
                warning: GREEN,
                special: GREEN,
                readout: GREEN,
                structure: GREEN_DIM,
                track: GREEN,
            },
            // MLU M1 Table 1-1 + Figs 1-17 / 1-18.
            ColorMode::ColorMfd => Self {
                mode,
                glass: BLACK,
                primary: GREEN,
                dim: GREEN_DIM,
                nav: CYAN,
                caution: AMBER,
                warning: RED,
                special: MAGENTA,
                readout: WHITE,
                structure: GREEN_DIM,
                track: YELLOW,
            },
            ColorMode::HighVis => Self {
                mode,
                glass: BLACK,
                primary: YELLOW,
                dim: rgb(160, 140, 20),
                nav: YELLOW,
                caution: AMBER,
                warning: RED,
                special: MAGENTA,
                readout: WHITE,
                structure: rgb(100, 90, 20),
                track: YELLOW,
            },
        }
    }
}
