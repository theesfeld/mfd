//! Composable MFD **widgets**.

mod alert;
mod attitude;
mod bezel;
mod extra;
mod label;
mod range_sensor;
mod round_gauge;
mod softkeys;
mod status;
mod tape;
mod topo;

pub use alert::{flash_label, flash_label_centered, master_warn_strip, status_cell_flash};
pub use attitude::{attitude_ball, heading_cardinal, heading_display, heading_rose};
pub use bezel::bezel_frame;
pub use extra::{
    bearing_pointer, bscope_grid, caution_box, content_after_osb, crosshair, horizon_cue,
    list_menu, numeric_readout, osb_chrome, osb_chrome_ex, progress_strip, range_rings,
    station_grid, track_gate, video_frame,
};
pub use label::{label, label_centered};
pub use range_sensor::{range_display, RangeSnapshot};
pub use round_gauge::{round_gauge, RoundGaugeOpts};
pub use softkeys::{softkey_row, SoftkeyLayout};
pub use status::{
    status_grid, status_grid_flash, tire_grid, value_readout, StatusItem, TireReading,
};
pub use tape::{tape_gauge, TapeOpts, TapeOrientation};
pub use topo::schematic_topo_map;
