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
        Self {
            slots: [
                Some(AutoPage::Eng),
                Some(AutoPage::Drive),
                Some(AutoPage::Fuel),
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
    /// Seed slots from probe-allowed pages (first three GO core formats).
    pub fn from_allowed(allowed: &[AutoPage]) -> Self {
        let prefer = [
            AutoPage::Eng,
            AutoPage::Drive,
            AutoPage::Fuel,
            AutoPage::Fluid,
            AutoPage::Elec,
            AutoPage::Attitude,
            AutoPage::Faults,
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
            if !picked.contains(&p) && !matches!(p, AutoPage::Own | AutoPage::Setup) {
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
        self.slots[self.active as usize].unwrap_or(AutoPage::Eng)
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
        if osb == 15 {
            self.menu_open = false;
            return FormatSelectAction::Own;
        }
        if osb == 11 && !self.menu_open {
            self.cycle_dclt();
            return FormatSelectAction::Declutter;
        }

        if self.menu_open {
            if let Some(page) = page_from_master_menu_osb(osb) {
                if allowed.is_empty() || allowed.contains(&page) {
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

        if let Some(slot) = FormatSlot::from_osb(osb) {
            let fmt = self.slots[slot as usize];
            if slot == self.active {
                self.menu_open = true;
                self.menu_target = slot;
                return FormatSelectAction::OpenMenu { for_slot: slot };
            }
            if fmt.is_none() {
                if self.last_blank_osb.map(|(o, _)| o) == Some(osb) {
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
        16 => Some(AutoPage::Faults),
        17 => Some(AutoPage::Map),
        18 => Some(AutoPage::Attitude),
        19 => Some(AutoPage::Setup),
        20 => Some(AutoPage::Bus),
        // 15 OWN handled before menu pick
        _ => None,
    }
}

/// Master Menu legends (blank if page not allowed).
pub fn master_menu_legends(
    allowed: &[AutoPage],
) -> (
    [&'static str; 5],
    [&'static str; 5],
    [&'static str; 5],
    [&'static str; 5],
) {
    let lab = |p: AutoPage| {
        if allowed.is_empty() || allowed.contains(&p) {
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
    // Bottom: OWN · (empty slots during menu) · cancel via format OSBs
    let bottom = ["OWN", "", "", "", ""];
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
}
