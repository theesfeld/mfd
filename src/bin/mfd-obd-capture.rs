//! Capture OBD / UDS traffic from Bluetooth or serial ELM327/STN.
//!
//! ```text
//! mfd-obd-capture --bt 00:04:3E:96:B8:F1 -o ./capture
//! mfd-obd-capture --port /dev/ttyUSB0 --uds --seconds 120
//! mfd-obd-capture --replay docs/odbii-session --seconds 5 -o /tmp/replay-test
//! ```
//!
//! Deep UDS probes need a live adapter. Truck capture in-repo is Mode 01 only.

use std::env;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use mfd::obd::capture::CaptureWriter;
use mfd::obd::j1979::{self, PRIORITY_PIDS};
use mfd::obd::session::{ConnectOpts, Session};
use mfd::obd::uds::{self, PROBE_DIDS};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    let mut bt = None;
    let mut port = None;
    let mut replay = None;
    let mut out = PathBuf::from(format!(
        "obd-capture-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ));
    let mut seconds: u64 = 60;
    let mut do_uds = false;
    let mut baud = 115_200u32;
    let mut channel = 1u8;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--bt" => {
                i += 1;
                bt = Some(args.get(i).ok_or("--bt needs MAC")?.clone());
            }
            "--port" => {
                i += 1;
                port = Some(args.get(i).ok_or("--port needs path")?.clone());
            }
            "--replay" => {
                i += 1;
                replay = Some(PathBuf::from(args.get(i).ok_or("--replay needs path")?));
            }
            "-o" | "--out" => {
                i += 1;
                out = PathBuf::from(args.get(i).ok_or("-o needs dir")?);
            }
            "--seconds" => {
                i += 1;
                seconds = args
                    .get(i)
                    .ok_or("--seconds needs n")?
                    .parse()
                    .map_err(|e| format!("seconds: {e}"))?;
            }
            "--uds" => do_uds = true,
            "--baud" => {
                i += 1;
                baud = args
                    .get(i)
                    .ok_or("--baud needs n")?
                    .parse()
                    .map_err(|e| format!("baud: {e}"))?;
            }
            "--channel" => {
                i += 1;
                channel = args
                    .get(i)
                    .ok_or("--channel needs n")?
                    .parse()
                    .map_err(|e| format!("channel: {e}"))?;
            }
            other => return Err(format!("unknown arg: {other}")),
        }
        i += 1;
    }

    // Env fallbacks
    if bt.is_none() {
        bt = env::var("MFD_OBD_BT").ok().filter(|s| !s.is_empty());
    }
    if port.is_none() {
        port = env::var("MFD_OBD_PORT").ok().filter(|s| !s.is_empty());
    }
    if replay.is_none() {
        replay = env::var_os("MFD_OBD_REPLAY").map(PathBuf::from);
    }

    let adapter_label = bt
        .as_ref()
        .map(|m| format!("bt://{m}"))
        .or_else(|| port.clone())
        .or_else(|| replay.as_ref().map(|p| format!("replay:{}", p.display())))
        .unwrap_or_else(|| "unknown".into());

    eprintln!("mfd-obd-capture — native OBD/UDS logger");
    eprintln!("  adapter: {adapter_label}");
    eprintln!("  out:     {}", out.display());
    eprintln!("  seconds: {seconds}");
    eprintln!("  uds:     {do_uds}");

    let mut session = Session::connect(ConnectOpts {
        serial_path: port,
        baud,
        bt_mac: bt,
        bt_channel: channel,
        replay,
        timeout: Duration::from_millis(4_000),
    })
    .map_err(|e| e.to_string())?;

    eprintln!(
        "  identity: {} · protocol: {}",
        session.identity(),
        session.protocol()
    );

    let software = format!("mfd-obd-capture {}", env!("CARGO_PKG_VERSION"));
    let mut cap =
        CaptureWriter::create(&out, &software, &adapter_label).map_err(|e| e.to_string())?;
    cap.set_caps(serde_json::json!({
        "identity": session.identity(),
        "protocol": session.protocol(),
        "link": adapter_label,
    }));

    if let Ok(vin) = session.read_vin_mode09() {
        eprintln!("  VIN: {vin}");
        cap.set_vin(&vin);
        let _ = cap.log_frame("rx", "hs", &format!("VIN:{vin}"), Some("mode09"));
    }

    if do_uds {
        eprintln!("  UDS probe (headers 7E0, DIDs)…");
        let _ = session.extended_session();
        let _ = cap.log_frame(
            "tx",
            "hs",
            "1003",
            Some("DiagnosticSessionControl extended"),
        );
        for &(did, name) in PROBE_DIDS {
            let req = format!("22{:04X}", did);
            let _ = cap.log_frame("tx", "hs", &req, Some(name));
            match session.read_did(uds::DEFAULT_ECM_HEADER, did) {
                Ok(data) => {
                    let hex = uds::hex_bytes(&data);
                    eprintln!("    DID {did:04X} {name}: {hex}");
                    let _ = cap.log_frame("rx", "hs", &hex, Some(name));
                }
                Err(e) => {
                    eprintln!("    DID {did:04X} {name}: {e}");
                    let _ = cap.log_frame("rx", "hs", &format!("ERR:{e}"), Some(name));
                }
            }
            let _ = session.tester_present();
        }
    }

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut i = 0usize;
    eprintln!("  polling Mode 01 PIDs…");
    while Instant::now() < deadline {
        let pid = PRIORITY_PIDS[i % PRIORITY_PIDS.len()];
        i += 1;
        let cmd = j1979::mode01_command(pid);
        let _ = cap.log_frame("tx", "hs", &cmd, None);
        match session.read_pid(pid) {
            Ok(v) => {
                // Re-encode data for log: use response via raw would need elm — store decoded
                let _ = cap.log_frame("rx", "hs", &format!("41{:02X}…", pid), Some(v.name));
                let _ = cap.log_signal(v.name, v.value, v.unit, v.mode, v.pid, "hs");
                if i % 20 == 0 {
                    eprintln!("    t{}  {}={:.2} {}", i, v.name, v.value, v.unit);
                }
            }
            Err(e) => {
                let _ = cap.log_frame("rx", "hs", &format!("ERR:{e}"), None);
            }
        }
        if i % 30 == 0 {
            let _ = session.tester_present();
            let _ = cap.log_frame("tx", "hs", "3E80", Some("TesterPresent"));
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let dir = cap.finish().map_err(|e| e.to_string())?;
    eprintln!("done → {}", dir.display());
    eprintln!("  frames.ndjson  signals.csv  meta.toml  session.json");
    Ok(())
}

fn print_help() {
    eprintln!(
        "\
mfd-obd-capture — log OBD-II / UDS from ELM327/STN

Usage:
  mfd-obd-capture --bt AA:BB:CC:DD:EE:FF -o ./cap --seconds 90
  mfd-obd-capture --port /dev/rfcomm0 --uds
  mfd-obd-capture --replay docs/odbii-session --seconds 10 -o /tmp/t

Options:
  --bt MAC          Bluetooth SPP (Linux RFCOMM)
  --channel N       RFCOMM channel (default 1)
  --port PATH       Serial device
  --baud N          Baud (default 115200)
  --replay PATH     Capture dir or frames.ndjson
  -o, --out DIR     Output directory
  --seconds N       Poll duration (default 60)
  --uds             Probe extended session + common 0x22 DIDs
  -h, --help        This help

Env: MFD_OBD_BT, MFD_OBD_PORT, MFD_OBD_REPLAY, MFD_OBD_BAUD

Display-only: never writes vehicle (no security unlock / clear DTC / DID write).
"
    );
}
