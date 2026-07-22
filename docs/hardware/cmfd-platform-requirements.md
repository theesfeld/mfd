# CMFD platform requirements

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)  
**Status:** design package v1  
**Related:** [`hardware-bezel.md`](../hardware-bezel.md) · [`hardware.md`](../hardware.md)

## Product statement

A **universal** color multi-function display brick. The face uses F-16 CMFD **layout language** (square glass, 20 OSB, four corner rockers). The industrial skin is late-80s consumer / mecha (rounded shell, silver/white/Walkman blue). Applications (vehicle OBD glass, lab instrument, sim, etc.) are **software and cables**, not redesigns of the unit.

This is **not** flight-certified equipment and is **not** an OEM F-16 LRU.

## Hard requirements

| ID | Requirement |
|----|-------------|
| R1 | Square glass aperture ≈ **4.0 in (102 mm)** class |
| R2 | **20** momentary OSB, IDs **1–20** clockwise from top-left |
| R3 | **4** corner rockers: GAIN UL · SYM UR · BRT LL · CON LR |
| R4 | Operator events match `BezelEvent` ABI (`docs/hardware-bezel.md`) |
| R5 | Two-board stack: **Board A** bezel MCU + **Board B** Linux SoM carrier |
| R6 | User-replaceable **18650** tray; **USB-C** in-unit charge |
| R7 | Ports: USB-C, Ethernet, CAN, UART, audio (see connector ICD) |
| R8 | Sensors: ALS, IMU, baro/temp, GNSS, mic, speaker, BT/Wi-Fi |
| R9 | Case primary process: **3D print**, durable for bag toss / short drop |
| R10 | IP54 **design goal** with dust caps |
| R11 | Display-only vehicle policy when OBD app is loaded — **no write actuators** |
| R12 | Purpose-designed PCBs — **not** a bare SBC as the product skin |

## Soft / aesthetic

| ID | Preference |
|----|------------|
| A1 | Outer envelope **spirit** of ~5.6–6 in class face; free to grow for battery/ports |
| A2 | 50/50 CMFD readability + Walkman/Gundam surface language |
| A3 | Optional TPU corner bumpers |
| A4 | First-article BOM target **≤ $300** via lean populate (see `hardware/bom/`) |

## Public reference (inspiration only)

Astronautics F-16 4″ MFD marketing: ~4.2″ glass, ~5.62″ bezel, 524×524 historic resolution. Civil product targets **≥ 720×720** and modern nits when cost allows.

## Deliverables map

| Artifact | Location |
|----------|----------|
| Gerbers + BOM + CPL | `hardware/elec/fab/` |
| Print STLs | `hardware/mech/print/` |
| Exploded view | `hardware/viewer/index.html` · `site/hardware/` |
| Connector ICD | `docs/hardware/cmfd-connector-icd.md` |
| Power | `docs/hardware/cmfd-power.md` |
| Flash | `docs/hardware/cmfd-flash.md` |
