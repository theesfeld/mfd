# CMFD studio stills (build-accurate)

**Source of truth for geometry:** Three.js viewer rendered from KiCad + OpenSCAD + BOM  
(`hardware/viewer/index.html`). Photoreal variants are **image edits of those captures**, not freehand AI.

## Accurate captures (from viewer)

| File | Shot |
|------|------|
| `cmfd-accurate-hero.png` | Slight explode, product angle |
| `cmfd-accurate-closed.png` | Closed unit |
| `cmfd-accurate-exploded.png` | Full explode |
| `cmfd-accurate-elec.png` | Electronics only (Board A **frame** + LCD **in hole**) |
| `cmfd-accurate-front.png` | Front view |
| `viewer-proof.png` | Load proof |

## Photoreal polish (edited from accurate frames)

| File | From |
|------|------|
| `cmfd-studio-closed.jpg` | accurate-closed |
| `cmfd-studio-exploded.jpg` | accurate-exploded |
| `cmfd-studio-electronics.jpg` | accurate-elec |

## Stack order (must match build)

```
front bezel + OSB caps
Board A FR4 **frame** (102 mm cutout)
LCD / glass **in the cutout**  ← never under a solid PCB
Board B carrier
18650 tray
rear shell
```

## How to open the viewer

```bash
cd hardware/viewer
python3 -m http.server 8765
# open http://127.0.0.1:8765/
```

Layout is **inlined** so `file://` also works if Three.js CDN is reachable once.
