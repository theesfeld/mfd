# Vehicle CMFD design (Lockheed-class)

**Status:** product law for `cmfd`  
**Issues:** [#103](https://github.com/theesfeld/mfd/issues/103) · [#105](https://github.com/theesfeld/mfd/issues/105)  
**Source:** Lockheed-style design review + operator hard requirements

## Operator hard requirements

| # | Requirement | Implementation |
|---|-------------|----------------|
| 1 | **Dedicated fault code page** | Format **DTC** — always when link ready; empty = honest `NONE`; display-only (no clear) |
| 2 | **Dedicated ATT page** | Format **ATT** — horizon ball + compass/heading; always when link ready |
| 3 | **Tach + speedo gauges** | **ENG** = large RPM tach; **DRV** = large speedo + tach pair |

## Principles

1. **Muscle memory** — stable OSB slots when a function exists.  
2. **Format vs options** — format identity vs controls for that format.  
3. **Unlabeled = no function** — blank, not grey “disabled.”  
4. **Active format highlighted** (OSB 12 / 13 / 14).  
5. **Declutter** — DCLT reduces density; do not paint every PID.  
6. **Power-on honesty** — black → blank chrome → formats when probe ready.  
7. **Fail-soft** — omit or “—”; never invent bus data; display-only.  
8. **Labels are page-owned** — options change with format.

## Format decision table

| Format | Always when link ready? | Probe may omit? | Primary glass |
|--------|-------------------------|-----------------|---------------|
| **ENG** | Yes | No | **Tach (RPM)** gauge + sparse numerics |
| **DRV** | Yes | No | **Speedo + tach** gauges |
| **ATT** | Yes (operator hard) | No | Horizon ball + heading/compass |
| **DTC** | Yes (operator hard) | No (omit only if no link) | Fault list; empty = NONE |
| FUEL | Yes | Rare | Fuel % + level tape |
| FLUD | Yes | Rare | Key temp gauges (ECT/OIL/TFT) |
| ELEC | Yes | Rare | Battery gauge |
| CHAS | No | Yes (TPMS/ABS) | Tire grid when GO |
| BODY · LITE | Default on truck | May tighten later | Status grids |
| CLIM · CAM · RNG · MAP | No | Yes | Only if GO |
| BUS | Yes (shop support) | No | Dense channel dump |
| OWN · SET | Yes | No | Identity / setup |

## OSB policy (frozen)

```
        1   2   3   4   5     top — options for active format
   20                       6
   19                       7     left / right — options
   18      [  GLASS  ]      8
   17                       9
   16                      10
       15  14  13  12  11     bottom
       OWN  A   B   C  DCLT
```

| OSB | Legend | Function |
|-----|--------|----------|
| **15** | OWN | Ownship |
| **14 / 13 / 12** | Format A/B/C | Active lit; other → switch; active → Master Menu |
| **11** | DCLT | Density 0 → 1 → 2 |
| **1–10** | Format options | Units, gear, lights, … (page-owned) |
| **20 / 19 / 16** | BUS · SET · DTC | Support jumps (always labeled when formats exist) |

**Default slots after probe:** ENG · DRV · ATT (when all GO).

**Blank-not-repack:** missing formats omitted from Master Menu; never slide labels to fill gaps.

## Widgets (discipline)

| Use as real widgets | Dense numeric only |
|---------------------|--------------------|
| RPM tach, speedo, fuel level tape | Secondary PIDs |
| Battery gauge, key temps | BUS dump, identity |
| ATT ball + heading | — |
| Tire grid (if TPMS GO) | Door/belt status |

Do not fill the face with tiny gauges for every channel.

## Power-on

1. Black face  
2. Blank content + chrome: `OWN · · · DCLT` (slots empty until probe)  
3. Probe off-glass → seed slots (ENG/DRV/ATT) → systems formats  

No GO/NOGO splash on glass. DTC is a format, not cold-power BIT.

## Master Menu

GO formats only. Pick assigns into the slot that opened the menu. Duplicate on another slot → blank other (MLU habit).

## Anti-patterns (rejected)

- Reshuffle OSB after probe  
- Grey disabled labels  
- Hollow formats for missing equipment  
- Marketing-dense glass  
- Write vehicle bus  
- Permanent car-radio of every category on top/sides  
