# CMFD flash and recovery

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)

## Paths

| Target | Path | Notes |
|--------|------|-------|
| Linux SoM | USB-C **J3** gadget / vendor tool / SD | Primary field update |
| Board A MCU | SWD header (Board A) or DFU via B2B | Factory + recovery |
| Combined | SoM image includes `cmfd-bezeld` that can IAP Board A | Preferred for users |

## First bring-up (lab)

1. Fit 18650 cells or bench 5 V on VBUS test points.  
2. Connect SWD to Board A; load bezel firmware; confirm OSB matrix scan.  
3. Seat SoM; hold recovery button (if fitted) and plug J3; flash OS image.  
4. Confirm UART bezel stream: `OsbDown` / `Knob` events.  
5. Light panel FPC only after rails measure 3V3/5V stable.

## Field update (user)

1. Charge pack above 20 %.  
2. Connect J3 to PC.  
3. Run project update tool or `dd`/vendor flasher per SoM docs.  
4. Reboot; verify version on SET page when software supports it.

## Recovery

| Symptom | Action |
|---------|--------|
| SoM no boot | SD recovery image; reflash eMMC |
| Bezel dead, glass OK | SWD Board A; check B2B seating |
| No charge | Check tray polarity, fuse F1, USB-C cable data/power |
| Panel black | BRT rocker, backlight PWM, FPC seating |

## Security note

Unsigned firmware loads are acceptable for this open project. Do not store vehicle credentials on the unit beyond what the read-only app needs.
