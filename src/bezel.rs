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

/// POC keyboard → OSB 1–20 (**linear row**, not face-spatial).
///
/// ```text
/// Keys:  1 2 3 4 5 6 7 8 9 0 q w e r t y u i o p
/// OSB:   1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20
/// ```
///
/// Face reminder (OSB numbers still clockwise on glass):
/// top 1–5 · right 6–10 · bottom 11–15 right→left as DCLT…OWN · left 16–20.
///
/// Rockers (lab):
/// - `[` `]` — format **prev / next** (lab only; not production face)
/// - `-` `=` — **brightness** − / +
/// - `;` `'` — contrast − / +
/// - `,` `.` — gain − / +
///
/// **Hard rule:** OSB 1–10 are options / Master Menu picks — not permanent format jumps.
#[derive(Default)]
pub struct KeyboardBezel {
    pending: Vec<BezelEvent>,
}

impl KeyboardBezel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Linear map: `1234567890qwertyuiop` → OSB 1..=20.
    pub fn push_key(&mut self, key: u8) {
        let k = key.to_ascii_lowercase();
        let osb = match k {
            b'1' => 1,
            b'2' => 2,
            b'3' => 3,
            b'4' => 4,
            b'5' => 5,
            b'6' => 6,
            b'7' => 7,
            b'8' => 8,
            b'9' => 9,
            b'0' => 10,
            b'q' => 11, // DCLT (bottom-right on glass)
            b'w' => 12, // format C
            b'e' => 13, // format B
            b'r' => 14, // format A
            b't' => 15, // OWN
            b'y' => 16, // DTC
            b'u' => 17,
            b'i' => 18,
            b'o' => 19, // SET
            b'p' => 20, // BUS
            _ => 0,
        };
        if osb != 0 {
            // Lab keys are edges: Down then Up so SOI does not stick on last press.
            self.pending.push(BezelEvent::OsbDown(osb));
            self.pending.push(BezelEvent::OsbUp(osb));
        }
    }

    /// Feed key with access to current levels (for ± rockers).
    pub fn push_key_state(&mut self, key: u8, st: &BezelState) {
        match key {
            // Brightness rocker (production BRT) — on -/= so [ ] can be prev/next
            b'-' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Brightness,
                (st.brightness - 0.05).max(0.0),
            )),
            b'=' => self.pending.push(BezelEvent::Knob(
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
            // SYM rocker (lab): use shifted punctuation pair free of BRT
            // (BRT is -/=). Map unused keys: we use '9'/'0' are OSB — keep ,
            // . for gain. SYM via PageUp not available → use backslash/pipe.
            b'\\' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Symbology,
                (st.symbology - 0.05).max(0.0),
            )),
            b'|' => self.pending.push(BezelEvent::Knob(
                BezelKnob::Symbology,
                (st.symbology + 0.05).min(1.0),
            )),
            b',' => self
                .pending
                .push(BezelEvent::Knob(BezelKnob::Gain, (st.gain - 0.05).max(0.0))),
            b'.' => self
                .pending
                .push(BezelEvent::Knob(BezelKnob::Gain, (st.gain + 0.05).min(1.0))),
            // [ ] handled in cmfd as format prev/next (lab), not here
            b'[' | b']' => {}
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
