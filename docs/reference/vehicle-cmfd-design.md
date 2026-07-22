# Vehicle CMFD design (Lockheed-class)

**Status:** product law for `cmfd`  
**Issue:** [#103](https://github.com/theesfeld/mfd/issues/103)  
**Source:** internal design review (MLU-class CMFD principles + F-150 display-only product)

## Principles

1. **Muscle memory** — stable OSB slots when a function exists.  
2. **Format vs options** — format identity vs controls for that format.  
3. **Unlabeled = no function** — blank, not grey “disabled.”  
4. **Active format highlighted** (OSB 12 / 13 / 14).  
5. **Declutter** — DCLT reduces density; do not paint every PID.  
6. **Power-on honesty** — black → blank chrome → formats when probe ready.  
7. **Fail-soft** — omit or “—”; never invent bus data; display-only.  
8. **Labels are page-owned** — options change with format.

## Taxonomy (fixed categories)

Formats are fixed **systems categories**. Probe gates **presence**, not identity.

| Format | Probe may omit? | Primary glass |
|--------|-----------------|---------------|
| ENG · FUEL · FLUD · ELEC · DRV | Rare if link up | Gauges: RPM, fuel level, speed; sparse numerics |
| CHAS · BODY · LITE · CLIM · CAM · RNG · ATT · MAP | Yes | Only if GO; blank slot if not |
| DTC | Yes only if no link | List (empty = none) |
| BUS · SET · OWN | Product always useful | Dense matrix / identity |

## OSB policy (frozen)

```
        1   2   3   4   5     top — format options
   20                       6
   19                       7     left / right — format options
   18      [  GLASS  ]      8
   17                       9
   16                      10
       15  14  13  12  11     bottom
       OWN sA  sB  sC DCLT
```

| OSB | Legend | Function |
|-----|--------|----------|
| **15** | OWN | Ownship page |
| **14 / 13 / 12** | Format slots A/B/C | Highlight active; other slot → switch; active → Master Menu |
| **11** | DCLT | Declutter 0 → 1 → 2 (full / reduced / gauges-only) |
| **1–10, 16–20** | Format options | Units, lights, BIT jumps (DTC/BUS/SET), page-local |

**Blank-not-repack:** if CHAS is NOGO, the Master Menu omits CHAS; no hollow CHAS format.

## Widgets

| Use gauge / ball / tape | Use dense numeric only |
|-------------------------|-------------------------|
| RPM, speed, fuel level, battery, key temps | Secondary PIDs, DTCs, BUS dump, identity |
| ATT ball + heading | — |
| Tire grid when TPMS GO | Door/belt status grids |

Do not fill the face with tiny gauges for every channel.

## Master Menu

Lists **GO formats only**. Pick assigns into the slot that opened the menu. Duplicate format on another slot → other slot blanks (MLU habit).

## Interim note

Full ring systems bank (ENG on OSB 1 always…) is **not** the end state. Navigation is **three format slots + Master Menu**.

## Anti-patterns (rejected)

- Reshuffle OSB after probe  
- Grey disabled labels  
- Hollow formats for missing equipment  
- Marketing-dense glass  
- Write vehicle bus  
