//! CMFD **format selection** — MLU M1 Pilot’s Guide rules.
//!
//! OSB **12 / 13 / 14** hold three format options. The active option is
//! highlighted. Pressing the active option opens the Master Menu; pressing
//! another option switches format. See `docs/reference/mlu-m1-cmfd.md`.

use super::Format;

/// Which of the three format-select OSBs is pressed: 14, 13, or 12.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatSlot {
    /// OSB 14 (leftmost of the three in bottom row L→R after SWAP).
    Osb14 = 0,
    /// OSB 13.
    Osb13 = 1,
    /// OSB 12 (rightmost of the three before DCLT).
    Osb12 = 2,
}

impl FormatSlot {
    pub fn osb(self) -> u8 {
        match self {
            FormatSlot::Osb14 => 14,
            FormatSlot::Osb13 => 13,
            FormatSlot::Osb12 => 12,
        }
    }

    pub fn from_osb(osb: u8) -> Option<Self> {
        match osb {
            14 => Some(FormatSlot::Osb14),
            13 => Some(FormatSlot::Osb13),
            12 => Some(FormatSlot::Osb12),
            _ => None,
        }
    }
}

/// Result of handling a format-select / menu OSB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatSelectAction {
    /// Display this format (slot switch or menu pick).
    Show(Format),
    /// Open Master Menu (assigning into `for_slot`).
    OpenMenu { for_slot: FormatSlot },
    /// Close menu without change.
    CloseMenu,
    /// No format-select handling (caller may use page-local OSBs).
    Ignore,
}

/// One CMFD’s three format options + active page (MLU M1).
#[derive(Clone, Debug)]
pub struct FormatSelect {
    /// Formats on OSB 14, 13, 12.
    pub slots: [Format; 3],
    /// Which slot is active / highlighted.
    pub active: FormatSlot,
    /// Master Menu open; next format pick assigns into `menu_target`.
    pub menu_open: bool,
    pub menu_target: FormatSlot,
    /// Double-tap helper when slots are blank.
    last_blank_osb: Option<(u8, u32)>,
}

impl Default for FormatSelect {
    fn default() -> Self {
        // Typical left-MFD NAV/A-A: FCR primary, HSD, SMS (public training default).
        Self {
            slots: [Format::Fcr, Format::Hsd, Format::Sms],
            active: FormatSlot::Osb14,
            menu_open: false,
            menu_target: FormatSlot::Osb14,
            last_blank_osb: None,
        }
    }
}

impl FormatSelect {
    pub fn current(&self) -> Format {
        if self.menu_open {
            Format::Menu
        } else {
            self.slots[self.active as usize]
        }
    }

    pub fn slot_format(&self, slot: FormatSlot) -> Format {
        self.slots[slot as usize]
    }

    /// Mnemonics for OSB 14 / 13 / 12 (for chrome).
    pub fn slot_labels(&self) -> [&'static str; 3] {
        [
            self.slots[0].name(),
            self.slots[1].name(),
            self.slots[2].name(),
        ]
    }

    /// Handle OSB for format select / menu. `tick` is a monotonic counter for double-tap.
    pub fn handle_osb(&mut self, osb: u8, tick: u32) -> FormatSelectAction {
        // Master Menu: pick a format from menu OSBs.
        if self.menu_open {
            if let Some(fmt) = format_from_master_menu_osb(osb) {
                self.assign(self.menu_target, fmt);
                self.menu_open = false;
                self.active = self.menu_target;
                return FormatSelectAction::Show(fmt);
            }
            // Press format slot OSBs to cancel.
            if FormatSlot::from_osb(osb).is_some() {
                self.menu_open = false;
                return FormatSelectAction::CloseMenu;
            }
            return FormatSelectAction::Ignore;
        }

        // OSB 12/13/14 format options.
        if let Some(slot) = FormatSlot::from_osb(osb) {
            let fmt = self.slots[slot as usize];
            if slot == self.active {
                // Active → Master Menu.
                self.menu_open = true;
                self.menu_target = slot;
                return FormatSelectAction::OpenMenu { for_slot: slot };
            }
            // Blank slot: double-press opens menu (MLU M1).
            if matches!(fmt, Format::Blank) {
                if self.last_blank_osb == Some((osb, tick.wrapping_sub(1)))
                    || self.last_blank_osb.map(|(o, _)| o) == Some(osb)
                {
                    self.menu_open = true;
                    self.menu_target = slot;
                    self.last_blank_osb = None;
                    return FormatSelectAction::OpenMenu { for_slot: slot };
                }
                self.last_blank_osb = Some((osb, tick));
                return FormatSelectAction::Ignore;
            }
            self.active = slot;
            self.last_blank_osb = None;
            return FormatSelectAction::Show(fmt);
        }

        FormatSelectAction::Ignore
    }

    /// Assign format to a slot; if that format is on another slot, blank the other (except Blank).
    pub fn assign(&mut self, slot: FormatSlot, fmt: Format) {
        if !matches!(fmt, Format::Blank) {
            for (i, s) in self.slots.iter_mut().enumerate() {
                if i != slot as usize && *s == fmt {
                    *s = Format::Blank;
                }
            }
        }
        self.slots[slot as usize] = fmt;
    }
}

/// Master Menu OSB → format (Figure 1-15 style).
fn format_from_master_menu_osb(osb: u8) -> Option<Format> {
    match osb {
        1 => Some(Format::Blank),
        2 => Some(Format::Had),
        // 3 empty on some figures
        4 => Some(Format::Blank), // RCCE not modeled as separate yet
        5 => Some(Format::Reset),
        6 => Some(Format::Sms),
        7 => Some(Format::Hsd),
        8 => Some(Format::Dte),
        9 => Some(Format::Test),
        10 => Some(Format::Blank), // FLCS
        16 => Some(Format::Flir),
        17 => Some(Format::Tfr),
        18 => Some(Format::Wpn),
        19 => Some(Format::Tgp),
        20 => Some(Format::Fcr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_osb_opens_menu() {
        let mut fs = FormatSelect::default();
        assert_eq!(fs.current(), Format::Fcr);
        let a = fs.handle_osb(14, 1);
        assert_eq!(
            a,
            FormatSelectAction::OpenMenu {
                for_slot: FormatSlot::Osb14
            }
        );
        assert!(fs.menu_open);
        assert_eq!(fs.current(), Format::Menu);
    }

    #[test]
    fn other_slot_switches() {
        let mut fs = FormatSelect::default();
        let a = fs.handle_osb(13, 1);
        assert_eq!(a, FormatSelectAction::Show(Format::Hsd));
        assert_eq!(fs.current(), Format::Hsd);
    }

    #[test]
    fn menu_pick_assigns_and_dedups() {
        let mut fs = FormatSelect::default();
        fs.handle_osb(14, 1); // menu
        let a = fs.handle_osb(7, 2); // HSD from menu onto slot 14
        assert_eq!(a, FormatSelectAction::Show(Format::Hsd));
        assert_eq!(fs.slots[0], Format::Hsd);
        // HSD was also on slot 13 → blanked
        assert_eq!(fs.slots[1], Format::Blank);
    }
}
