//! Bezel / OSB input model — **plug in real buttons later without rewriting pages**.
//!
//! # Hardware SoT
//!
//! Full production button types, OSB map, and rocker roles:
//! **`docs/hardware-bezel.md`**.
//!
//! # Layout (F-16-class 20-OSB)
//!
//! ```text
//!         1   2   3   4   5     top options (active format)
//!    20                       6
//!    19                       7
//!    18      [  GLASS  ]      8     right options
//!    17                       9
//!    16                      10
//!        15  14  13  12  11     OWN · fmtA · fmtB · fmtC · DCLT
//! ```
//!
//! Frozen product roles: OSB 11=DCLT, 12–14=format slots, 15=OWN,
//! 16=DTC, 19=SET, 20=BUS. See [`osb_role`].
//!
//! Corner knobs: BRT / CON / SYM / GAIN ([`BezelKnob`]).
//!
//! Pages only consume [`BezelEvent`]. Sources implement [`BezelSource`]:
//! keyboard (POC), later GPIO / HID.

/// OSB index **1..=20** (not zero-based). Matches face silkscreen.
pub type OsbId = u8;

/// Frozen production OSB numbers (hardware silkscreen / GPIO map).
pub mod osb_role {
    use super::OsbId;

    pub const DCLT: OsbId = 11;
    pub const FORMAT_C: OsbId = 12; // default ATT
    pub const FORMAT_B: OsbId = 13; // default DRV
    pub const FORMAT_A: OsbId = 14; // default ENG; active → Master Menu
    pub const OWN: OsbId = 15;
    pub const DTC: OsbId = 16;
    pub const SET: OsbId = 19;
    pub const BUS: OsbId = 20;

    /// Top option row OSB 1..=5.
    pub fn is_top_option(osb: OsbId) -> bool {
        (1..=5).contains(&osb)
    }
    /// Right option column OSB 6..=10.
    pub fn is_right_option(osb: OsbId) -> bool {
        (6..=10).contains(&osb)
    }
    /// Bottom format-select strip OSB 11..=15.
    pub fn is_bottom_format_strip(osb: OsbId) -> bool {
        (11..=15).contains(&osb)
    }
    /// Left support column OSB 16..=20.
    pub fn is_left_support(osb: OsbId) -> bool {
        (16..=20).contains(&osb)
    }
}

/// Continuous bezel controls (corners / rockers).
///
/// Hardware: absolute 0.0..=1.0 after host scaling, or relative steps
/// converted by firmware before emitting [`BezelEvent::Knob`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BezelKnob {
    /// Glass brightness (lower-left rocker class).
    Brightness,
    /// Glass contrast (lower-right rocker class).
    Contrast,
    /// Symbology intensity (upper-right class).
    Symbology,
    /// CAM/FLIR gain when video GO; else software no-op (upper-left class).
    Gain,
}

/// Edge-triggered and level events from a bezel.
///
/// **Production:** GPIO matrix emits only these variants.  
/// **POC:** [`KeyboardBezel`] maps keys → same events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BezelEvent {
    /// OSB pressed (`osb` in 1..=20).
    OsbDown(OsbId),
    /// OSB released (`osb` in 1..=20).
    OsbUp(OsbId),
    /// Rocker/knob absolute level 0.0..=1.0.
    Knob(BezelKnob, f32),
}

/// Live bezel state (levels + which OSBs are held).
#[derive(Clone, Debug)]
pub struct BezelState {
    pub osb_down: [bool; 21], // index 1..=20
    pub brightness: f32,
    pub contrast: f32,
    pub symbology: f32,
    pub gain: f32,
    /// Last OSB that went down (for UI highlight).
    pub last_osb: Option<OsbId>,
}

impl Default for BezelState {
    fn default() -> Self {
        Self {
            osb_down: [false; 21],
            brightness: 0.8,
            contrast: 0.7,
            symbology: 0.75,
            gain: 0.5,
            last_osb: None,
        }
    }
}

impl BezelState {
    pub fn apply(&mut self, ev: BezelEvent) {
        match ev {
            BezelEvent::OsbDown(id) if (1..=20).contains(&id) => {
                self.osb_down[id as usize] = true;
                self.last_osb = Some(id);
            }
            BezelEvent::OsbUp(id) if (1..=20).contains(&id) => {
                self.osb_down[id as usize] = false;
            }
            BezelEvent::Knob(BezelKnob::Brightness, v) => self.brightness = v.clamp(0.0, 1.0),
            BezelEvent::Knob(BezelKnob::Contrast, v) => self.contrast = v.clamp(0.0, 1.0),
            BezelEvent::Knob(BezelKnob::Symbology, v) => self.symbology = v.clamp(0.0, 1.0),
            BezelEvent::Knob(BezelKnob::Gain, v) => self.gain = v.clamp(0.0, 1.0),
            _ => {}
        }
    }

    pub fn is_down(&self, id: OsbId) -> bool {
        (1..=20).contains(&id) && self.osb_down[id as usize]
    }
}

/// Anything that can produce bezel events (keyboard, GPIO, HID, …).
pub trait BezelSource {
    fn poll(&mut self) -> Vec<BezelEvent>;
}

/// No hardware — empty events.
#[derive(Default)]
pub struct NullBezel;

impl BezelSource for NullBezel {
    fn poll(&mut self) -> Vec<BezelEvent> {
        Vec::new()
    }
}

/// POC: map ASCII keys → **dedicated OSB sides** (real bezel later).
///
/// | Side | Keys | OSB |
/// |------|------|-----|
/// | **Top options** | `1` `2` `3` `4` `5` | 1–5 |
/// | **Right options** | `6` `7` `8` `9` `0` | 6–10 |
/// | **Bottom** | `q` `w` `e` `r` `t` | 15–11 (OWN · slotA · slotB · slotC · DCLT) |
/// | **Left** | `a` `s` `d` `f` `g` | 16–20 (DTC · · · SET · BUS bottom→top) |
///
/// Knobs: `[` `]` BRT · `;` `'` CON · `-` `=` SYM · `,` `.` GAIN
///
/// **Hard rule:** these keys are **never** format-select shortcuts.
/// Format change is Master Menu / n·p / bottom slots 12–14 only.
/// On LIGHTS, `1`=`LO` … `5`=`INT` — not ENG/FUEL jumps.
#[derive(Default)]
pub struct KeyboardBezel {
    pending: Vec<BezelEvent>,
}

impl KeyboardBezel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed raw key bytes (from raw stdin).
    pub fn push_key(&mut self, key: u8) {
        match key {
            b'1' => self.pending.push(BezelEvent::OsbDown(1)),
            b'2' => self.pending.push(BezelEvent::OsbDown(2)),
            b'3' => self.pending.push(BezelEvent::OsbDown(3)),
            b'4' => self.pending.push(BezelEvent::OsbDown(4)),
            b'5' => self.pending.push(BezelEvent::OsbDown(5)),
            b'6' => self.pending.push(BezelEvent::OsbDown(6)),
            b'7' => self.pending.push(BezelEvent::OsbDown(7)),
            b'8' => self.pending.push(BezelEvent::OsbDown(8)),
            b'9' => self.pending.push(BezelEvent::OsbDown(9)),
            b'0' => self.pending.push(BezelEvent::OsbDown(10)),
            b'q' | b'Q' => self.pending.push(BezelEvent::OsbDown(15)),
            b'w' | b'W' => self.pending.push(BezelEvent::OsbDown(14)),
            b'e' | b'E' => self.pending.push(BezelEvent::OsbDown(13)),
            b'r' | b'R' => self.pending.push(BezelEvent::OsbDown(12)),
            b't' | b'T' => self.pending.push(BezelEvent::OsbDown(11)),
            b'a' | b'A' => self.pending.push(BezelEvent::OsbDown(16)),
            b's' | b'S' => self.pending.push(BezelEvent::OsbDown(17)),
            b'd' | b'D' => self.pending.push(BezelEvent::OsbDown(18)),
            b'f' | b'F' => self.pending.push(BezelEvent::OsbDown(19)),
            b'g' | b'G' => self.pending.push(BezelEvent::OsbDown(20)),
            b'[' => self
                .pending
                .push(BezelEvent::Knob(BezelKnob::Brightness, 0.0)), // filled by state
            _ => {}
        }
        // Nudge knobs with -/= and ;/'
        // Handled in push_key_with_state
    }

    /// Feed key with access to current levels (for ± nudges).
    pub fn push_key_state(&mut self, key: u8, st: &BezelState) {
        match key {
            b'[' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Brightness,
                (st.brightness - 0.05).max(0.0),
            )),
            b']' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Brightness,
                (st.brightness + 0.05).min(1.0),
            )),
            b';' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Contrast,
                (st.contrast - 0.05).max(0.0),
            )),
            b'\'' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Contrast,
                (st.contrast + 0.05).min(1.0),
            )),
            b'-' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Symbology,
                (st.symbology - 0.05).max(0.0),
            )),
            b'=' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Symbology,
                (st.symbology + 0.05).min(1.0),
            )),
            b',' => self
                .pending
                .push(BezelEvent::Knob(BezelKnob::Gain, (st.gain - 0.05).max(0.0))),
            b'.' => self
                .pending
                .push(BezelEvent::Knob(BezelKnob::Gain, (st.gain + 0.05).min(1.0))),
            _ => self.push_key(key),
        }
    }
}

impl BezelSource for KeyboardBezel {
    fn poll(&mut self) -> Vec<BezelEvent> {
        std::mem::take(&mut self.pending)
    }
}

/// Top-row OSB → format select index (demo / default binding).
pub fn top_osb_format_index(osb: OsbId) -> Option<usize> {
    match osb {
        1..=5 => Some((osb - 1) as usize),
        _ => None,
    }
}
