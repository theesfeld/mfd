//! Capture OBD / UDS traffic from Bluetooth or serial ELM327/STN.
//!
//! ```text
//! mfd-obd-capture --bt 00:04:3E:96:B8:F1 -o ./capture --crush --seconds 7200
//! mfd-obd-capture --port /dev/ttyUSB0 --uds
//! mfd-obd-capture --replay docs/odbii-session --seconds 5 -o /tmp/replay-test
//! ```
//!
//! **Display-only** — never clear DTCs, write DIDs, or security unlock.
//! `--crush` = discover every Mode 01 PID + multi-module DID ranges + continuous poll.

use std::env;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use mfd::obd::capture::CaptureWriter;
use mfd::obd::ford::{self, DecodedDid, HDR_ABS, HDR_BCM, HDR_IPC, HDR_PCM, HDR_PSCM};
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
    let mut crush = false;
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
            "--crush" | "--full" | "--everything" => {
                crush = true;
                do_uds = true;
            }
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
    eprintln!("  crush:   {crush}");

    let opts = ConnectOpts {
        serial_path: port,
        baud,
        bt_mac: bt,
        bt_channel: channel,
        replay,
        timeout: Duration::from_millis(if crush { 2_500 } else { 4_000 }),
    };
    eprintln!("  link:     resilient search until OBD answers…");
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut session = Session::connect_resilient(&opts, &stop, |msg| {
        eprintln!("  … {msg}");
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
        "crush": crush,
    }));

    if let Ok(vin) = session.read_vin_mode09() {
        eprintln!("  VIN: {vin}");
        cap.set_vin(&vin);
        let _ = cap.log_frame("rx", "hs", &format!("VIN:{vin}"), Some("mode09"));
    }

    // Mode 09 extras (read-only)
    for cmd in ["090A", "0904", "0906"] {
        let _ = cap.log_frame("tx", "hs", cmd, Some("mode09"));
        match session.request_raw(cmd) {
            Ok(b) => {
                let _ = cap.log_frame("rx", "hs", &uds::hex_bytes(&b), Some("mode09"));
            }
            Err(e) => {
                let _ = cap.log_frame("rx", "hs", &format!("ERR:{e}"), Some("mode09"));
            }
        }
    }

    // DTCs
    eprintln!("  DTCs (03/07/0A)…");
    if let Ok(dtcs) = session.read_all_dtcs() {
        for d in &dtcs {
            let _ = cap.log_frame(
                "rx",
                "hs",
                &d.code,
                Some(&format!("dtc {}", d.kind.label())),
            );
            eprintln!("    {} {}", d.code, d.kind.label());
        }
        if dtcs.is_empty() {
            eprintln!("    (none)");
        }
    }

    // Discover Mode 01 PIDs
    eprintln!("  discovering Mode 01 PIDs…");
    let mut poll_pids: Vec<u8> = session
        .discover_mode01_pids()
        .unwrap_or_else(|_| PRIORITY_PIDS.to_vec());
    for &p in PRIORITY_PIDS {
        if !poll_pids.contains(&p) {
            poll_pids.push(p);
        }
    }
    eprintln!("    {} Mode 01 PIDs to poll", poll_pids.len());
    let _ = cap.log_frame(
        "rx",
        "hs",
        &format!(
            "PIDS:{}",
            poll_pids
                .iter()
                .map(|p| format!("{p:02X}"))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Some("discover"),
    );

    // Live DID list for continuous poll
    let mut live_dids: Vec<(String, u16, &'static str)> = Vec::new(); // header, did, name

    if do_uds || crush {
        eprintln!("  UDS modules + DIDs…");
        let modules: &[(&str, &str)] = if crush {
            &[
                ("PCM", HDR_PCM),
                ("BCM", HDR_BCM),
                ("ABS", HDR_ABS),
                ("IPC", HDR_IPC),
                ("PSCM", HDR_PSCM),
                ("ECM2", "7E1"),
                ("TCM", "7E2"),
            ]
        } else {
            &[("PCM", HDR_PCM)]
        };

        for &(mod_name, hdr) in modules {
            eprintln!("    module {mod_name} header {hdr}");
            let _ = session.elm_mut().set_header(hdr);
            let _ = session.extended_session();
            let _ = cap.log_frame("tx", "hs", &format!("ATSH{hdr}"), Some(mod_name));
            let _ = cap.log_frame("tx", "hs", "1003", Some("session"));
            let _ = session.tester_present();

            // Generic ISO DIDs
            for &(did, name) in PROBE_DIDS {
                probe_did(&mut session, &mut cap, hdr, did, name, &mut live_dids);
            }

            // Ford catalog
            if hdr == HDR_PCM {
                for def in ford::probe_dids() {
                    let _ = session.elm_mut().set_header(def.header);
                    match ford::read_did(&mut session, def) {
                        Ok(DecodedDid::Number { name, value, unit }) => {
                            eprintln!("      {name}={value:.2}{unit}");
                            let _ = cap.log_frame(
                                "rx",
                                "hs",
                                &format!("{value}"),
                                Some(&format!("{} {}", name, unit)),
                            );
                            let _ = cap.log_signal(
                                name,
                                value,
                                unit,
                                0x22,
                                (def.did & 0xFF) as u8,
                                "hs",
                            );
                            live_dids.push((def.header.into(), def.did, def.name));
                        }
                        Ok(DecodedDid::Text { name, value }) => {
                            eprintln!("      {name}={value}");
                            let _ = cap.log_frame("rx", "hs", &value, Some(name));
                            live_dids.push((def.header.into(), def.did, def.name));
                        }
                        Ok(DecodedDid::Hex { name, value }) => {
                            let _ = cap.log_frame("rx", "hs", &value, Some(name));
                            live_dids.push((def.header.into(), def.did, def.name));
                        }
                        Err(e) => {
                            let _ = cap.log_frame("rx", "hs", &format!("ERR:{e}"), Some(def.name));
                        }
                    }
                }
            }

            // Crush: range scan (log every positive 0x62)
            if crush {
                scan_did_range(&mut session, &mut cap, hdr, 0xF400, 0xF4FF, &mut live_dids)?;
                if hdr == HDR_PCM {
                    scan_did_range(&mut session, &mut cap, hdr, 0x1E00, 0x1EFF, &mut live_dids)?;
                }
                if matches!(hdr, HDR_ABS | HDR_BCM) {
                    scan_did_range(&mut session, &mut cap, hdr, 0x2B00, 0x2B7F, &mut live_dids)?;
                }
            }
        }

        // Dedupe live_dids
        live_dids.sort_by(|a, b| (a.0.as_str(), a.1).cmp(&(b.0.as_str(), b.1)));
        live_dids.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        eprintln!(
            "  live DIDs for drive poll: {} · Mode 01 PIDs: {}",
            live_dids.len(),
            poll_pids.len()
        );
        let _ = cap.log_frame(
            "rx",
            "hs",
            &format!("LIVE_DIDS:{}", live_dids.len()),
            Some("discover"),
        );
    }

    // Continuous drive poll
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut i = 0usize;
    let mut di = 0usize;
    let mut ticks = 0u64;
    eprintln!("  continuous poll until deadline (drive)…");
    while Instant::now() < deadline {
        // High rate: Mode 01
        let pid = poll_pids[i % poll_pids.len()];
        i = i.wrapping_add(1);
        let cmd = j1979::mode01_command(pid);
        let _ = cap.log_frame("tx", "hs", &cmd, None);
        match session.read_pid(pid) {
            Ok(v) => {
                let _ = cap.log_frame("rx", "hs", &format!("OK:{:02X}", pid), Some(v.name));
                let _ = cap.log_signal(v.name, v.value, v.unit, v.mode, v.pid, "hs");
            }
            Err(e) => {
                let _ = cap.log_frame("rx", "hs", &format!("ERR:{e}"), None);
            }
        }

        // Lower rate: rotate live DIDs
        if crush && !live_dids.is_empty() && ticks % 3 == 0 {
            let (ref hdr, did, name) = live_dids[di % live_dids.len()];
            di = di.wrapping_add(1);
            let _ = session.elm_mut().set_header(hdr);
            let req = format!("22{did:04X}");
            let _ = cap.log_frame("tx", "hs", &req, Some(name));
            match session.read_did(hdr, did) {
                Ok(data) => {
                    let hex = uds::hex_bytes(&data);
                    let _ = cap.log_frame("rx", "hs", &hex, Some(name));
                    if let Some(b0) = data.first() {
                        let _ =
                            cap.log_signal(name, *b0 as f64, "raw", 0x22, (did & 0xFF) as u8, "hs");
                    }
                }
                Err(e) => {
                    let _ = cap.log_frame("rx", "hs", &format!("ERR:{e}"), Some(name));
                }
            }
        }

        if ticks % 40 == 0 {
            let _ = session.tester_present();
            let _ = cap.log_frame("tx", "hs", "3E80", Some("TesterPresent"));
        }
        if ticks % 200 == 0 {
            // Refresh DTCs occasionally during drive
            if let Ok(dtcs) = session.read_all_dtcs() {
                for d in dtcs {
                    let _ = cap.log_frame("rx", "hs", &d.code, Some("dtc"));
                }
            }
        }
        if ticks % 50 == 0 {
            let left = deadline.saturating_duration_since(Instant::now()).as_secs();
            eprintln!(
                "    t{ticks}  remaining {left}s  pids={} dids={}",
                poll_pids.len(),
                live_dids.len()
            );
        }
        ticks = ticks.wrapping_add(1);
        std::thread::sleep(Duration::from_millis(if crush { 15 } else { 25 }));
    }

    let dir = cap.finish().map_err(|e| e.to_string())?;
    eprintln!("done → {}", dir.display());
    eprintln!("  frames.ndjson  signals.csv  meta.toml  session.json");
    Ok(())
}

fn probe_did(
    session: &mut Session,
    cap: &mut CaptureWriter,
    hdr: &str,
    did: u16,
    name: &str,
    live: &mut Vec<(String, u16, &'static str)>,
) {
    let req = format!("22{did:04X}");
    let _ = cap.log_frame("tx", "hs", &req, Some(name));
    match session.read_did(hdr, did) {
        Ok(data) => {
            let hex = uds::hex_bytes(&data);
            eprintln!("      {name}: {hex}");
            let _ = cap.log_frame("rx", "hs", &hex, Some(name));
            // leak name as static for live list — use fixed ISO names only
            let static_name: &'static str = match did {
                0xF190 => "vin",
                0xF191 => "ecu_hw",
                0xF18C => "ecu_serial",
                0xF187 => "spare_part",
                0xF189 => "hw_ver",
                0xF1A0 => "approval",
                _ => "did",
            };
            live.push((hdr.into(), did, static_name));
        }
        Err(e) => {
            let _ = cap.log_frame("rx", "hs", &format!("ERR:{e}"), Some(name));
        }
    }
    let _ = session.tester_present();
}

fn scan_did_range(
    session: &mut Session,
    cap: &mut CaptureWriter,
    hdr: &str,
    start: u16,
    end: u16,
    live: &mut Vec<(String, u16, &'static str)>,
) -> Result<(), String> {
    eprintln!("      scan {hdr} DIDs {start:04X}–{end:04X}…");
    let _ = session.elm_mut().set_header(hdr);
    let _ = session.extended_session();
    let mut hits = 0u32;
    for did in start..=end {
        let req = format!("22{did:04X}");
        let _ = cap.log_frame("tx", "hs", &req, Some("scan"));
        match session.read_did(hdr, did) {
            Ok(data) if !data.is_empty() => {
                // Skip pure NRC patterns if any slipped through
                if data.first() == Some(&0x7F) {
                    let _ = cap.log_frame("rx", "hs", &uds::hex_bytes(&data), Some("nrc"));
                    continue;
                }
                hits += 1;
                let hex = uds::hex_bytes(&data);
                let _ = cap.log_frame("rx", "hs", &hex, Some(&format!("HIT {did:04X}")));
                live.push((hdr.into(), did, "scan_hit"));
                if hits <= 20 || hits % 10 == 0 {
                    eprintln!("        HIT {did:04X} → {hex}");
                }
            }
            Ok(_) => {}
            Err(_) => {
                // Don't log every miss (huge); only occasional progress
            }
        }
        if did % 32 == 0 {
            let _ = session.tester_present();
        }
    }
    eprintln!("      scan {hdr} {start:04X}–{end:04X}: {hits} hits");
    let _ = cap.log_frame(
        "rx",
        "hs",
        &format!("SCAN_{hdr}_{start:04X}_{end:04X}_hits={hits}"),
        Some("scan_summary"),
    );
    Ok(())
}

fn print_help() {
    eprintln!(
        "\
mfd-obd-capture — log OBD-II / UDS from ELM327/STN (read-only)

Usage:
  mfd-obd-capture --bt AA:BB:CC:DD:EE:FF -o ./cap --crush --seconds 7200
  mfd-obd-capture --port /dev/rfcomm0 --uds
  mfd-obd-capture --replay docs/odbii-session --seconds 10 -o /tmp/t

Options:
  --bt MAC          Bluetooth SPP (Linux RFCOMM)
  --channel N       RFCOMM channel (default 1)
  --port PATH       Serial device
  --baud N          Baud (default 115200)
  --replay PATH     Capture dir or frames.ndjson
  -o, --out DIR     Output directory
  --seconds N       Poll duration (default 60; drive: 3600–10800)
  --uds             Probe extended session + common 0x22 DIDs
  --crush           EVERYTHING: PID discover + multi-module DID range scan
                    + continuous Mode 01 + live DID poll (implies --uds)
  -h, --help        This help

Env: MFD_OBD_BT, MFD_OBD_PORT, MFD_OBD_REPLAY, MFD_OBD_BAUD

Display-only: never writes vehicle (no security unlock / clear DTC / DID write).
"
    );
}
