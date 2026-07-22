# Widget gallery

Each widget is a **call** you compose on a `Page` / `Surface`.  
Diagrams are schematic (not screenshots). Real F-16 art is proprietary; these show **layout roles**.

---

### `bezel_frame`

```
┌────────────────────┐
│                    │
│                    │
└────────────────────┘
```

Outer 1px frame around the square face.

---

### `osb_chrome` (20 OSB legends)

```
    [SMS] [HSD] [TGP] [FCR] [WPN]
[HAD]                        [DCLT]
[FLIR]      title / glass     [SWAP]
[ECM]                         [CNTL]
[HUD]                         [MODE]
[BLNK]                        [GAIN]
    [DTE] [TEST] [ENG] [FUEL] [CNI]
```

Labels stay **inside** a reserved margin (simulates bezel text on a single FB).

---

### `tape_gauge` (vertical)

```
 FUEL
 ┌──┐
 │──│  ← ticks
 │██│  ← value bar
 │◄─┤  ← index
 └──┘
  62
```

Horizontal variant via `TapeOrientation::Horizontal`.

---

### `round_gauge`

```
      ·  ·  ·
   ·    │red ·
  ·   ──●──   ·   needle + hub
   ·    │    ·
      ·  ·  ·
       RPM
```

Arc ticks, optional redline arc, AA needle.

---

### `range_rings` + `bearing_pointer`

```
    ╭───────╮
  ╭─┤       ├─╮
  │  ╲  ·  ╱  │   rings + heading line
  ╰───╲ │ ╱───╯
       ╲│╱
```

---

### `track_gate` + `crosshair` + `video_frame`

```
 ┌──────────────┐
 │    ┌──┐      │  FOV frame
 │  ──│  │──    │  gate + crosshair
 │    └──┘      │
 └──────────────┘
```

---

### `bscope_grid` (radar)

```
┼───┼───┼───┼
│   │ ● │   │  contact
┼───┼───┼───┼
│   │   │   │
┼───┼───┼───┼
```

---

### `list_menu`

```
▶ MODE  CCRP
  PROFILE  1
  TARGET  TGP
  RELEASE  SGL
```

---

### `station_grid` (SMS)

```
┌────┐┌────┐┌────┐
│ 1  ││ 2  ││ 3  │
└────┘└────┘└────┘
┌────┐┌────┐┌────┐
│ 4  ││ 5  ││ 6  │
└────┘└────┘└────┘
```

---

### `numeric_readout`

```
    HDG 270
```

Centered string (baked font).

---

### `caution_box`

```
┌────────────┐
│  BIT GO    │
└────────────┘
```

---

### `horizon_cue`

```
    ────────  banked bar
      ──●──   wing marks
```

---

### `progress_strip`

```
[████████····]
```

0..1 load / BIT progress.

---

## Jet formats that exercise widgets

| Format | Primary widgets |
|--------|-----------------|
| ENG | round_gauge ×2, tape_gauge ×2 |
| FUEL | tape_gauge ×3 |
| HSD | range_rings, bearing_pointer, readout |
| SMS | station_grid |
| TGP/FLIR | video_frame, track_gate, crosshair |
| FCR* | bscope_grid, readout |
| DTE/CNI/UFC/PFL | list_menu |
| TEST/RESET | caution_box, progress_strip |
| TFR/HUD | horizon_cue, bearing_pointer |
| WPN/HAD/ECM | list_menu |

**Demo default starts on ENG** so tapes and round gauges are visible immediately.
