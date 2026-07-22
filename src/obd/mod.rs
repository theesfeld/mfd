//! **Native OBD-II / UDS stack** for the color MFD (**CMFD**).
//!
//! # Safety — display only
//!
//! This product **only displays** information. It **never** writes the vehicle
//! (no clear DTC, no DID write, no security unlock, no programming). UDS allow-list
//! is hard-coded in [`uds`]; there is **no** write override env.
//!
//! Layers:
//! - [`transport`] — serial + Linux Bluetooth RFCOMM SPP
//! - [`elm`] — ELM327/STN AT commands
//! - [`j1979`] — Mode 01 PID decode
//! - [`isotp`] — multi-frame reassembly
//! - [`uds`] — read path: `0x10` / `0x19` / `0x3E` / `0x22`
//! - [`ford`] — F-150 class DID catalog + decode (verify on truck)
//! - [`capture`] / [`replay`] — log + play back frames
//! - [`feed`] — background poll → vehicle snapshot
//!
//! # Env
//! | Variable | Meaning |
//! |----------|---------|
//! | `MFD_OBD_BT=AA:BB:…` | Bluetooth classic MAC (SPP); feed **keeps searching** until up |
//! | `MFD_OBD_BT_CHANNEL=1` | Preferred RFCOMM channel (others tried on failure) |
//! | `MFD_OBD_PORT=/dev/ttyUSB0` | Serial / rfcomm device |
//! | `MFD_OBD_BAUD=115200` | Serial baud |
//! | `MFD_OBD_REPLAY=path` | Capture dir or frames.ndjson |
//! | `MFD_OBD_CAPTURE=dir` | Live capture dir (same process as glass) |
//! | `MFD_OBD_CRUSH=1` | Discover all Mode 01 PIDs + multi-module UDS |
//!
//! Bluetooth: BlueZ assist (`bluetoothctl power/connect`), OBD-name discovery among
//! paired devices, channel scan, and reconnect after link loss. Pair the dongle once.

#![cfg(feature = "obd")]

pub mod capture;
pub mod elm;
pub mod error;
pub mod feed;
pub mod ford;
pub mod isotp;
pub mod j1979;
pub mod replay;
pub mod session;
pub mod transport;
pub mod uds;

pub use error::{Error, Result};
pub use feed::ObdFeed;
pub use ford::{decode_data, feed_poll_dids, prepare_pcm_read, probe_dids, DidDef, F150_DIDS};
pub use j1979::{
    decode_dtc_response, decode_mode01, format_dtc_bytes, merge_dtcs, Dtc, DtcKind, LiveValue,
    PRIORITY_PIDS,
};
pub use session::{ConnectOpts, Session};
// transport helpers used by capture tool / tests
pub use transport::{discover_obd_macs, normalize_mac};
