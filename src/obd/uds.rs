//! UDS (ISO 14229) over ELM/ISO-TP — **read-only (display-only CMFD)**.
//!
//! # Safety — display only
//!
//! The color MFD (**CMFD**) **only displays** vehicle data. It must **never**:
//! clear DTCs, write DIDs, security-unlock, program, or command actuators.
//!
//! Allowed request SIDs (read path only):
//! - `0x10` DiagnosticSessionControl — open extended **read** session when needed
//! - `0x3E` TesterPresent — keep-alive for that session
//! - `0x22` ReadDataByIdentifier — data for glass
//! - `0x19` ReadDTCInformation — DTC detail (read only)
//!
//! All other SIDs are rejected in software (no env override).
//! See `docs/reference/ford-f150-uds-readonly.md`.

use crate::obd::elm::Elm;
use crate::obd::error::{Error, Result};
use crate::obd::isotp;

/// Product policy: CMFD never mutates the vehicle.
pub const DISPLAY_ONLY: bool = true;

/// Default ECM request header on 11-bit OBD (functional often 7DF; physical 7E0).
pub const DEFAULT_ECM_HEADER: &str = "7E0";
pub const DEFAULT_FUNCTIONAL_HEADER: &str = "7DF";

/// SIDs permitted on the wire from this product.
const ALLOWED_SIDS: &[u8] = &[
    0x10, // DiagnosticSessionControl (read path)
    0x19, // ReadDTCInformation (read)
    0x22, // ReadDataByIdentifier
    0x3E, // TesterPresent
];

/// True if this UDS service ID is allowed for display-only operation.
pub fn is_read_only_sid(sid: u8) -> bool {
    ALLOWED_SIDS.contains(&sid)
}

/// Format bytes as continuous uppercase hex for ELM.
pub fn hex_bytes(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

/// Send raw UDS request bytes (ISO-TP single-frame via ELM) to current header.
///
/// Rejects any SID outside the display-only allow-list. **No override.**
pub fn request(elm: &mut Elm, payload: &[u8]) -> Result<Vec<u8>> {
    let sid = *payload
        .first()
        .ok_or_else(|| Error::Protocol("empty UDS request".into()))?;
    if !is_read_only_sid(sid) {
        return Err(Error::DisplayOnly(format!(
            "UDS SID 0x{sid:02X} forbidden (CMFD display-only)"
        )));
    }
    let cmd = hex_bytes(payload);
    let raw = elm.cmd(&cmd)?;
    if raw.to_ascii_uppercase().contains("NO DATA") {
        return Err(Error::NoData);
    }
    let bytes = crate::obd::elm::parse_elm_hex_payload(&raw)?;
    // Prefer ISO-TP reassembly; if that fails, use raw (already-stripped ELM stream).
    match isotp::reassemble(&bytes) {
        Ok(p) => Ok(p),
        Err(_) => Ok(bytes),
    }
}

/// `0x10` DiagnosticSessionControl — e.g. `0x01` default, `0x03` extended (for reads).
pub fn diagnostic_session_control(elm: &mut Elm, session: u8) -> Result<Vec<u8>> {
    request(elm, &[0x10, session])
}

/// `0x3E` TesterPresent (keep-alive). `suppress_pos` uses sub-function `0x80`.
pub fn tester_present(elm: &mut Elm, suppress_pos: bool) -> Result<()> {
    let sub = if suppress_pos { 0x80 } else { 0x00 };
    match request(elm, &[0x3E, sub]) {
        Ok(_) | Err(Error::NoData) => Ok(()),
        Err(e) => Err(e),
    }
}

/// `0x22` ReadDataByIdentifier — 16-bit DID.
pub fn read_data_by_identifier(elm: &mut Elm, did: u16) -> Result<Vec<u8>> {
    let hi = (did >> 8) as u8;
    let lo = (did & 0xFF) as u8;
    let resp = request(elm, &[0x22, hi, lo])?;
    if resp.first() == Some(&0x7F) {
        return Err(Error::Protocol(format!("UDS NRC: {resp:02X?}")));
    }
    // Positive response may be bare 62 DID_H DID_L data… or still have PCI.
    let resp = match isotp::reassemble(&resp) {
        Ok(p) if !p.is_empty() => p,
        _ => resp,
    };
    if resp.first() == Some(&0x7F) {
        return Err(Error::Protocol(format!("UDS NRC: {resp:02X?}")));
    }
    if resp.len() >= 3 && resp[0] == 0x62 {
        return Ok(resp[3..].to_vec());
    }
    // Sometimes ELM already stripped SID+DID
    Ok(resp)
}

/// SecurityAccess (`0x27`) — **always forbidden**. CMFD never unlocks modules.
pub fn security_access(_elm: &mut Elm, _sub_function: u8, _key: Option<&[u8]>) -> Result<Vec<u8>> {
    let _ = DISPLAY_ONLY;
    Err(Error::DisplayOnly(
        "SecurityAccess 0x27 forbidden (CMFD display-only)".into(),
    ))
}

/// Set header, extended session, keep-alive, read DID (read path only).
pub fn read_did_on_module(elm: &mut Elm, header_hex: &str, did: u16) -> Result<Vec<u8>> {
    elm.set_header(header_hex)?;
    let _ = diagnostic_session_control(elm, 0x03);
    let _ = tester_present(elm, true);
    read_data_by_identifier(elm, did)
}

/// Generic identification DIDs (ISO / common). **Read only.**
pub const PROBE_DIDS: &[(u16, &str)] = &[
    (0xF190, "VIN"),
    (0xF191, "vehicle_manufacturer_ecu_hw"),
    (0xF18C, "ecu_serial"),
    (0xF187, "spare_part_number"),
    (0xF189, "hw_version"),
    (0xF1A0, "approval"),
];

/// `0x19` ReadDTCInformation — report number of DTCs by status mask (example).
pub fn read_dtc_number_by_status_mask(elm: &mut Elm, status_mask: u8) -> Result<Vec<u8>> {
    request(elm, &[0x19, 0x01, status_mask])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_list_is_read_only() {
        assert!(is_read_only_sid(0x22));
        assert!(is_read_only_sid(0x10));
        assert!(is_read_only_sid(0x3E));
        assert!(is_read_only_sid(0x19));
        assert!(!is_read_only_sid(0x27));
        assert!(!is_read_only_sid(0x2E));
        assert!(!is_read_only_sid(0x14));
        assert!(!is_read_only_sid(0x31));
        assert!(!is_read_only_sid(0x2F));
        assert!(!is_read_only_sid(0x34));
    }
}
