//! **MFD** — multi-function display library.
//!
//! Build instrument **pages** from widgets (softkeys, tapes, round gauges,
//! labels, bezels). Aviation page calls live in [`jet`]; automotive reuse
//! and OBD-shaped inputs live in [`auto`].
//!
//! Low-level pixel strokes: pure assembly **libmfd** (`mfd_line`, `mfd_circle`, …).
//! Text: baked **B612 Mono** bitmap atlas (no runtime TTF). Glass: black + fighter ink.
//!
//! ```ignore
//! use mfd::page::Page;
//! use mfd::jet;
//! let mut s = mfd::Surface::new(800, 480);
//! let mut page = Page::new(&mut s);
//! jet::hsd(&mut page, 270.0, 40.0);
//! ```

#![allow(non_camel_case_types)]

pub mod audio;
pub mod auto;
pub mod bezel;
pub mod color;
pub mod font;
pub mod frame;
pub mod geom;
pub mod jet;
#[cfg(feature = "obd")]
pub mod obd;
pub mod page;
pub mod palette;
pub mod surface;
pub mod term;
pub mod video;
pub mod warn;
pub mod widget;

pub use bezel::{
    osb_role, BezelEvent, BezelKnob, BezelSource, BezelState, KeyboardBezel, NullBezel, OsbId,
};
pub use color::{
    rgb, Ink, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, GREY, MAGENTA, PANEL, RED, TRANSPARENT, WHITE,
    YELLOW,
};
pub use font::{
    draw_text, draw_text_centered, draw_text_size, draw_text_size_centered, text_height,
    text_height_size, text_width, text_width_size, FontSize,
};
pub use geom::Rect;
pub use page::Page;
pub use palette::{ColorMode, Palette};
pub use surface::{engine_version, using_assembly, Surface, Xform};
// term re-exports used by demos/integrators
pub use term::{
    cell_pixel_size_device, display_ppi, display_ppi_info, mfd_face_inches, physical_mfd_layout,
    pixel_space, PhysicalFace, PixelSpace, PpiSource, PxSpaceSource,
};
#[cfg(target_os = "linux")]
pub use video::V4l2Source;
pub use video::{blit_grey_flir, GreyFrame};
pub use warn::{
    evaluate as evaluate_warns, flash_on, flash_warn_on, owning_format, slot_flash_owner,
    ActiveWarn, WarnId, WarnLevel, WarningEngine, BINGO_FUEL,
};
pub use widget::{
    attitude_ball, bearing_pointer, bezel_frame, bscope_grid, caution_box, content_after_osb,
    crosshair, flash_label, flash_label_centered, heading_cardinal, heading_display, heading_rose,
    horizon_cue, label, label_centered, list_menu, master_warn_strip, numeric_readout, osb_chrome,
    osb_chrome_ex, progress_strip, range_display, range_rings, round_gauge, schematic_topo_map,
    softkey_row, station_grid, status_cell_flash, status_grid, status_grid_flash, tape_gauge,
    tire_grid, track_gate, value_readout, video_frame, RangeSnapshot, RoundGaugeOpts,
    SoftkeyLayout, StatusItem, TapeOpts, TapeOrientation, TireReading,
};

/// Packed color `0xAARRGGBB`.
pub type Color = u32;

pub const VERSION: &str = "0.1.0-dev.1";

#[inline]
pub const fn alpha(c: Color) -> u8 {
    ((c >> 24) & 0xFF) as u8
}
