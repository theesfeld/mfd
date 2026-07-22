# Changelog

## [Unreleased]

### Added

- **ATTITUDE** page: artificial horizon ball + heading (degrees + N/NNW/…). **MAP** schematic topo. Restore demo **[ ] BRT** (real CMFD rocker); page cycle is **n/p**. Refs #65.
- **Demo defaults to AUTO** vehicle MFD: keys 1–0/o/s, [] page cycle, on-glass feed status (DEMO/OBD/CAM). Refs #62.
- **Live sensors:** V4L2 camera (`MFD_CAMERA`), OBD-II feed via `obd-io` (`MFD_OBD_PORT` / `MFD_OBD_REPLAY`), **RANGE** collision page (`range_display`). Refs #60.
- **Auto vehicle pages:** CLUSTER, FUEL/BAT, TEMPS, DRIVE, LIGHTS, TPM, BODY, CLIMATE, FLIR/CAM, OBD, SETUP. Widgets: status_grid, tire_grid, value_readout; tapes for continuous channels. FLIR via synthetic or `MFD_FLIR_PATH` PGM. Refs #58.
- **MLU M1 visual OCR** of CMFD figures (1-14…1-38, 3-2): widgets, OSB maps, Table 1-1 colors. `docs/reference/mlu-m1-visual-ocr.md`. Ownship cyan. Refs #56.

- **MLU M1 CMFD model** (Pilot’s Guide 16PR14341): `FormatSelect` OSB **12/13/14** + Master Menu, per-page OSB maps, Table 1-1 colors (`Palette.track` yellow, cyan safety, white ownship). Digest `docs/reference/mlu-m1-cmfd.md`. F4-SMS page pattern noted. Refs #54.
- **Real F-16-class MFD formats** from public manuals: Master **MENU**, **FCR** RWS B-scope, **HSD**, **SMS INV**, **TGP**. Demo starts on **FCR**. Refs #52.
- **WIDG** gallery retained for widget QA (`g`). Refs #50.
- Richer pages: WPN, HAD, FUEL, ECM; auto Setup.

### Fixed

- Ruler face size on Ghostty/Wayland: convert terminal winsize **buffer** pixels to panel **device** pixels before layout (compositor window × scale, or `MFD_PX_SCALE`). Fixes ~2–2.5″ faces when EDID PPI and winsize were mixed. Refs #48.
- **Present crawl** after long runs: square face, 30 Hz default, reusable `PresentScratch`, lower density
- **OSB labels** clipped: reserved margin + short side legends
- **Brightness** knob applies `Surface::apply_brightness`

### Changed

- Demo face is **square** (~4×4 in class); starts on **ENG** (gauges/tapes visible)
- API white paper `docs/API.md` · widget gallery `docs/widgets.md`

### Added

- **ATTITUDE** page: artificial horizon ball + heading (degrees + N/NNW/…). **MAP** schematic topo. Restore demo **[ ] BRT** (real CMFD rocker); page cycle is **n/p**. Refs #65.
- **Demo defaults to AUTO** vehicle MFD: keys 1–0/o/s, [] page cycle, on-glass feed status (DEMO/OBD/CAM). Refs #62.
- **Live sensors:** V4L2 camera (`MFD_CAMERA`), OBD-II feed via `obd-io` (`MFD_OBD_PORT` / `MFD_OBD_REPLAY`), **RANGE** collision page (`range_display`). Refs #60.
- **Auto vehicle pages:** CLUSTER, FUEL/BAT, TEMPS, DRIVE, LIGHTS, TPM, BODY, CLIMATE, FLIR/CAM, OBD, SETUP. Widgets: status_grid, tire_grid, value_readout; tapes for continuous channels. FLIR via synthetic or `MFD_FLIR_PATH` PGM. Refs #58.
- **MLU M1 visual OCR** of CMFD figures (1-14…1-38, 3-2): widgets, OSB maps, Table 1-1 colors. `docs/reference/mlu-m1-visual-ocr.md`. Ownship cyan. Refs #56.

- F-16 format catalog + public source notes (`docs/reference/f16-mfd-catalog.md`)
- Widgets: range rings, bearing pointer, track gate, crosshair, B-scope, list, station grid, readout, caution, horizon, progress, video frame, full OSB chrome
- Jet formats: SMS HSD TGP FCR/GM/SEA WPN HAD FLIR DTE TEST ENG FUEL CNI RESET ECM TFR HUD UFC PFL STORES BLANK
- Color modes: `GreenMono`, `ColorMfd`, `HighVis`
- **Bezel/OSB plug-in API** (`BezelSource`, `KeyboardBezel`, knobs) — hardware later without page rewrites
- Auto pages use same bezel + widgets
- **Baked B612 Mono atlas** (sizes 12/16/20) in-library; no runtime TTF/`fontdue`
- `bake-font-atlas` tool (`--features bake_font`) to regenerate atlas data
- `FontSize` + `draw_text_size` API

### Added

- **ATTITUDE** page: artificial horizon ball + heading (degrees + N/NNW/…). **MAP** schematic topo. Restore demo **[ ] BRT** (real CMFD rocker); page cycle is **n/p**. Refs #65.
- **Demo defaults to AUTO** vehicle MFD: keys 1–0/o/s, [] page cycle, on-glass feed status (DEMO/OBD/CAM). Refs #62.
- **Live sensors:** V4L2 camera (`MFD_CAMERA`), OBD-II feed via `obd-io` (`MFD_OBD_PORT` / `MFD_OBD_REPLAY`), **RANGE** collision page (`range_display`). Refs #60.
- **Auto vehicle pages:** CLUSTER, FUEL/BAT, TEMPS, DRIVE, LIGHTS, TPM, BODY, CLIMATE, FLIR/CAM, OBD, SETUP. Widgets: status_grid, tire_grid, value_readout; tapes for continuous channels. FLIR via synthetic or `MFD_FLIR_PATH` PGM. Refs #58.
- **MLU M1 visual OCR** of CMFD figures (1-14…1-38, 3-2): widgets, OSB maps, Table 1-1 colors. `docs/reference/mlu-m1-visual-ocr.md`. Ownship cyan. Refs #56.

- **Product rename:** library and demo are **MFD** (multi-function display)
- **B612 Mono** embedded cockpit font (`fontdue` raster, AA coverage)
- Fighter **color tokens** (green / cyan / amber / red / white / magenta)
- **Widgets:** softkeys, tape, round gauge, label, bezel
- **Page** compositor (multi-widget pages)
- **Jet pages:** SMS, HSD, TGP, FCR, ENG, FUEL, DTE, TEST, BLANK
- **Auto pages:** cluster, power, temps, OBD status + `ObdSnapshot`
- Public MFD **photo/type reference index** (`docs/reference/mfd-photo-index.md`)
- Pure-asm draw library renamed **libmfd** (`mfd_*` ABI)

### Removed

- Public **VGE** product naming (legacy identity)

### Changed

- Demo is multi-page MFD (`mfd-demo`); black glass full screen
