# Changelog

## [Unreleased]

### Added

- **WIDG gallery** format: every public widget on one face (`softkey_row`, tapes V/H, gauges, rings, radar, video gate, list, station, caution, progress, horizon, labels). Demo starts here; `g` or left OSB **WIDG**. Refs #50.
- Richer pages: WPN softkeys, HAD rings, FUEL horizontal tape, ECM progress; auto Setup softkeys + caution + BRT strip.

### Fixed

- Ruler face size on Ghostty/Wayland: convert terminal winsize **buffer** pixels to panel **device** pixels before layout (compositor window × scale, or `MFD_PX_SCALE`). Fixes ~2–2.5″ faces when EDID PPI and winsize were mixed. Refs #48.
- **Present crawl** after long runs: square face, 30 Hz default, reusable `PresentScratch`, lower density
- **OSB labels** clipped: reserved margin + short side legends
- **Brightness** knob applies `Surface::apply_brightness`

### Changed

- Demo face is **square** (~4×4 in class); starts on **ENG** (gauges/tapes visible)
- API white paper `docs/API.md` · widget gallery `docs/widgets.md`

### Added

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
