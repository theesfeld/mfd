//! Composable MFD **widgets** (call one or many per page).
//!
//! Lessons applied from open glass/MFD research:
//! - black glass + high-contrast ink
//! - chrome (softkeys) separate from content
//! - tape / round gauges as first-class widgets
//! - labels via cockpit font (B612)

mod bezel;
mod label;
mod round_gauge;
mod softkeys;
mod tape;

pub use bezel::bezel_frame;
pub use label::{label, label_centered};
pub use round_gauge::{round_gauge, RoundGaugeOpts};
pub use softkeys::{softkey_row, SoftkeyLayout};
pub use tape::{tape_gauge, TapeOpts, TapeOrientation};
