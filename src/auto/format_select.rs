//! Vehicle CMFD **format selection** — MLU-class OSB 12 / 13 / 14 habit.
//!
//! See `docs/reference/vehicle-cmfd-design.md`.

use crate::auto::AutoPage;

/// Format-select OSB: 14, 13, or 12 (bottom row, L→R after OWN).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatSlot {
    Osb14 = 0,
    Osb13 = 1,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatSelectAction {
    Show(AutoPage),
    OpenMenu {
        for_slot: FormatSlot,
    },
    CloseMenu,
    Own,
    Declutter,
    /// Page-local option OSB (caller handles).
    Ignore,
}

/// Three format slots + Master Menu + declutter level.
#[derive(Clone, Debug)]
pub struct AutoFormatSelect {
    /// Formats on OSB 14, 13, 12 (`None` = blank slot).
    pub slots: [Option<AutoPage>; 3],
    pub active: FormatSlot,
    pub menu_open: bool,
    pub menu_target: FormatSlot,
    /// 0 = full, 1 = reduced numerics, 2 = gauges / primary only.
    pub dclt: u8,
    last_blank_osb: Option<(u8, u32)>,
}

impl Default for AutoFormatSelect {
    fn default() -> Self {
        // Default slots: tach · speedo · ATT (operator hard formats).
        Self {
            slots: [
                Some(AutoPage::Eng),
                Some(AutoPage::Drive),
                Some(AutoPage::Attitude),
            ],
            active: FormatSlot::Osb14,
            menu_open: false,
            menu_target: FormatSlot::Osb14,
            dclt: 0,
            last_blank_osb: None,
        }
    }
}

impl AutoFormatSelect {
    /// Seed three format slots from GO formats.
    ///
    /// Default preference: **ENG · DRV · ATT** (tach, speedo, horizon) when present.
    pub fn from_allowed(allowed: &[AutoPage]) -> Self {
        let prefer = [
            AutoPage::Eng,      // tach
            AutoPage::Drive,    // speedo
            AutoPage::Attitude, // horizon / heading
            AutoPage::Faults,   // DTC
            AutoPage::Fuel,
            AutoPage::Fluid,
            AutoPage::Elec,
        ];
        let mut picked = Vec::new();
        for p in prefer {
            if allowed.contains(&p) && !picked.contains(&p) {
                picked.push(p);
            }
            if picked.len() == 3 {
                break;
            }
        }
        for &p in allowed {
            if picked.len() == 3 {
                break;
            }
            if !picked.contains(&p) && !matches!(p, AutoPage::Own | AutoPage::Setup | AutoPage::Bus)
            {
                picked.push(p);
            }
        }
        let mut slots = [None; 3];
        for (i, p) in picked.into_iter().take(3).enumerate() {
            slots[i] = Some(p);
        }
        Self {
            slots,
            active: FormatSlot::Osb14,
            menu_open: false,
            menu_target: FormatSlot::Osb14,
            dclt: 0,
            last_blank_osb: None,
        }
    }

    pub fn current(&self) -> AutoPage {
        if self.menu_open {
            return AutoPage::Setup; // menu drawn specially; placeholder
        }
        if let Some(p) = self.slots[self.active as usize] {
            return p;
        }
        // Never invent ENG for empty active slot unless no slot is filled.
        self.slots
            .iter()
            .flatten()
            .copied()
            .next()
            .unwrap_or(AutoPage::Eng)
    }

    /// True if this format may be shown (GO list). Empty allow = none after boot.
    pub fn is_allowed(page: AutoPage, allowed: &[AutoPage]) -> bool {
        !allowed.is_empty() && allowed.contains(&page)
    }

    pub fn slot_labels(&self) -> [&'static str; 3] {
        [
            self.slots[0].map(|p| p.name()).unwrap_or(""),
            self.slots[1].map(|p| p.name()).unwrap_or(""),
            self.slots[2].map(|p| p.name()).unwrap_or(""),
        ]
    }

    pub fn cycle_dclt(&mut self) {
        self.dclt = (self.dclt + 1) % 3;
    }

    pub fn assign(&mut self, slot: FormatSlot, page: AutoPage) {
        for (i, s) in self.slots.iter_mut().enumerate() {
            if i != slot as usize && *s == Some(page) {
                *s = None;
            }
        }
        self.slots[slot as usize] = Some(page);
    }

    /// Handle global format-select OSBs. `allowed` = probe GO formats.
    pub fn handle_osb(&mut self, osb: u8, tick: u32, allowed: &[AutoPage]) -> FormatSelectAction {
        // Master Menu has priority: OSB 11 = RNG, 15 = OWN pick (not DCLT/OWN jump).
        if self.menu_open {
            if let Some(page) = page_from_master_menu_osb(osb) {
                if Self::is_allowed(page, allowed) {
                    self.assign(self.menu_target, page);
                    self.menu_open = false;
                    self.active = self.menu_target;
                    return FormatSelectAction::Show(page);
                }
                return FormatSelectAction::Ignore;
            }
            if FormatSlot::from_osb(osb).is_some() {
                self.menu_open = false;
                return FormatSelectAction::CloseMenu;
            }
            return FormatSelectAction::Ignore;
        }

        if osb == 15 {
            return FormatSelectAction::Own;
        }
        if osb == 11 {
            self.cycle_dclt();
            return FormatSelectAction::Declutter;
        }

        if let Some(slot) = FormatSlot::from_osb(osb) {
            let fmt = self.slots[slot as usize];
            if slot == self.active {
                self.menu_open = true;
                self.menu_target = slot;
                return FormatSelectAction::OpenMenu { for_slot: slot };
            }
            if fmt.is_none() {
                const TAP_WINDOW: u32 = 45;
                if let Some((o, t0)) = self.last_blank_osb {
                    if o == osb && tick.wrapping_sub(t0) <= TAP_WINDOW {
                        self.menu_open = true;
                        self.menu_target = slot;
                        self.last_blank_osb = None;
                        return FormatSelectAction::OpenMenu { for_slot: slot };
                    }
                }
                self.last_blank_osb = Some((osb, tick));
                return FormatSelectAction::Ignore;
            }
            self.active = slot;
            self.last_blank_osb = None;
            return FormatSelectAction::Show(fmt.unwrap());
        }

        FormatSelectAction::Ignore
    }
}

/// Master Menu OSB → format (stable positions; blank if not GO at draw time).
pub fn page_from_master_menu_osb(osb: u8) -> Option<AutoPage> {
    match osb {
        1 => Some(AutoPage::Eng),
        2 => Some(AutoPage::Fuel),
        3 => Some(AutoPage::Fluid),
        4 => Some(AutoPage::Elec),
        5 => Some(AutoPage::Drive),
        6 => Some(AutoPage::Chas),
        7 => Some(AutoPage::Body),
        8 => Some(AutoPage::Lights),
        9 => Some(AutoPage::Clim),
        10 => Some(AutoPage::Cam),
        11 => Some(AutoPage::Range), // DCLT only when menu closed
        15 => Some(AutoPage::Own),
        16 => Some(AutoPage::Faults),
        17 => Some(AutoPage::Map),
        18 => Some(AutoPage::Attitude),
        19 => Some(AutoPage::Setup),
        20 => Some(AutoPage::Bus),
        _ => None,
    }
}

/// Master Menu legends (blank if page not allowed). Empty allowed → all blank.
pub fn master_menu_legends(
    allowed: &[AutoPage],
) -> (
    [&'static str; 5],
    [&'static str; 5],
    [&'static str; 5],
    [&'static str; 5],
) {
    let lab = |p: AutoPage| {
        if AutoFormatSelect::is_allowed(p, allowed) {
            p.name()
        } else {
            ""
        }
    };
    let top = [
        lab(AutoPage::Eng),
        lab(AutoPage::Fuel),
        lab(AutoPage::Fluid),
        lab(AutoPage::Elec),
        lab(AutoPage::Drive),
    ];
    let right = [
        lab(AutoPage::Chas),
        lab(AutoPage::Body),
        lab(AutoPage::Lights),
        lab(AutoPage::Clim),
        lab(AutoPage::Cam),
    ];
    let left = [
        lab(AutoPage::Bus),
        lab(AutoPage::Setup),
        lab(AutoPage::Attitude),
        lab(AutoPage::Map),
        lab(AutoPage::Faults),
    ];
    // Bottom OSB 15..11: OWN · · · · RNG
    let bottom = [lab(AutoPage::Own), "", "", "", lab(AutoPage::Range)];
    (top, right, bottom, left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_opens_menu() {
        let mut fs = AutoFormatSelect::default();
        let a = fs.handle_osb(14, 1, AutoPage::ALL);
        assert!(matches!(a, FormatSelectAction::OpenMenu { .. }));
        assert!(fs.menu_open);
    }

    #[test]
    fn other_slot_switches() {
        let mut fs = AutoFormatSelect::default();
        let a = fs.handle_osb(13, 1, AutoPage::ALL);
        assert_eq!(a, FormatSelectAction::Show(AutoPage::Drive));
        assert_eq!(fs.current(), AutoPage::Drive);
    }

    #[test]
    fn default_slots_are_eng_drv_att() {
        let fs = AutoFormatSelect::default();
        assert_eq!(fs.slots[0], Some(AutoPage::Eng));
        assert_eq!(fs.slots[1], Some(AutoPage::Drive));
        assert_eq!(fs.slots[2], Some(AutoPage::Attitude));
    }

    #[test]
    fn menu_pick_dedups() {
        let mut fs = AutoFormatSelect::default();
        fs.handle_osb(14, 1, AutoPage::ALL);
        let a = fs.handle_osb(5, 2, AutoPage::ALL); // DRV onto slot 14
        assert_eq!(a, FormatSelectAction::Show(AutoPage::Drive));
        assert_eq!(fs.slots[0], Some(AutoPage::Drive));
        assert_eq!(fs.slots[1], None); // was Drive
    }

    #[test]
    fn nogo_not_assigned() {
        let mut fs = AutoFormatSelect::default();
        fs.handle_osb(14, 1, &[AutoPage::Eng, AutoPage::Fuel]);
        let a = fs.handle_osb(10, 2, &[AutoPage::Eng, AutoPage::Fuel]); // CAM nogo
        assert_eq!(a, FormatSelectAction::Ignore);
    }

    #[test]
    fn empty_allowed_rejects_menu_pick() {
        let mut fs = AutoFormatSelect::default();
        fs.handle_osb(14, 1, AutoPage::ALL);
        assert!(fs.menu_open);
        let a = fs.handle_osb(1, 2, &[]); // empty GO set
        assert_eq!(a, FormatSelectAction::Ignore);
    }

    #[test]
    fn blank_double_tap_opens_menu() {
        let mut fs = AutoFormatSelect::default();
        fs.slots[1] = None; // OSB 13 blank
        let a1 = fs.handle_osb(13, 10, AutoPage::ALL);
        assert_eq!(a1, FormatSelectAction::Ignore);
        let a2 = fs.handle_osb(13, 20, AutoPage::ALL); // within window
        assert!(matches!(a2, FormatSelectAction::OpenMenu { .. }));
    }
}
