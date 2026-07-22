# F-16 MLU M1 Pilot’s Guide — CMFD model (agent digest)

**Source:** `docs/246416220-F16-MLU-M1-Pilot-s-Manual.pdf`  
**Title:** F-16 A/B Mid-Life Update Production Tape M1 — *The Pilot’s Guide to new capabilities & cockpit enhancements*  
**Doc id:** 16PR14341 · Lockheed Martin · 15 Nov 1998  

**Related:**  
- HAF `docs/HAF-F16.pdf` = T.O. GR1F16CJ-1 (basic flight manual — **not** full MFD art; defers to 34-1-1)  
- [Blu3wolf/F4-SMS](https://github.com/Blu3wolf/F4-SMS) — SMS sub-pages; each page **sets OSB labels** when entered  

This digest is for **library design** (widgets + page layouts). Not certified flight data.

---

## 1. Hardware (Figure 1-14)

| Item | Fact |
|------|------|
| Glass | **4×4 in** color AMLCD (CMFD) |
| OSB | **20** buttons, numbered **1→20 clockwise from top-left** |
| Rockers | **GAIN** (UL), **SYM** intensity (UR), **BRT** (LL), **CON** (LR) |
| ALS | Ambient light sensor; auto brightness |

OSB numbering (library bezel matches this):

```
        1   2   3   4   5
   20                       6
   19                       7
   18      [  GLASS  ]      8
   17                       9
   16                      10
       15  14  13  12  11
```

---

## 2. Formats available (MLU M1 § CMFD)

Nine formats on the two CMFDs (Section 1 text):

| Mnemonic | Role |
|----------|------|
| **SMS** | Stores Management Set |
| **FCR** | Fire Control Radar |
| **WPN** | Weapon (AGM-65 / 84 / 119 class) |
| **DTE** | Data Transfer Equipment |
| **HSD** | Horizontal Situation Display |
| **TEST** | Fault reporting |
| **Blank** | Empty glass |
| **FLIR** | Navigation pod FLIR |
| **RCCE** | Reconnaissance pod |

Master Menu also shows **TGP, TFR, FLCS, RESET MENU** (Figure 1-15 / 1-16).

---

## 3. Page change rules (critical)

1. **OSB 12, 13, 14** hold the three **display format options** for that CMFD (assigned by DTC or pilot).
2. The **active** format mnemonic is **highlighted**.
3. Press OSB under a **non-active** format option → switch to that format.
4. Press OSB under the **highlighted/active** format → open **Format Selection Master Menu**.
5. If slots are blank / nothing highlighted → press OSB 12 or 13 or 14 **twice** → Master Menu.
6. On Master Menu, pick a format OSB → that format is assigned to the slot and displayed.
7. **No format except BLANK** may appear on more than one of the six slots (both MFDs) at once; reassigning moves it and blanks the old slot.
8. **OSB 15 SWAP** — swap formats between left and right CMFD.
9. **Unlabeled OSBs** (except 12/13/14) have **no function**.
10. **Rotary OSB** — short list of functions: press again to cycle mnemonics. **≥5 options** → dedicated menu page using all 20 OSBs.

**Implication for this library:** every **layout** must supply a full **20-label OSB map**. Labels change when the page (or SMS sub-page) changes. Format select is **not** “always the same top-row bank.”

---

## 4. Color symbology (Table 1-1 + Figures 1-17 / 1-18)

Palette: **red, white, green, yellow, cyan**, plus **black** glass.

Colorization is **primarily for HSD**; uncolored symbols stay **green** (classic mono). Same colored symbols keep their color when shown on FCR.

| Role (MLU) | Color | Examples |
|------------|-------|----------|
| Safety cursors | **Cyan** | FCR cursor, HSD ghost A-A / A-G |
| Ownship / nav data | **White** | Ownship, STPT/IP/TGT, routes, lines, FCR volume, bullseye data, text |
| Tracks / bug | **Yellow** | Radar tracks, bugged target + alts |
| Default / structure | **Green** | Range rings, SOI cue, non-colorized symbology |
| Threat / safety alert | **Red** | (break X / hostile class — use warning role) |
| Glass | **Black** | Background |

FCR figure callouts (1-17): cyan bullseye/cursor; white returns/gain/cursor structure; yellow tracks/bug.  
HSD figure callouts (1-18): cyan bullseye; white ownship/route class; yellow threats/tracks as applicable.

---

## 5. HSD page family (Section 1)

| Page | Access | Notes |
|------|--------|-------|
| **Base** | Format = HSD | DEP/CEN, DCPL/CPL, expand, MSG, CNTL, FRZ, range INC/DEC |
| **Message** | OSB 4 MSG when text pending | Free text; RTN returns |
| **Control** | OSB 5 CNTL | Declutter / status of symbology toggles |
| Expand | OSB 3 / pinky when SOI | NORM → EXP1 (2:1) → EXP2 (4:1) |

Range coupling: CPL ties HSD scale to FCR; DCPL independent. DEP vs CEN ownship placement.

---

## 6. SMS page family (Section 1 + F4-SMS)

MLU figures: INV (NAV mastermode), MDDE, AAM, GUN EEGS, DGFT, A-G, S-J, E-J.

F4-SMS page list (implementation model):  
`OFF, STBY, INV, S-J, E-J, AAM, MSL, DGFT, GUN, A-G, BIT`

Pattern from F4-SMS: **each page class sets OSB labels on enter** (`UpdateOSB(n, text)`). That is the correct library pattern.

Stations **1–9** clockwise from lower left to lower right (wing tips 1/9, center 5). Unloaded = dashed. Hung stores advisory at station.

---

## 7. Library architecture (forced by this manual)

```
widget/*          reusable marks (no format name)
jet::layout       one format = OSB map + widget placements + data
FormatSelect      OSB 12/13/14 slots + Master Menu + SWAP
Palette           Table 1-1 roles (ColorMfd mode)
```

**Do not** hardcode one global OSB bank for all pages.  
**Do** change OSB mnemonics with every page / SMS sub-page.

---

## 8. Demo keys (map to real rules)

| Action | Real jet | Demo |
|--------|----------|------|
| Cycle format on this MFD | DMS L/R or OSB 12–14 | OSB 12/13/14 |
| Open Master Menu | Press active format OSB | Press active slot OSB or `m` |
| Assign format | Menu OSB | Menu OSB |
| SWAP | OSB 15 | OSB 15 (single-MFD: no-op or cycle) |

---

*End of digest. Expand with figure extractions as layouts gain fidelity.*
