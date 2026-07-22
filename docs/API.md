# MFD library — API / ABI white paper

**Product:** `mfd` — multi-function display library  
**Version:** 0.1.0-dev.1  
**Audience:** integrators (PC POC, future MCU panel, OBD-II vehicle MFD, sim pit)

## 1. Goals

1. Draw instrument **pages** from composable **widgets**.  
2. Model real MFD **bezel OSB** inputs so hardware plugs in without rewriting pages.  
3. Share the same widgets for **jet formats** and **automotive** pages.  
4. Keep a lean path to on-chip: baked font atlas + pixel strokes; host Rust is the lab shell.  
5. Target a **physical ~4×4 in panel** with buttons; prefer **vehicle bus** for attitude/heading (see `docs/hardware.md`).

## 2. Architecture layers

```
┌──────────────────────────────────────────────────────────┐
│ Application (mfd-demo, future vehicle app)               │
│  domain state · OBD / sim bus · bezel source             │
├──────────────────────────────────────────────────────────┤
│ Page formats  mfd::jet::*  ·  mfd::auto::*               │
│  draw_format / draw_auto                                 │
├──────────────────────────────────────────────────────────┤
│ Widgets  mfd::widget::*                                  │
│  tape · round · OSB chrome · rings · lists · gates …     │
├──────────────────────────────────────────────────────────┤
│ Text  mfd::font  (baked B612 Mono atlas)                 │
│ Color  mfd::palette · mfd::color                         │
│ Bezel  mfd::bezel  (events only)                         │
├──────────────────────────────────────────────────────────┤
│ Surface  mfd::Surface  → FFI libmfd (asm plot/line/…)    │
│ Present  mfd::term  (Kitty / half-block / ascii)  [host] │
└──────────────────────────────────────────────────────────┘
```

### 2.1 Native draw ABI (`include/mfd.h` / `libmfd`)

System V AMD64. Packed color `0xAARRGGBB`.

| Symbol | Role |
|--------|------|
| `mfd_clear` | Fill surface |
| `mfd_plot` | One pixel |
| `mfd_line` / `mfd_line_aa` | Line |
| `mfd_line_thick` | Thick line |
| `mfd_circle` | Circle outline |
| `mfd_rect_fill` | Filled rect |
| `mfd_polyline` | Polyline |
| `mfd_version` | Version string |

Rust `Surface` methods call these. Future MCU: reimplement the same C ABI.

### 2.2 Rust crate surface

```rust
use mfd::{Surface, Page, Palette, ColorMode, jet, auto, bezel};

let mut s = Surface::new(512, 512); // square face
let mut page = Page::new(&mut s);
let pal = Palette::new(ColorMode::ColorMfd);
let bezel = bezel::BezelState::default();
jet::draw_format(&mut page, jet::Format::Eng, &pal, &bezel, t_secs);
s.apply_brightness(bezel.brightness);
```

## 3. Physical face geometry

| Item | Value |
|------|--------|
| F-16 MLU color MFD (Honeywell class) | **≈ 4×4 in (10×10 cm)** square LCD |
| Default face | **4.0 inches** square on the **physical monitor** (`MFD_FACE_IN`) |
| PPI | `MFD_PPI` env, else DRM **EDID** mm + mode, else 96 (panel **device** pixels) |
| Cell size | `TIOCGWINSZ` × `pixel_space()` → device px (`MFD_PX_SCALE` or compositor window × scale) |
| Framebuffer | `side_px = inches × PPI` (1∶1 device px), capped by terminal + `MFD_MAX_*` |
| Terminal box | Cell counts so **on-glass** width ≈ height ≈ face inches |

**Ruler mode (any screen):** `PhysicalFace::layout(backend, 4.0)` sets the 1∶1 framebuffer and the cell viewport from the same device-pixel side. Kitty places by **cells**; cell size must be in the same space as EDID PPI. Ghostty buffer winsize and Wayland fractional scale often disagree — auto-correct via compositor geometry, or set `MFD_PX_SCALE` (device_px / winsize_px). Put a real ruler on the glass — it should read ~4"×4" when not clipped by a small terminal window.

## 4. Bezel input ABI (plug-in hardware)

**Hardware design SoT (button types, roles, harness):** [`hardware-bezel.md`](hardware-bezel.md)

### Events

```rust
pub type OsbId = u8; // 1..=20

pub enum BezelEvent {
    OsbDown(OsbId),
    OsbUp(OsbId),
    Knob(BezelKnob, f32), // 0..=1 absolute
}
pub enum BezelKnob { Brightness, Contrast, Symbology, Gain }

// Frozen production OSB numbers:
// mfd::osb_role::{DCLT, FORMAT_A, FORMAT_B, FORMAT_C, OWN, DTC, SET, BUS}
```

### Source trait

```rust
pub trait BezelSource {
    fn poll(&mut self) -> Vec<BezelEvent>;
}
```

- **POC:** `KeyboardBezel` (maps 1–5 / 6–0 / qwert / asdfg / rocker keys)  
- **Future:** `GpioBezel`, `HidBezel` — **same events only**

Pages **must not** read keyboard or GPIO. They only read `BezelState` / handle events in the app loop.

### OSB numbering (production silkscreen)

```
        1  2  3  4  5     top = options for active format
   20                 6
   19                 7
   18    [ GLASS ]    8
   17                 9
   16                10
       15 14 13 12 11     OWN · fmtA · fmtB · fmtC · DCLT
```

| OSB | Frozen role |
|-----|-------------|
| 1–5 | Format options (e.g. Lights LO/HI/FOG) |
| 6–10 | Format options |
| 11 | DCLT |
| 12–14 | Format slots (default ATT / DRV / ENG) |
| 15 | OWN |
| 16 | DTC |
| 19 | SET |
| 20 | BUS |

### Brightness

`Surface::apply_brightness(factor)` scales RGB after draw. Map `BezelState.brightness` from the **BRT rocker** (POC: `[` `]`). **BRT is not cosmetic text only.**

## 5. Color modes

| Mode | Enum | Use |
|------|------|-----|
| Green mono | `ColorMode::GreenMono` | Classic CRT-style |
| Color MFD | `ColorMode::ColorMfd` | Green/cyan/amber/red/white/magenta |
| High-vis | `ColorMode::HighVis` | Yellow-dominant legends |

`Palette::new(mode)` resolves role colors (`primary`, `nav`, `caution`, …).

## 6. Font atlas

| Face | ~px |
|------|-----|
| `FontSize::Sm` | 12 |
| `FontSize::Md` | 16 |
| `FontSize::Lg` | 20 |

ASCII `0x20`–`0x7E` coverage bitmaps in `font_atlas_data.rs`.  
Re-bake: `cargo run --release --bin bake-font-atlas --features bake_font`

## 7. Widgets (stable calls)

See **[widgets.md](widgets.md)** for diagrams of each.

| Call | Purpose |
|------|---------|
| `bezel_frame` | Outer 1px frame |
| `osb_chrome` | 20 OSB legend labels inside face margin |
| `content_after_osb` | Inner rect after OSB strip |
| `softkey_row` | Single-edge softkey row |
| `tape_gauge` | Vertical/horizontal tape |
| `round_gauge` | Arc + needle |
| `label` / `label_centered` | Baked text |
| `range_rings` | HSD rings |
| `bearing_pointer` | Heading / bearing line |
| `track_gate` | TGP/FLIR gate |
| `crosshair` | Cross |
| `bscope_grid` | Radar grid |
| `list_menu` | Selectable lines |
| `station_grid` | SMS stations |
| `numeric_readout` | Centered value string |
| `caution_box` | Framed caution |
| `horizon_cue` | Bank bar |
| `progress_strip` | 0..1 bar |
| `video_frame` | FOV rectangle |

## 8. Jet formats

`jet::Format` + `jet::draw_format(page, fmt, palette, bezel, t)`.

BLANK · SMS · HSD · TGP · FCR · FCR GM · FCR SEA · WPN · HAD · FLIR · DTE · TEST · ENG · FUEL · CNI · RESET · ECM · TFR · HUD · UFC · PFL · STORES

Catalog + public sources: [f16-mfd-catalog.md](reference/f16-mfd-catalog.md).

## 9. Automotive formats

`auto::AutoPage` + `auto::draw_auto` · `ObdSnapshot` fields are normalized 0..1 (map PIDs at the edge).

CLUSTER · POWER · TEMPS · OBD · SETUP

Same OSB chrome model as jet.

## 10. Present (host only)

| Backend | Env |
|---------|-----|
| Kitty graphics | `MFD_TERM=kitty` |
| Half-block | `MFD_TERM=half` |
| ASCII | `MFD_TERM=ascii` |

Use `PresentScratch` for long runs (avoids alloc/frame → terminal queue lag).  
Default demo rate **30 Hz** (`MFD_HZ`).

## 11. Threading / realtime notes

- Demo is single-threaded.  
- `libmfd` is re-entrant per surface if you own the buffer.  
- Do not flood Kitty with uncapped full-frame presents.  

## 12. License

- Code: MIT  
- B612 source + atlas: EPL-2.0 (see `NOTICE`)
