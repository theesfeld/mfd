# MFD — multi-function display library

<!-- agents:status:begin -->
> **Status:** active · `0.1.0-dev.1` · square ~4×4 in face · baked B612 · OSB bezel API · [#43](https://github.com/theesfeld/mfd/issues/43) · MIT
<!-- agents:status:end -->

## What this is

Composable **instrument pages** for:

- F-16-class **jet MFD formats** (SMS, HSD, TGP, FCR, ENG, FUEL, …)
- **Automotive** cluster / OBD pages reusing the same widgets
- A **plug-in bezel** (20 OSB + knobs) so keyboard POC and real GPIO share one path

**Face geometry:** real F-16 MLU color MFDs are about **4×4 inches (10×10 cm)** square. This library defaults to a **square** framebuffer (512×512), not a full-wide terminal fill.

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

Default demo: **ENG** page (round gauges + tapes), **30 Hz**, square face.  
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

**See tapes/needles:** open demo (starts on **ENG**) or press bottom OSB for **FUEL** / top bank for other formats.

## Jet formats

`BLANK SMS HSD TGP FCR FCR-GM FCR-SEA WPN HAD FLIR DTE TEST ENG FUEL CNI RESET ECM TFR HUD UFC PFL STORES`

## Auto formats

`CLUSTER POWER TEMPS OBD SETUP` + `ObdSnapshot` (normalize OBD PIDs at the edge).

## Color modes

| Mode | Role |
|------|------|
| `GreenMono` | Classic green |
| `ColorMfd` | Color LCD palette |
| `HighVis` | Yellow-dominant |

## Performance notes

- Long crawls were from **huge full-TTY Kitty payloads every frame** + allocs.  
- Demo now: **square ≤512**, **30 Hz**, **PresentScratch** reuse.  
- Raise size: `MFD_MAX_W=640 MFD_MAX_H=640` · rate: `MFD_HZ=60` if the terminal keeps up.

## License

- Code: **MIT**  
- B612 + atlas: **EPL-2.0** — `NOTICE`, `assets/fonts/`
