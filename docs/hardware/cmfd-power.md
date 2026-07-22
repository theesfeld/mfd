# CMFD power and battery

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)

## Architecture

```
USB-C VBUS (J3) ──► charger IC (BQ25895 class)
                         │
                         ▼
              18650 pack (user tray) ◄── NTC, protect
                         │
                         ▼
              system buck / LDO rails
                 ├─ 5 V  → SoM, USB
                 ├─ 3.3 V → logic, Board A
                 └─ LED  → backlight (PWM from Board A)
```

Optional vehicle/bench DC (9–14 V) may feed the charger path through TVS + fuse on dedicated pads (DNP on lean BOM).

## Pack

| Item | Spec |
|------|------|
| Format | 18650, user-replaceable tray |
| Default | **2 × 1S parallel** (simple charge) or **1S2P** holder |
| Charge | In-unit via USB-C |
| Protect | UV/OV/short on pack or charger IC |
| Retention | Tray latches; no loose cells in cavity |

## Runtime target

Design goal: **2–4 hours** at moderate backlight on 2×3000 mAh class cells. Measure on first article.

## Safety rules

1. Do not charge unattended on flammable surfaces during bring-up.  
2. Use protected cells or a pack with PCB.  
3. Observe polarity silk on tray.  
4. Ship with cells removed if air-freight rules require it.  
5. No vehicle bus write loads on any power rail.

## Power budget (estimate)

| Rail | Typical | Peak |
|------|---------|------|
| Backlight | 1–3 W | 5 W |
| SoM | 1.5–3 W | 5 W |
| Board A + sensors | 0.2 W | 0.5 W |
| RF | 0.3 W | 1 W |
| **Total** | **~4–7 W** | **~12 W** |
