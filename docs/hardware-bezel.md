# Hardware bezel — button inputs and types

**Audience:** panel / PCB / GPIO integrators  
**Software SoT:** `src/bezel.rs` (`BezelEvent`, `BezelKnob`, `BezelSource`)  
**Product law:** [`reference/vehicle-cmfd-design.md`](reference/vehicle-cmfd-design.md)  
**Status:** freeze for production panel design

This document defines **what hardware must deliver** so software never needs keyboard shortcuts.

---

## 1. Principle

| Rule | Meaning |
|------|---------|
| **Single input plane** | All operator intent becomes `BezelEvent` |
| **Pages do not know GPIO** | Only `BezelSource::poll()` → events |
| **Unlabeled = no function** | Blank OSB legend → switch may still report press; software ignores |
| **No extra face keys** | No dedicated n/p, color, or unit buttons on the bezel |

POC keyboard maps onto these same events. Production replaces `KeyboardBezel` with a GPIO matrix source.

---

## 2. Geometry (face)

```
              TOP options
           1    2    3    4    5

   20                              6
   19                              7     RIGHT
L  18         [  4×4 GLASS  ]      8     options
E  17                              9
F  16                             10
T
           15   14   13   12   11
              BOTTOM (format select)

OSB numbers: 1 → 20 clockwise from top-left.
```

**Face size:** ~4×4 in color glass (see `hardware.md`).

---

## 3. Discrete inputs — OSB (20)

### 3.1 Electrical type

| Property | Spec |
|----------|------|
| Type | Momentary pushbutton, normally open |
| Sense | Active low or high — host firmware documents polarity |
| Debounce | Hardware RC and/or software ≥ 20 ms |
| Scan | Matrix or 1:1 GPIO; report **down** and **up** edges |
| Id | Integer **1…20** (not zero-based) matching geometry above |

### 3.2 Software events

```text
BezelEvent::OsbDown(osb)   // press edge, osb in 1..=20
BezelEvent::OsbUp(osb)     // release edge
```

Rust types: `mfd::bezel::OsbId` (`u8`), `mfd::bezel::BezelEvent`.

### 3.3 Fixed product roles (when labeled)

| OSB | Side | Frozen product role | Notes |
|-----|------|---------------------|--------|
| **1–5** | Top | **Options for active format** | e.g. Lights: LO HI FOG DRL INT; ENG/DRV: UNIT |
| **6–10** | Right | **Options for active format** | Page-owned; often blank |
| **11** | Bottom | **DCLT** | Declutter level cycle |
| **12** | Bottom | **Format slot C** | Default ATT |
| **13** | Bottom | **Format slot B** | Default DRV |
| **14** | Bottom | **Format slot A** | Default ENG; active → Master Menu |
| **15** | Bottom | **OWN** | Ownship / link identity |
| **16** | Left | **DTC** | Fault codes (support jump) |
| **17** | Left | *(blank)* | Reserved; no permanent function |
| **18** | Left | *(blank)* | Reserved |
| **19** | Left | **SET** | Setup (UNIT, PAL, …) |
| **20** | Left | **BUS** | Channel dump (shop) |

**Bottom left→right on glass:** OSB **15 → 14 → 13 → 12 → 11** = OWN · A · B · C · DCLT.

**Left top→bottom on glass:** OSB **20 → 19 → 18 → 17 → 16** = BUS · SET · blank · blank · DTC.

### 3.4 Format navigation (hardware only)

| Action | OSB path |
|--------|----------|
| Switch among assigned formats | Press non-active slot **12 / 13 / 14** |
| Open Master Menu | Press **active** (highlighted) slot 12/13/14 |
| Assign any GO format | Master Menu → press labeled format OSB |
| Assign into empty slot | Press blank slot twice (within short window) |
| Declutter | OSB **11** |
| Ownship | OSB **15** |
| DTC / SET / BUS | OSB **16 / 19 / 20** |

No other navigation keys exist on production hardware.

---

## 4. Continuous inputs — corner rockers (4)

### 4.1 Electrical type

| Property | Spec |
|----------|------|
| Type | Rocker, rotary encoder, or dual momentary (+/−) |
| Report | Absolute level **0.0 … 1.0** after host scaling, **or** relative steps converted by firmware |
| Rate | Update on change; no need > 30 Hz |

### 4.2 Software events

```text
BezelEvent::Knob(BezelKnob::Brightness, f32)  // 0.0 ..= 1.0
BezelEvent::Knob(BezelKnob::Contrast, f32)
BezelEvent::Knob(BezelKnob::Symbology, f32)
BezelEvent::Knob(BezelKnob::Gain, f32)
```

Rust: `mfd::bezel::BezelKnob`.

### 4.3 Roles

| Knob | Product role | Placement (MLU-class) |
|------|--------------|------------------------|
| **Brightness** | Glass brightness (also ALS bias if present) | Lower-left class |
| **Contrast** | Glass contrast | Lower-right class |
| **Symbology** | Chrome/symbology intensity | Upper-right class |
| **Gain** | CAM/FLIR gain when video GO; else no-op | Upper-left class |

**Do not** map rockers to UNIT, color mode, or format change.

### 4.4 Optional ALS

| Input | Type | Software use |
|-------|------|--------------|
| Ambient light | Analog or digital lux | Bias default brightness; still allow BRT override |

ALS is **not** an OSB and not a `BezelEvent` today — host may adjust brightness before calling the draw path, or future event can be added without changing OSB map.

---

## 5. Input source ABI (software)

```text
trait BezelSource {
    fn poll(&mut self) -> Vec<BezelEvent>;
}
```

| Implementation | Role |
|----------------|------|
| `KeyboardBezel` | Lab / laptop POC |
| `NullBezel` | Headless / tests |
| **Future `GpioBezel` / `HidBezel`** | Production panel |

**GPIO mapping requirement:** firmware emits the same `OsbDown`/`OsbUp`/`Knob` stream. No page code changes.

---

## 6. POC keyboard map (lab only — not production hardware)

| Face side | Keys | OSB |
|-----------|------|-----|
| Top options | `1` `2` `3` `4` `5` | 1–5 |
| Right options | `6` `7` `8` `9` `0` | 6–10 |
| Bottom | `q` `w` `e` `r` `t` | 15, 14, 13, 12, 11 |
| Left | `a` `s` `d` `f` `g` | 16, 17, 18, 19, 20 |
| BRT −/+ | `[` `]` | Knob Brightness |
| CON −/+ | `;` `'` | Knob Contrast |
| SYM −/+ | `-` `=` | Knob Symbology |
| GAIN −/+ | `,` `.` | Knob Gain |

**Lab-only aliases (not on production face):**

| Key | Alias for | Production path |
|-----|-----------|-----------------|
| `n` / `p` / arrows | Format cycle | Slots + Master Menu |
| `m` | Open Master Menu | Press active format slot |
| `c` | Color palette | SET → OSB **PAL/MODE** |
| `u` | Speed unit | SET/DRV → OSB **UNIT** |
| Esc | Quit process | Host power / app lifecycle |

**Acceptance test:** every product function works with **only** OSB 1–20 + 4 rockers (no letter shortcuts).

---

## 7. SET / option functions (OSB-bound prefs)

| Function | Hardware path |
|----------|----------------|
| Speed unit (MPH / KM/H / KT) | Format **SET** or **DRV/ENG**: top OSB **1–2** (**UNIT**) |
| Color palette | Format **SET**: top OSB **3–4** (**PAL** / **MODE**) |
| Brightness | BRT rocker |
| Declutter density | OSB **11** DCLT |

---

## 8. Connector / harness notes (integrator checklist)

| Item | Requirement |
|------|-------------|
| OSB 1–20 | Unique id per position; silkscreen matches OSB number |
| Rockers | Four independent channels + common ground/Vref as needed |
| ESD | Human contact on bezel |
| Glove use | Travel and force suitable for gloves |
| Cable | Enough for panel flex; no dependence on keyboard HID |
| Host | Linux GPIO or MCU → userspace events → `BezelSource` |

---

## 9. What hardware must **not** add

| Do not add | Reason |
|------------|--------|
| Dedicated “next format” / “prev format” buttons | Master Menu + 3 slots |
| Dedicated color or unit buttons | SET options |
| Touch as primary control | OSB muscle memory |
| Buttons that write the vehicle bus | Display-only product |

---

## 10. Related code

| Path | Content |
|------|---------|
| `src/bezel.rs` | `OsbId`, `BezelKnob`, `BezelEvent`, `BezelState`, `BezelSource`, `KeyboardBezel` |
| `src/auto/format_select.rs` | OSB 11–15 format select / Master Menu |
| `src/bin/cmfd.rs` | Consumes `BezelEvent` only for production paths |
| `docs/hardware.md` | System target (face, BT, sensors) |
| `docs/reference/vehicle-cmfd-design.md` | Product / OSB law |

---

## 11. Revision

| Date | Change |
|------|--------|
| 2026-07-22 | Initial freeze for hardware design (#113) |
