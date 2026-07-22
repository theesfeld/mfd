# CMFD hardware platform

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)  
**Class:** Universal color multi-function display (F-16 CMFD *spirit* + late-80s industrial skin)  
**Not:** flight-certified equipment · not an OEM F-16 LRU · not a single-board computer in a shell

## What you get

| Path | Contents |
|------|----------|
| [`elec/bezel-mcu/`](elec/bezel-mcu/) | Board A — bezel / matrix / sensors MCU |
| [`elec/carrier-som/`](elec/carrier-som/) | Board B — Linux SoM carrier, power, ports |
| [`elec/fab/`](elec/fab/) | **Gerber + drill + BOM + CPL** ready for JLCPCB / PCBWay |
| [`mech/print/`](mech/print/) | **STL** files for 3D print houses |
| [`mech/src/`](mech/src/) | OpenSCAD sources (regenerate STLs) |
| [`bom/`](bom/) | Master BOM (full + lean ≤$300 config) |
| [`viewer/`](viewer/) | Three.js interactive **exploded view** |
| [`../docs/hardware/`](../docs/hardware/) | Requirements, connector ICD, power, flash |

## Quick send-out

### PCB fab (both boards)

1. Zip each folder under `elec/fab/board-a-bezel/` and `elec/fab/board-b-carrier/` (or use the pre-made zips if present).
2. Upload to JLCPCB / PCBWay / your house.
3. Order **2-layer**, 1.6 mm, HASL or ENIG, green or black solder mask.
4. For assembly: upload matching `bom.csv` + `cpl.csv` (LCSC part numbers where filled).

```bash
# regenerate fab outputs after design changes
nix-shell -p python3Packages.numpy --run 'python3 hardware/tools/gen_pcbs.py'
```

### 3D print

Send all `mech/print/*.stl` (or the combined zip). Recommended:

| Part | Material | Notes |
|------|----------|--------|
| `cmfd-front-bezel.stl` | PETG or ABS | Face; 0.2 mm layers |
| `cmfd-rear-shell.stl` | PETG or ABS | Ports + tray |
| `cmfd-battery-tray.stl` | PETG | 18650 retention |
| `cmfd-osb-cap.stl` ×20 | PETG | Print 20 copies |
| `cmfd-rocker.stl` ×4 | PETG | Print 4 copies |
| `cmfd-corner-bumper.stl` ×4 | **TPU 95A** | Drop protection |

```bash
nix-shell -p openscad --run 'bash hardware/tools/export_stls.sh'
```

### Exploded view

Open `hardware/viewer/index.html` in a browser (or the copy under `site/hardware/`).

## Architecture (two board)

```
Face (print)  →  Board A (MCU, OSB matrix, sensors)
                     │ board-to-board
Display glass ←  Board B (SoM, graphics, power, ports, RF)
                     │
                 18650 tray + USB-C charge
```

Operator I/O law (software SoT): [`docs/hardware-bezel.md`](../docs/hardware-bezel.md).

## Safety

Display-only when used as vehicle glass: **no** bus-write actuators on either board. CAN is isolated transceiver only.
