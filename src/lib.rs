//! **MFD** — multi-function display library.
//!
//! Build instrument **pages** from widgets (softkeys, tapes, round gauges,
//! labels, bezels). Aviation page calls live in [`jet`]; automotive reuse
//! and OBD-shaped inputs live in [`auto`].
//!
//! Low-level pixel strokes: pure assembly **libmfd** (`mfd_line`, `mfd_circle`, …).
//! Text: **B612 Mono** (cockpit font). Glass: black + fighter symbology colors.
//!
//! ```ignore
//! use mfd::page::Page;
//! use mfd::jet;
//! let mut s = mfd::Surface::new(800, 480);
//! let mut page = Page::new(&mut s);
//! jet::hsd(&mut page, 270.0, 40.0);
//! ```

#![allow(non_camel_case_types)]

pub mod auto;
pub mod color;
pub mod font;
pub mod frame;
pub mod geom;
pub mod jet;
pub mod page;
pub mod surface;
pub mod term;
pub mod widget;

pub use color::{
    rgb, Ink, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, GREY, MAGENTA, PANEL, RED, TRANSPARENT, WHITE,
    YELLOW,
};
pub use font::{draw_text, draw_text_centered, text_height, text_width};
pub use geom::Rect;
pub use page::Page;
pub use surface::{engine_version, using_assembly, Surface, Xform};
pub use widget::{
    bezel_frame, label, label_centered, round_gauge, softkey_row, tape_gauge, RoundGaugeOpts,
    SoftkeyLayout, TapeOpts, TapeOrientation,
};

/// Packed color `0xAARRGGBB`.
pub type Color = u32;

pub const VERSION: &str = "0.1.0-dev.1";

#[inline]
pub const fn alpha(c: Color) -> u8 {
    ((c >> 24) & 0xFF) as u8
}
