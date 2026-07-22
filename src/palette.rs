//! MFD **color modes** (mono green, color LCD, high-vis).

use crate::color::{rgb, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, GREY, MAGENTA, RED, WHITE, YELLOW};
use crate::Color;

/// Selectable display color set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Classic monochrome green CRT-style.
    #[default]
    GreenMono,
    /// Color MFD (cyan geometry, amber caution, red warn, white readout).
    ColorMfd,
    /// High-visibility (yellow-dominant legends).
    HighVis,
}

/// Resolved ink roles for drawing.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub mode: ColorMode,
    pub glass: Color,
    pub primary: Color,
    pub dim: Color,
    pub nav: Color,
    pub caution: Color,
    pub warning: Color,
    pub special: Color,
    pub readout: Color,
    pub structure: Color,
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
            },
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
                structure: GREY,
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
            },
        }
    }
}
