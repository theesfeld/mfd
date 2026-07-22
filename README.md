# MFD — multi-function display library

<!-- agents:status:begin -->
> **Status:** active · `0.1.0-dev.1` · **vehicle CMFD** · format select · drive capture · display-only · [#103](https://github.com/theesfeld/mfd/issues/103) · MIT
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

## Drive day (one command)

```bash
cd ~/Projects/mfd
./cmfd.sh
```

Release build + live vehicle CMFD + **crush OBD/UDS capture** under `captures/drive-TIMESTAMP/`.  
Quit with Esc. After the commute, parse that directory.

| Mode | Command |
|------|---------|
| Drive (default) | `./cmfd.sh` |
| Headless maximal capture | `./cmfd.sh capture --seconds 7200` |
| Glass only | `./cmfd.sh glass` |

Full notes: **[docs/drive-day.md](docs/drive-day.md)**.

## Build and run (live glass)

```bash
cd ~/Projects/mfd
make
./cmfd.sh                 # drive: release + BT + capture
cargo run --release --bin cmfd
# Ghostty/Kitty:
MFD_TERM=kitty cargo run --release --bin cmfd
```

Default: **vehicle CMFD** (ENG bank), **30 Hz**. Without OBD env → offline **SIM** data only.

### Auto pages (default) — jump keys

| Key | Page | Content |
|-----|------|---------|
| `1` | CLUSTER | RPM, speed, gear, throttle |
| `2` | FUEL | Fuel / battery / load **tapes** |
| `3` | TEMPS | Oil, coolant, trans, IAT, MAF, EGT |
| `4` | DRIVE | P/R/N/D/M · 2H/4H/4L |
| `5` | LIGHTS | Beams, fog, brake, turns, cabin |
| `6` | TPM | Four tire pressures |
| `7` | BODY | Doors + seat belts |
| `8` | CLIMATE | Out/in temp, HVAC |
| `9` | **FLIR** | Camera / FLIR glass |
| `0` | **RANGE** | Collision / park arcs |
| `v` | **ATT** | Horizon sphere + heading ° / N–NW |
| `x` | **MAP** | Schematic topo (demo scroll) |
| `f` | **DTC** | All fault codes (read only · Mode 03/07/0A) |
| `o` / `s` | OBD / SETUP | PIDs · config |
| `n` / `p` | Next / previous page | |
| `[` `]` | **BRT** −/+ (real CMFD rocker) | |
| `u` | Speed unit | MPH → KM/H → KT |
| `a` / `j` / Tab | Auto / Jet / toggle | |

OSB (auto): top CLST…LITE · right TPM/BODY/CLIM/FLIR/**RNG** · left OBD/SET/**ATT**/**MAP**.

### Sensors (env)

| Env | Role |
|-----|------|
| `MFD_CAMERA=/dev/video0` or `auto` | Live V4L2 → FLIR |
| `MFD_FLIR_PATH=grey.pgm` | Still greyscale |
| `MFD_OBD_BT` / `MFD_OBD_PORT` / `MFD_OBD_REPLAY` | Live OBD (native stack) → vehicle pages |
| `MFD_OBD_CAPTURE=dir` | Log frames/signals while glass runs (one BT client) |
| `MFD_OBD_CAPTURE_FULL=1` | Log every TX/RX + every signal (heavy; long drives bog the host) |
| `MFD_OBD_CRUSH=1` | Discover all Mode 01 PIDs + multi-module UDS |
| `MFD_RANGE=2.1,3,2.8,1.2` | Range page (m) |
| `MFD_AUTO_PAGE=FLIR` | Start page |

See `docs/auto-sensors.md` and **`docs/hardware.md`** (physical 4×4 + OSB; prefer vehicle bus for pitch/heading).  
Jet: `m` Master Menu · `g` widget QA · OSB 12/13/14.  
MLU M1 CMFD SoT: `docs/reference/mlu-m1-cmfd.md`.

### Bezel knobs (when not using page jump keys)

| Key | Event |
|-----|--------|
| `q w e r t` | Bottom OSB 15–11 |
| `a s d f g` | Left OSB (on jet; on auto `a`=`AUTO`, `s`=`SETUP`) |
| `; '` | Contrast −/+ |
| `- =` | Symbology −/+ |
| `, .` | Gain −/+ |
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
