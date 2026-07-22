# MFD — multi-function display library

<!-- agents:status:begin -->
> **Status:** active · `0.1.0-dev.1` · square ~4×4 in face · baked B612 · OSB bezel API · [#43](https://github.com/theesfeld/mfd/issues/43) · MIT
<!-- agents:status:end -->

## What this is

Composable **instrument pages** for:

- F-16-class **jet MFD formats** (SMS, HSD, TGP, FCR, ENG, FUEL, …)
- **Automotive** cluster / OBD pages reusing the same widgets
- A **plug-in bezel** (20 OSB + knobs) so keyboard POC and real GPIO share one path

**Face geometry:** F-16 MLU color MFD ≈ **4×4 in (10×10 cm)**. The demo sizes the face like a **ruler on your monitor**: `side_px = 4" × display_PPI` (EDID or `MFD_PPI`), then maps a **visually square** cell box so it is not stretched.

**Text:** baked **B612 Mono** atlas (no runtime TTF).  
**Draw core:** pure asm **libmfd** (`mfd_plot` / `mfd_line` / …).

## Docs (start here)

| Doc | Contents |
|-----|----------|
| **[docs/API.md](docs/API.md)** | Full API/ABI white paper |
| **[docs/widgets.md](docs/widgets.md)** | Widget list + ASCII diagrams |
| **[docs/reference/f16-mfd-catalog.md](docs/reference/f16-mfd-catalog.md)** | Formats, OSB map, public manual sources |
| **[docs/reference/mfd-photo-index.md](docs/reference/mfd-photo-index.md)** | Public photo search index |

## Build and demo

```bash
cd ~/Projects/mfd
make
cargo run --release --bin mfd-demo
# Ghostty/Kitty:
MFD_TERM=kitty cargo run --release --bin mfd-demo
```

Default demo: **WIDG** gallery (every public widget), **30 Hz**, square face.  
`[` `]` change **real brightness** (scales RGB after draw).

### Bezel keys (events, not hard-wired pages)

| Key | Event |
|-----|--------|
| `1`–`5` | Top OSB 1–5 |
| `6`–`9` `0` | Right OSB 6–10 |
| `q w e r t` | Bottom OSB 15–11 |
| `a s d f g` | Left OSB 16–20 |
| `[ ]` | Brightness −/+ (**does dim/brighten**) |
| `; '` | Contrast −/+ |
| `- =` | Symbology −/+ |
| `, .` | Gain −/+ |
| `Tab` | Jet ↔ Auto |
| `/` | Jet format bank |
| `c` | Color mode |
| `Esc` | Quit |

## Quick model

```text
BezelSource ──BezelEvent──► BezelState
                              │
                         app loop routes OSB
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
        jet::draw_format               auto::draw_auto
              │                               │
              └──────── widgets + font ───────┘
                              │
                         Surface (square)
                              │
                    apply_brightness(BRT)
                              │
                    term present (PresentScratch)
```

```rust
use mfd::{
    auto, bezel, jet, page::Page, palette::{ColorMode, Palette}, Surface,
};

let mut s = Surface::new(512, 512);
let mut page = Page::new(&mut s);
let pal = Palette::new(ColorMode::ColorMfd);
let st = bezel::BezelState::default();
jet::draw_format(&mut page, jet::Format::Eng, &pal, &st, 0.0);
s.apply_brightness(st.brightness);
```

## Widget list (summary)

Full diagrams: **[docs/widgets.md](docs/widgets.md)**

`bezel_frame` · `osb_chrome` · `content_after_osb` · `softkey_row` · **`tape_gauge`** · **`round_gauge`** · `label` · `range_rings` · `bearing_pointer` · `track_gate` · `crosshair` · `bscope_grid` · `list_menu` · `station_grid` · `numeric_readout` · `caution_box` · `horizon_cue` · `progress_strip` · `video_frame`

**See all widgets:** demo starts on **WIDG** gallery (`g` key or left OSB **WIDG**). Other formats: bottom OSB **FUEL** / **ENG**, top bank for SMS/HSD/…

## Jet formats

`BLANK WIDG SMS HSD TGP FCR FCR-GM FCR-SEA WPN HAD FLIR DTE TEST ENG FUEL CNI RESET ECM TFR HUD UFC PFL STORES`

## Auto formats

`CLUSTER POWER TEMPS OBD SETUP` + `ObdSnapshot` (normalize OBD PIDs at the edge).

## Color modes

| Mode | Role |
|------|------|
| `GreenMono` | Classic green |
| `ColorMfd` | Color LCD palette |
| `HighVis` | Yellow-dominant |

## Face size (ruler-accurate 4×4 on any screen)

Default face is **4.0 inches square** on the **physical monitor** (F-16 MLU class).

```
side_px = MFD_FACE_IN × PPI
viewport cells chosen so on-glass width = height = side
```

| Env | Meaning |
|-----|---------|
| `MFD_FACE_IN=4` | Edge length in **inches** (default **4**) |
| `MFD_PPI=190` | **Force** pixels/inch (use if EDID is wrong — put a ruler on the panel) |
| `MFD_PX_SCALE=0.76` | **Force** device_px per terminal winsize px (use if the face is still wrong) |
| `MFD_MAX_W` / `MFD_MAX_H` | Cap FB (default **1024**) |

PPI detection: `MFD_PPI` → EDID detailed mm → EDID cm → 96 (not accurate).

**Pixel space:** EDID PPI is panel **device** pixels. Ghostty `TIOCGWINSZ` is often **buffer** pixels at a different content scale than the compositor (e.g. content ~2, niri scale 1.5). The layout converts winsize cells to device pixels via the compositor window size when available (`niri` / `hyprctl` / `swaymsg`), or `MFD_PX_SCALE`.

Startup example:
```text
ruler face 4.00" @ 191.2 ppi (EDID-cm)  px×0.763 (compositor)  cell 14.5×32.1dev  → 765×765px  cells 53×24  on-glass 4.00"×4.00"
```
If the terminal is too small: `on-glass` drops and `[clipped to window]` is printed.

## Performance notes

- Long crawls: huge Kitty payloads — demo uses capped square face, **30 Hz**, **PresentScratch**.  
- Rate: `MFD_HZ=60` if the terminal keeps up.

## License

- Code: **MIT**  
- B612 + atlas: **EPL-2.0** — `NOTICE`, `assets/fonts/`
