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
//! - [`uds`] — read path: `0x10` / `0x3E` / `0x22` only
//! - [`capture`] / [`replay`] — log + play back frames
//! - [`feed`] — background poll → vehicle snapshot
//!
//! # Env
//! | Variable | Meaning |
//! |----------|---------|
//! | `MFD_OBD_BT=AA:BB:…` | Bluetooth classic MAC (SPP) |
//! | `MFD_OBD_BT_CHANNEL=1` | RFCOMM channel (default 1) |
//! | `MFD_OBD_PORT=/dev/ttyUSB0` | Serial / rfcomm device |
//! | `MFD_OBD_BAUD=115200` | Serial baud |
//! | `MFD_OBD_REPLAY=path` | Capture dir or frames.ndjson |

#![cfg(feature = "obd")]

pub mod capture;
pub mod elm;
pub mod error;
pub mod feed;
pub mod isotp;
pub mod j1979;
pub mod replay;
pub mod session;
pub mod transport;
pub mod uds;

pub use error::{Error, Result};
pub use feed::ObdFeed;
pub use j1979::{
    decode_dtc_response, decode_mode01, format_dtc_bytes, merge_dtcs, Dtc, DtcKind, LiveValue,
    PRIORITY_PIDS,
};
pub use session::{ConnectOpts, Session};
