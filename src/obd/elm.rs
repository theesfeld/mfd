//! ELM327 / STN **AT** command layer over a [`Transport`](super::transport::Transport).

use crate::obd::error::{Error, Result};
use crate::obd::transport::Transport;
use std::time::Duration;

/// Live ELM/STN session (text protocol).
pub struct Elm {
    transport: Box<dyn Transport>,
    timeout: Duration,
    identity: String,
    protocol: String,
}

impl Elm {
    pub fn new(transport: Box<dyn Transport>, timeout: Duration) -> Self {
        Self {
            transport,
            timeout,
            identity: String::new(),
            protocol: String::new(),
        }
    }

    pub fn name(&self) -> &str {
        self.transport.name()
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    /// Open link and run standard ELM init (reset, echo off, spaces off, auto protocol).
    pub fn open_and_init(&mut self) -> Result<()> {
        self.transport.open()?;
        // Warm drain
        let _ = self.transport.read_until_prompt(Duration::from_millis(200));
        self.cmd("ATZ")?;
        std::thread::sleep(Duration::from_millis(500));
        let id = self.cmd("ATI")?;
        self.identity = clean_response(&id);
        self.cmd("ATE0")?;
        self.cmd("ATL0")?;
        self.cmd("ATS0")?;
        self.cmd("ATH0")?;
        // Longer headers off; adaptive timing
        let _ = self.cmd("ATAT1");
        let _ = self.cmd("ATSP0"); // auto
        let proto = self.cmd("ATDP")?;
        self.protocol = clean_response(&proto);
        Ok(())
    }

    /// Send AT or OBD command (without CR); returns raw text before `>`.
    pub fn cmd(&mut self, command: &str) -> Result<String> {
        let mut line = command.trim().to_string();
        if !line.ends_with('\r') {
            line.push('\r');
        }
        self.transport.write_str(&line)?;
        let raw = self.transport.read_until_prompt(self.timeout)?;
        if raw.to_ascii_uppercase().contains("UNABLE TO CONNECT") {
            return Err(Error::Protocol("UNABLE TO CONNECT".into()));
        }
        if raw.to_ascii_uppercase().contains("BUS INIT")
            && raw.to_ascii_uppercase().contains("ERROR")
        {
            return Err(Error::Protocol("BUS INIT ERROR".into()));
        }
        Ok(raw)
    }

    /// Request Mode/PID hex payload (e.g. `010C`) and return response hex bytes.
    pub fn request_hex(&mut self, req: &str) -> Result<Vec<u8>> {
        let raw = self.cmd(req)?;
        if raw.to_ascii_uppercase().contains("NO DATA") {
            return Err(Error::NoData);
        }
        parse_elm_hex_payload(&raw)
    }

    /// Set 11-bit CAN header for ECU targeting (e.g. `7E0` for ECM).
    pub fn set_header(&mut self, header_hex: &str) -> Result<()> {
        let _ = self.cmd(&format!("ATSH{header_hex}"))?;
        Ok(())
    }

    /// STN: try dual-bus / MS-CAN helpers when available (ignore failures).
    pub fn try_stn_hints(&mut self) {
        let _ = self.cmd("STP 33"); // ISO 15765-4 CAN 11/500 often already set
        let _ = self.cmd("STCSWM 1"); // allow STN extended if present
    }
}

/// Strip prompts, spaces, line noise → printable summary.
pub fn clean_response(raw: &str) -> String {
    raw.chars()
        .filter(|c| *c != '>' && *c != '\r')
        .collect::<String>()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && *l != "OK" && !l.starts_with("SEARCHING"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Collect hex pairs from ELM response lines into bytes.
///
/// Handles:
/// - Flat hex (`410C0A8E`)
/// - Multi-line ELM ISO-TP style (`0:62F190…` / `1:4557…`) — **line index is not data**
/// - Optional leading length word (`01B`) discarded when `N:` lines follow
pub fn parse_elm_hex_payload(raw: &str) -> Result<Vec<u8>> {
    let mut hex = String::new();
    let mut saw_indexed = false;
    let mut length_prefix: Option<String> = None;

    // ELM uses CR-separated lines; `str::lines` alone misses bare `\r`.
    for line in raw.split(['\n', '\r']) {
        let t = line.trim().trim_end_matches('>');
        let t = t.trim();
        if t.is_empty() || t == ">" || t.eq_ignore_ascii_case("OK") {
            continue;
        }
        if t.eq_ignore_ascii_case("SEARCHING...") || t.eq_ignore_ascii_case("SEARCHING") {
            continue;
        }
        if t.eq_ignore_ascii_case("NO DATA") {
            return Err(Error::NoData);
        }
        if t.starts_with("AT") || t.starts_with("ST") {
            continue;
        }
        // ELM multi-frame: "0:AABBCC" or "1:DDEE"
        if let Some((idx, rest)) = t.split_once(':') {
            if !idx.is_empty()
                && idx.chars().all(|c| c.is_ascii_hexdigit())
                && idx.len() <= 2
                && rest.chars().any(|c| c.is_ascii_hexdigit())
            {
                saw_indexed = true;
                for ch in rest.chars() {
                    if ch.is_ascii_hexdigit() {
                        hex.push(ch);
                    }
                }
                continue;
            }
        }
        // Standalone length line before indexed payload (e.g. "01B")
        let only_hex: String = t.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if !saw_indexed
            && only_hex.len() <= 4
            && only_hex.len() >= 2
            && t.chars().all(|c| c.is_ascii_hexdigit() || c.is_whitespace())
        {
            // Might be length or short SF — hold; if later lines are indexed, drop it.
            if length_prefix.is_none() && only_hex.len() <= 3 {
                length_prefix = Some(only_hex);
                continue;
            }
        }
        for ch in t.chars() {
            if ch.is_ascii_hexdigit() {
                hex.push(ch);
            }
        }
    }

    // If we never saw indexed lines, include any held length prefix as data (single-frame).
    if !saw_indexed {
        if let Some(p) = length_prefix {
            hex = format!("{p}{hex}");
        }
    }
    // Odd nibble: pad is wrong — fail cleanly
    if hex.len() < 2 {
        return Err(Error::Decode(format!("bad hex payload: {raw:?}")));
    }
    if hex.len() % 2 != 0 {
        // Drop trailing nibble from incomplete last frame (common on last CF)
        hex.pop();
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let b = hex.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        let s = std::str::from_utf8(&b[i..i + 2]).unwrap();
        out.push(u8::from_str_radix(s, 16).map_err(|e| Error::Decode(format!("hex byte: {e}")))?);
        i += 2;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode01_rpm() {
        let raw = "410C0A8E\r\n>";
        let b = parse_elm_hex_payload(raw).unwrap();
        assert_eq!(b, vec![0x41, 0x0C, 0x0A, 0x8E]);
    }

    #[test]
    fn parse_multiline_vin_indexed() {
        // Real MX+ multi-line style (length + N:payload). Must not include line indices.
        let raw = "01B\r0:62F190314654\r1:4557314550394B\r2:46433733343939\r3:000000000000\r\r>";
        let b = parse_elm_hex_payload(raw).unwrap();
        assert!(b.starts_with(&[0x62, 0xF1, 0x90]), "got {b:02X?}");
        let ascii: String = b[3..]
            .iter()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| *c as char)
            .collect();
        assert!(
            ascii.starts_with("1FTEW1"),
            "VIN ascii {ascii} from {b:02X?}"
        );
    }
}
