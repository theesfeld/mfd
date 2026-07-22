# 3D print files — CMFD enclosure

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)  
**Source:** `../src/cmfd_enclosure.scad`  
**Regenerate:** `bash hardware/tools/export_stls.sh` (needs OpenSCAD)

## Files to send the print house

| File | Qty | Material | Layer | Infill | Notes |
|------|-----|----------|-------|--------|--------|
| `cmfd-front-bezel.stl` | 1 | PETG or ABS | 0.2 mm | 30–40% | Face up; supports for glass lip if needed |
| `cmfd-rear-shell.stl` | 1 | PETG or ABS | 0.2 mm | 30–40% | Port windows clean |
| `cmfd-battery-tray.stl` | 1 | PETG | 0.2 mm | 40% | Dimensional accuracy for 18650 |
| `cmfd-osb-cap.stl` | **20** | PETG | 0.16 mm | 50% | Hard caps — fidelity priority |
| `cmfd-rocker.stl` | **4** | PETG | 0.16 mm | 50% | |
| `cmfd-corner-bumper.stl` | **4** | **TPU 95A** | 0.2 mm | 15–20% | Drop protection |

Or send `cmfd-print-files.zip` if present.

## Hardware inserts

- 4× M3 heat-set inserts in front/rear bosses  
- Optional 1/4-20 insert in rear mount boss  
- Cover glass: 102×102 mm, 1.0–1.1 mm thick, corners lightly broken  

## Durability notes

Walls are **≥ 3.2 mm** with ribs. TPU bumpers take corner hits. Do not use brittle pure PLA for the shell if the unit will be tossed in a bag.
