//! Capture format: frames (TX/RX hex) + optional decoded signals.
//!
//! Compatible with the truck dump under `docs/odbii-session/` (obdtui origin;
//! we only **read** that layout — writer is new).

use crate::obd::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CAPTURE_FORMAT_VERSION: u32 = 1;

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
    frames: File,
    signals: File,
    meta: SessionMeta,
    frame_count: u64,
}

impl CaptureWriter {
    pub fn create(dir: impl Into<PathBuf>, software: &str, adapter: &str) -> Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let frames = File::create(dir.join("frames.ndjson"))?;
        let signals = {
            let mut f = File::create(dir.join("signals.csv"))?;
            writeln!(f, "ts,name,value,unit,mode,pid,bus")?;
            f
        };
        let meta = SessionMeta {
            format_version: CAPTURE_FORMAT_VERSION,
            software: software.into(),
            started_at: now_rfc3339(),
            ended_at: String::new(),
            vin: String::new(),
            adapter_path: adapter.into(),
            profile_id: "mfd_native".into(),
            notes: String::new(),
            capabilities: serde_json::json!({}),
        };
        Ok(Self {
            dir,
            frames,
            signals,
            meta,
            frame_count: 0,
        })
    }

    pub fn set_vin(&mut self, vin: &str) {
        self.meta.vin = vin.into();
    }

    pub fn set_caps(&mut self, caps: serde_json::Value) {
        self.meta.capabilities = caps;
    }

    pub fn log_frame(
        &mut self,
        dir: &str,
        bus: &str,
        data: &str,
        note: Option<&str>,
    ) -> Result<()> {
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
        // Keep capture durable on kill / power loss (drive logs).
        if self.frame_count % 8 == 0 {
            let _ = self.frames.flush();
            let _ = self.signals.flush();
        }
        Ok(())
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
        // meta.toml (simple)
        let mut t = File::create(self.dir.join("meta.toml"))?;
        writeln!(t, "format_version = {}", self.meta.format_version)?;
        writeln!(t, "software = {:?}", self.meta.software)?;
        writeln!(t, "started_at = {:?}", self.meta.started_at)?;
        writeln!(t, "ended_at = {:?}", self.meta.ended_at)?;
        writeln!(t, "vin = {:?}", self.meta.vin)?;
        writeln!(t, "adapter_path = {:?}", self.meta.adapter_path)?;
        writeln!(t, "profile_id = {:?}", self.meta.profile_id)?;
        writeln!(t, "notes = {:?}", self.meta.notes)?;
        // session.json with empty frames array (frames live in ndjson for size)
        let session = serde_json::json!({
            "meta": self.meta,
            "frames": [],
            "frame_file": "frames.ndjson",
            "frame_count": self.frame_count,
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
    // try as ndjson anyway
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
    // Simple UTC timestamp (good enough for capture ordering)
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
