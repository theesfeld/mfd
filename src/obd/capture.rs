//! Capture format: frames (TX/RX hex) + optional decoded signals.
//!
//! Compatible with the truck dump under `docs/odbii-session/` (obdtui origin;
//! we only **read** that layout — writer is new).
//!
//! **IO discipline:** buffered writers; flush every `FLUSH_EVERY` frames (not
//! every few polls) so long drives do not bog the host.

use crate::obd::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CAPTURE_FORMAT_VERSION: u32 = 1;
/// Flush capture files every N frames (balance durability vs disk thrash).
const FLUSH_EVERY: u64 = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub ts: String,
    pub dir: String, // "tx" | "rx"
    pub bus: String, // "hs" | "ms" | …
    pub data: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMeta {
    pub format_version: u32,
    pub software: String,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: String,
    #[serde(default)]
    pub vin: String,
    #[serde(default)]
    pub adapter_path: String,
    #[serde(default)]
    pub profile_id: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub capabilities: serde_json::Value,
}

#[derive(Debug)]
pub struct CaptureWriter {
    dir: PathBuf,
    frames: BufWriter<File>,
    signals: BufWriter<File>,
    meta: SessionMeta,
    frame_count: u64,
    signal_count: u64,
    /// When true, every TX/RX is written (discover phase / MFD_OBD_CAPTURE_FULL).
    log_all_frames: bool,
    /// Continuous poll: log at most 1 frame pair per this many poll ticks (0 = all).
    frame_sample: u32,
    /// Continuous poll: log signal at most 1/N ticks unless value changes enough (0 = all).
    signal_sample: u32,
    poll_tick: u32,
    /// Last logged signal value by name (for change-gated signal CSV).
    last_signal: std::collections::HashMap<String, f64>,
}

impl CaptureWriter {
    pub fn create(dir: impl Into<PathBuf>, software: &str, adapter: &str) -> Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let frames = BufWriter::with_capacity(64 * 1024, File::create(dir.join("frames.ndjson"))?);
        let signals = {
            let mut f = BufWriter::with_capacity(32 * 1024, File::create(dir.join("signals.csv"))?);
            writeln!(f, "ts,name,value,unit,mode,pid,bus")?;
            f
        };
        // Full wire log only when explicitly requested (huge on long drives).
        let log_all = matches!(
            std::env::var("MFD_OBD_CAPTURE_FULL").ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE") | Some("yes")
        );
        // Sample continuous frames ~1/8 polls unless full.
        let frame_sample = if log_all { 0 } else { 8 };
        // Signals: change-gated + ~1/4 sample (still dense enough for crush charts).
        let signal_sample = if log_all { 0 } else { 4 };
        let meta = SessionMeta {
            format_version: CAPTURE_FORMAT_VERSION,
            software: software.into(),
            started_at: now_rfc3339(),
            ended_at: String::new(),
            vin: String::new(),
            adapter_path: adapter.into(),
            profile_id: "mfd_native".into(),
            notes: if log_all {
                "full_frames".into()
            } else {
                "sampled_frames+change_signals".into()
            },
            capabilities: serde_json::json!({}),
        };
        Ok(Self {
            dir,
            frames,
            signals,
            meta,
            frame_count: 0,
            signal_count: 0,
            log_all_frames: log_all,
            frame_sample,
            signal_sample,
            poll_tick: 0,
            last_signal: std::collections::HashMap::new(),
        })
    }

    pub fn set_vin(&mut self, vin: &str) {
        self.meta.vin = vin.into();
    }

    pub fn set_caps(&mut self, caps: serde_json::Value) {
        self.meta.capabilities = caps;
    }

    /// Force full wire logging (discover phase).
    pub fn set_log_all_frames(&mut self, all: bool) {
        self.log_all_frames = all;
    }

    /// Call once per continuous poll iteration for sample pacing.
    pub fn tick_poll(&mut self) {
        self.poll_tick = self.poll_tick.wrapping_add(1);
    }

    fn write_frame(&mut self, dir: &str, bus: &str, data: &str, note: Option<&str>) -> Result<()> {
        let f = Frame {
            ts: now_rfc3339(),
            dir: dir.into(),
            bus: bus.into(),
            data: data.into(),
            note: note.map(|s| s.into()),
        };
        serde_json::to_writer(&mut self.frames, &f).map_err(|e| Error::Protocol(e.to_string()))?;
        writeln!(self.frames)?;
        self.frame_count += 1;
        if self.frame_count % FLUSH_EVERY == 0 {
            let _ = self.frames.flush();
            let _ = self.signals.flush();
        }
        Ok(())
    }

    /// Continuous-poll frame log (sampled unless `MFD_OBD_CAPTURE_FULL=1`).
    pub fn log_frame(
        &mut self,
        dir: &str,
        bus: &str,
        data: &str,
        note: Option<&str>,
    ) -> Result<()> {
        if !self.log_all_frames && self.frame_sample != 0 && self.poll_tick % self.frame_sample != 0
        {
            return Ok(());
        }
        self.write_frame(dir, bus, data, note)
    }

    /// Always log (discover phase / DTC / important events).
    pub fn log_frame_always(
        &mut self,
        dir: &str,
        bus: &str,
        data: &str,
        note: Option<&str>,
    ) -> Result<()> {
        self.write_frame(dir, bus, data, note)
    }

    pub fn log_signal(
        &mut self,
        name: &str,
        value: f64,
        unit: &str,
        mode: u8,
        pid: u8,
        bus: &str,
    ) -> Result<()> {
        // Skip near-duplicate samples during continuous poll (disk thrash on long drives).
        if !self.log_all_frames && self.signal_sample != 0 {
            let changed = match self.last_signal.get(name) {
                Some(&prev) => {
                    let d = (value - prev).abs();
                    // Absolute or relative change (handles near-zero gauges).
                    d > 0.05 && d > prev.abs() * 0.002
                }
                None => true,
            };
            let sample_ok = self.poll_tick % self.signal_sample == 0;
            if !changed && !sample_ok {
                return Ok(());
            }
            self.last_signal.insert(name.to_string(), value);
        }
        writeln!(
            self.signals,
            "{},{},{},{},{},{},{}",
            now_rfc3339(),
            name,
            value,
            unit,
            mode,
            pid,
            bus
        )?;
        self.signal_count += 1;
        if self.signal_count % FLUSH_EVERY == 0 {
            let _ = self.signals.flush();
        }
        Ok(())
    }

    /// Force flush open capture files to disk.
    pub fn flush(&mut self) -> Result<()> {
        self.frames.flush()?;
        self.signals.flush()?;
        Ok(())
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn finish(mut self) -> Result<PathBuf> {
        self.meta.ended_at = now_rfc3339();
        self.frames.flush()?;
        self.signals.flush()?;
        let mut t = File::create(self.dir.join("meta.toml"))?;
        writeln!(t, "format_version = {}", self.meta.format_version)?;
        writeln!(t, "software = {:?}", self.meta.software)?;
        writeln!(t, "started_at = {:?}", self.meta.started_at)?;
        writeln!(t, "ended_at = {:?}", self.meta.ended_at)?;
        writeln!(t, "vin = {:?}", self.meta.vin)?;
        writeln!(t, "adapter_path = {:?}", self.meta.adapter_path)?;
        writeln!(t, "profile_id = {:?}", self.meta.profile_id)?;
        writeln!(t, "notes = {:?}", self.meta.notes)?;
        let session = serde_json::json!({
            "meta": self.meta,
            "frames": [],
            "frame_file": "frames.ndjson",
            "frame_count": self.frame_count,
            "signal_count": self.signal_count,
        });
        let mut s = File::create(self.dir.join("session.json"))?;
        serde_json::to_writer_pretty(&mut s, &session)
            .map_err(|e| Error::Protocol(e.to_string()))?;
        Ok(self.dir)
    }
}

/// Load frames from `frames.ndjson` or legacy `session.json` with embedded frames.
pub fn load_frames(path: &Path) -> Result<Vec<Frame>> {
    if path.is_dir() {
        let nd = path.join("frames.ndjson");
        if nd.exists() {
            return load_ndjson(&nd);
        }
        let sj = path.join("session.json");
        if sj.exists() {
            return load_session_json(&sj);
        }
        return Err(Error::Adapter(format!(
            "no frames.ndjson or session.json in {}",
            path.display()
        )));
    }
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e == "ndjson" || e == "jsonl")
    {
        return load_ndjson(path);
    }
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e == "json")
    {
        return load_session_json(path);
    }
    load_ndjson(path)
}

fn load_ndjson(path: &Path) -> Result<Vec<Frame>> {
    let f = File::open(path)?;
    let mut out = Vec::new();
    for line in BufReader::new(f).lines() {
        let line = line?;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let fr: Frame =
            serde_json::from_str(t).map_err(|e| Error::Decode(format!("frame: {e}")))?;
        out.push(fr);
    }
    Ok(out)
}

fn load_session_json(path: &Path) -> Result<Vec<Frame>> {
    let text = fs::read_to_string(path)?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| Error::Decode(format!("session.json: {e}")))?;
    let frames = v
        .get("frames")
        .and_then(|f| f.as_array())
        .ok_or_else(|| Error::Decode("session.json missing frames".into()))?;
    let mut out = Vec::with_capacity(frames.len());
    for fr in frames {
        let f: Frame = serde_json::from_value(fr.clone())
            .map_err(|e| Error::Decode(format!("frame obj: {e}")))?;
        out.push(f);
    }
    Ok(out)
}

fn now_rfc3339() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:09}Z", dur.as_secs(), dur.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_truck_capture() {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/odbii-session/frames.ndjson");
        if !p.exists() {
            return;
        }
        let frames = load_frames(&p).unwrap();
        assert!(frames.len() > 100);
        assert_eq!(frames[0].dir, "tx");
        assert_eq!(frames[0].data, "010C");
    }
}
