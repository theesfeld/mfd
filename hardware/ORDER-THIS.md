# What to order (you do not need KiCad)

You never have to open KiCad. The files are already built.

---

## 1. Circuit boards (send to a PCB factory)

Go to **[JLCPCB](https://jlcpcb.com/)** or **[PCBWay](https://www.pcbway.com/)**.

### Board A — button face (the bezel)

| | |
|--|--|
| **File** | `hardware/elec/fab/cmfd-board-a-kicad-gerbers.zip` |
| **Layers** | 2 |
| **Thickness** | 1.6 mm |
| **Surface** | HASL (lead-free) is fine |
| **Quantity** | 5 (cheap for first try) |
| **What it is** | Only buttons + pins. No computer chip. |

### Board B — computer / ports (optional second board)

| | |
|--|--|
| **File** | `hardware/elec/fab/cmfd-board-b-kicad-gerbers.zip` |
| Same settings as Board A | |

Board B still needs a small computer module (SoM) plugged in later. For a first test you can **skip Board B** and wire Board A pins to any MCU you already have (Arduino, Pico, STM32 blue pill, etc.).

---

## 2. Plastic case (send to a 3D print shop)

| | |
|--|--|
| **File** | `hardware/mech/print/cmfd-print-files.zip` |
| **Material** | PETG or ABS for the main shell |
| **Bumpers** | print `cmfd-corner-bumper.stl` **four times** in **TPU** (flexible) |
| **Buttons** | print `cmfd-osb-cap.stl` **20 times**, `cmfd-rocker.stl` **4 times** in PETG |

Print shop notes are in `hardware/mech/print/README.md`.

---

## 3. Parts to buy (Amazon / Digi-Key / LCSC)

| Part | How many | What for |
|------|----------|----------|
| 6×6 mm tactile switches (through-hole, ~5 mm tall) | 24 | 20 OSB + 4 rockers |
| 1-pin or single Dupont male headers | ~30 | one pin next to each button |
| Dupont jumper wires (female–female or female–male) | 30+ | button → computer |
| 4″ square color LCD (or closest IPS panel) | 1 | the glass |
| M3 screws + heat-set inserts | handful | hold case together |
| 18650 cells + holder (if portable) | 1–2 | battery later |

Exact LCSC codes are in the board `bom.csv` files when you want factory assembly.

---

## 4. How it connects (plain English)

```
[ 24 buttons on Board A ]
        |
   short copper tracks
        |
[ one metal pin per button + GND pins ]
        |
   Dupont wires (you plug these)
        |
[ any small computer board ]
        |
   USB cable to your PC (for now)
```

Which wire is which: open  
`hardware/elec/bezel-mcu/cmfd-board-a-pinmap.md`  
(it is a table: button → pin name).

Software already knows button presses as “OSB 1–20” and rockers. You only map MCU pin numbers in one config file when the firmware for the physical pins is written.

---

## 5. See the 3D exploded model (spec-accurate)

Open in a browser (needs internet once for Three.js). Geometry matches KiCad Board A/B, BOM parts, and the 148 mm enclosure:

`hardware/viewer/index.html`

Or: `site/hardware/index.html`

Drag to rotate. Slider = explode.

---

## 6. Rebuild files (only if someone changes the design)

```bash
nix-shell -p kicad zip --run 'bash hardware/tools/kicad_export.sh'
bash hardware/tools/export_stls.sh   # needs openscad
```

You do not need this to place an order.

---

## Honesty (short)

- Board A is a **finished switch panel design** with **no copper shorts** on the last automatic check. Leftover warnings are mostly silk text near edges (cosmetic).
- Board B is a **carrier outline + connectors** for the next step (computer module). USB/Ethernet part footprints always look “noisy” to the checker; they are standard library parts.
- This is **not** a flight-certified F-16 unit. It is a purpose-built copy for your projects.

If a factory asks a question you do not understand, paste their email here and it can be answered for you.
