//! Fixed facts for the operator’s truck + As-Built **feature labels** for SETUP help.
//!
//! Labels come from the FORScan Common sheet (read file only). **Never** write
//! As-Built to the vehicle.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Model year.
pub const YEAR: u16 = 2019;
/// Marketing / cab: SuperCrew = 4-door full rear seat.
pub const CAB: &str = "SuperCrew";
/// Driveline.
pub const DRIVE: &str = "4x4";
/// Engine short name.
pub const ENGINE: &str = "2.7L EcoBoost";
/// APIM / head unit.
pub const APIM: &str = "Sync 3";
/// Platform family.
pub const PLATFORM: &str = "P552 F-150";
/// Default Bluetooth ELM MAC (truck adapter).
pub const OBD_BT_MAC: &str = "00:04:3E:96:B8:F1";

/// One-line identity for glass (no VIN).
pub fn identity_line() -> String {
    format!("{YEAR} {PLATFORM} · {CAB} {DRIVE} · {ENGINE} · {APIM}")
}

/// Path to FORScan Common.csv (repo layout).
pub fn common_csv_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/reference/ford-f150-forscan/00_Common.csv")
}

/// Feature labels from Common.csv column 0 (Feature Name), non-empty, unique order.
pub fn asbuilt_feature_labels() -> &'static [String] {
    static LABELS: OnceLock<Vec<String>> = OnceLock::new();
    LABELS.get_or_init(|| load_feature_labels(&common_csv_path()))
}

fn load_feature_labels(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut past_header = false;
    for line in text.lines() {
        // CSV: Feature Name,Module,Address,...
        let name = line.split(',').next().unwrap_or("").trim();
        if name.is_empty() || name.starts_with("**") || name.starts_with("Last Revised") {
            continue;
        }
        if name.eq_ignore_ascii_case("Feature Name") {
            past_header = true;
            continue;
        }
        if !past_header {
            // some rows before header
            if name.contains("Disable") || name.contains("DRL") || name.contains("Auto") {
                past_header = true;
            } else {
                continue;
            }
        }
        // skip pure module-continuation rows that have empty feature (already handled empty)
        // strip quotes
        let name = name.trim_matches('"').trim();
        if name.len() < 3 || name.starts_with(',') {
            continue;
        }
        // skip address-looking only
        if name.len() <= 12
            && name.contains('-')
            && name.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        {
            continue;
        }
        if seen.insert(name.to_string()) {
            out.push(name.to_string());
        }
        if out.len() >= 200 {
            break;
        }
    }
    out
}

/// Short labels for SETUP list (fit glass).
/// Prefer live [`crate::auto::VehicleSnapshot::bus_link_lines`] when OBD is up.
pub fn setup_help_lines(max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(identity_line());
    lines.push(format!("OBD BT  {OBD_BT_MAC}"));
    lines.push("BT CH   1  (MFD_OBD_BT_CHANNEL)".into());
    lines.push("ENV     MFD_OBD_BT / PORT / REPLAY".into());
    lines.push("DISPLAY ONLY  no As-Built write".into());
    lines.push("FEATURES (ref labels)".into());
    for lab in asbuilt_feature_labels().iter().take(max.saturating_sub(6)) {
        // trim long names for mono face
        let s = if lab.len() > 28 {
            format!("{}…", &lab[..27])
        } else {
            lab.clone()
        };
        lines.push(s);
    }
    if lines.len() == 6 {
        lines.push("(no Common.csv labels)".into());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_mentions_sync3_and_2019() {
        let s = identity_line();
        assert!(s.contains("2019"));
        assert!(s.contains("Sync 3"));
        assert!(s.contains("2.7"));
        assert!(s.contains("SuperCrew"));
    }

    #[test]
    fn common_csv_yields_labels() {
        let labs = asbuilt_feature_labels();
        assert!(!labs.is_empty(), "expected feature labels from Common.csv");
        assert!(labs
            .iter()
            .any(|l| l.contains("DRL") || l.contains("Start")));
    }
}
