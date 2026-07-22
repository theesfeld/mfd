# MFD — multi-function display library

<!-- agents:status:begin -->
> **Status:** active · Version: `0.1.0-dev.1` · Phase 0 rename from VGE · [#33](https://github.com/theesfeld/vge/issues/33) · MIT  
> **Product:** composable aviation MFD pages + automotive reuse · **Font:** B612 Mono · **Glass:** black + fighter ink
<!-- agents:status:end -->

## What this is

**MFD** is a library for building **instrument pages** in a terminal (or later FB):

- **Widgets:** softkeys (OSB), tape gauges, round gauges, labels, bezel  
- **Pages:** many widgets per page  
- **Jet calls:** F-16-class pages (`SMS`, `HSD`, `TGP`, `FCR`, `ENG`, `FUEL`, …)  
- **Auto calls:** cluster / temps / OBD-shaped PIDs reusing the same widgets  
- **Text:** baked **B612 Mono** bitmap atlas (sizes 12 / 16 / 20; EPL — see `NOTICE`)  
- **Draw core:** pure x86_64 assembly **libmfd** (`mfd_line`, `mfd_circle`, …)

This is **not** a transparent toy overlay. Pages clear **black glass** and draw high-contrast symbology.

## Build

```bash
make                 # build/libmfd.a + .so
cargo build --release
cargo run --release --bin mfd-demo
```

Prefer Kitty/Ghostty: `MFD_TERM=kitty cargo run --release --bin mfd-demo`

### Demo keys (bezel / OSB model)

Single keypress (raw stdin). Keys emit **bezel events** — same path real OSB GPIO will use.

| Key | Bezel |
|-----|--------|
| `1`–`5` | Top OSB 1–5 (format / auto page) |
| `6`–`9`, `0` | Right OSB 6–10 |
| `q w e r t` | Bottom OSB 15–11 |
| `a s d f g` | Left OSB 16–20 |
| `[ ]` `; '` `- =` `, .` | BRT / CON / SYM / GAIN knobs |
| `Tab` | Jet ↔ Auto domain |
| `/` | Jet format bank (primary/secondary/tertiary) |
| `c` | Color mode: green mono → color MFD → high-vis |
| `Esc` | Quit |

Catalog: [`docs/reference/f16-mfd-catalog.md`](docs/reference/f16-mfd-catalog.md)

## Library model

```text
Page (black glass)
  ├─ softkey_row / bezel
  ├─ round_gauge / tape_gauge / label  (any mix)
  └─ jet::* or auto::* page call
```

```rust
use mfd::page::Page;
use mfd::jet;
use mfd::Surface;

let mut s = Surface::new(960, 540);
let mut page = Page::new(&mut s);
jet::hsd(&mut page, 180.0, 40.0);
```

### Widgets

| Call | Role |
|------|------|
| `softkey_row` | Bezel button legends (OSB) |
| `tape_gauge` | Vertical/horizontal tape |
| `round_gauge` | Arc + needle (tach / eng) |
| `label` / `label_centered` | B612 text |
| `bezel_frame` | Outer frame |

### Jet pages (`mfd::jet`)

`blank` · `sms` · `hsd` · `tgp` · `fcr` · `eng` · `fuel` · `dte` · `test`

### Auto pages (`mfd::auto`)

`cluster` · `power` · `temps` · `obd_status` · `ObdSnapshot` · `rpm_norm`

## Colors (fighter glass)

| Token | Role |
|-------|------|
| `GREEN` | Primary / normal |
| `GREEN_DIM` | Structure |
| `CYAN` | Nav / geometry |
| `AMBER` / `YELLOW` | Caution |
| `RED` | Warning / redline |
| `WHITE` | Readout |
| `MAGENTA` | Special cue |
| `BLACK` | Glass |

## Research index

Public MFD photo **search list** and page-type catalog:  
[`docs/reference/mfd-photo-index.md`](docs/reference/mfd-photo-index.md)

We do **not** vendor 50 copyrighted image binaries. We do keep a **type catalog** and public URLs for study.

## Env

| Env | Effect |
|-----|--------|
| `MFD_TERM=kitty\|half\|ascii` | Present backend |
| `MFD_MAX_W` / `MFD_MAX_H` | Pixel cap (default 1280×720) |
| `MFD_HZ` | Demo phase lock (default 60) |

## Text / font atlas

Runtime text uses a **compiled coverage atlas** (`src/font_atlas_data.rs`), not TTF.

| API | Role |
|-----|------|
| `FontSize::Sm` / `Md` / `Lg` | Baked faces (~12 / 16 / 20 px) |
| `draw_text_size` / `draw_text` | Draw from atlas |
| `text_width_size` / `text_height_size` | Layout |

Re-bake from source TTF (host only):

```bash
cargo run --release --bin bake-font-atlas --features bake_font
```

## License

- Library code: **MIT**  
- B612 source TTF + atlas derived from it: **EPL-2.0** (PolarSys / Airbus) — see `NOTICE` and `assets/fonts/`
