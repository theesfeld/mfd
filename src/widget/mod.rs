//! Composable MFD **widgets**.

mod bezel;
mod extra;
mod label;
mod round_gauge;
mod softkeys;
mod tape;

pub use bezel::bezel_frame;
pub use extra::{
    bearing_pointer, bscope_grid, caution_box, content_after_osb, crosshair, horizon_cue,
    list_menu, numeric_readout, osb_chrome, progress_strip, range_rings, station_grid, track_gate,
    video_frame,
};
pub use label::{label, label_centered};
pub use round_gauge::{round_gauge, RoundGaugeOpts};
pub use softkeys::{softkey_row, SoftkeyLayout};
pub use tape::{tape_gauge, TapeOpts, TapeOrientation};
