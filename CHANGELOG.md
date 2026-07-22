# Changelog

## [Unreleased]

### Changed

- **CMFD design P1/P2 (Lockheed review):** green SOI box; empty Master Menu glass; round_gauge uses palette structure + white needle; SYM/CON rockers affect glass; ATT sky blue / ground brown; short format ID header; DCLT label fixed. Refs #135.
- **CMFD design P0 (Lockheed review):** support-page return (DTC→ENG via active slot); lab `OsbUp` so SOI does not stick; lab key map off glass unless `MFD_LAB_CHROME=1`; RNG no synthetic; LITE OSBs do not invent lamps. Refs #133.
- **Lab OSB keys (linear):** `1234567890qwertyuiop` = OSB 1–20. `[` `]` = format prev/next. `-` `=` = brightness. MLU: lit `*` slot = Master Menu. Refs #129 #131.

### Fixed

- **Morning capture data path:** multi-line ELM VIN (`N:hex` / CR lines) → real `1FTEW1…`; Mode 01 preferred for glass (RPM/speed/fuel/voltage/temps); dead UDS DIDs demoted; unique `pid_XX` keys (no nibble collision); heavier Mode 01 poll weight. Refs #127.
- **False DOOR AJAR / soft ERR on drive:** door defaults closed when no body DID; soft NO DATA/UDS NRC no longer paint bus ERR or drop LIVE. From capture `drive-20260722-061858`. Refs #125.
- **cmfd owns Bluetooth connect:** BlueZ power/trust/connect/wait + scan + RFCOMM inside the feed; no operator `bluetoothctl connect` for normal use. Shell only notes the target MAC. Refs #123.

### Removed

- **SIM / demo vehicle path:** product glass never invents RPM, speed, DTCs, or lights. Empty snapshot until OBD is LIVE. Default BT MAC when env unset. Warnings only when `bus_state == LIVE`. Refs #121.

### Fixed

- **Bluetooth OBD link robustness:** feed always starts when `MFD_OBD_BT` is set; keeps searching (preferred MAC + OBD-named paired devices, RFCOMM channel scan, BlueZ connect assist); reconnects after link loss; glass shows SEARCH/RECONN. `cmfd.sh` powers BlueZ and pre-connects. Pair once with `bluetoothctl`. Refs #119.
- **Long-run slowdown / host bog:** capture uses BufWriter + sampled continuous frames (default) and change-gated signals; less flush thrash. HashMap keys reused; Kitty base64 encode-into; SIM in-place update; GO page list cached; caps frozen after BIT (no HashSet clone every frame). Demo probe no longer rebuilds `demo_complete` every tick. Full wire log: `MFD_OBD_CAPTURE_FULL=1`. Refs #117.

### Added

- **Hardware bezel SoT:** `docs/hardware-bezel.md` — OSB 1–20 types, rocker types, GPIO ABI, POC key map, production reachability. Rust `mfd::osb_role` constants. Refs #113.
- **Lockheed-class CMFD design:** OSB 12/13/14 format slots + Master Menu (GO formats only), OSB 11 DCLT, OSB 15 OWN; blank-not-repack. Hard formats: **DTC**, **ATT**, **ENG tach**, **DRV speedo+tach**. Default slots ENG·DRV·ATT. Design law: `docs/reference/vehicle-cmfd-design.md`. Refs #103 #105.
- **Product glass `cmfd`:** binary renamed from `mfd-demo` (alias kept). Offline without OBD = **SIM**. Live drive = `./cmfd.sh`. Refs #95.
- **Drive day:** `./cmfd.sh` — release build + live glass + crush OBD/UDS capture under `captures/drive-TIMESTAMP/`. `MFD_OBD_CAPTURE` + `MFD_OBD_CRUSH` on the feed (one BT client). Headless `./cmfd.sh capture` for DID range scans. Dense page layouts (gauges + numerics). **Bluetooth/link block** on OWN + SETUP + BUS + bottom strip (MAC, channel, adapter, protocol, LIVE/ERR). Docs: `docs/drive-day.md`. Refs #93.
- **Warnings:** BINGO aural (low fuel), ALERT aural, red **flash** fields (park brake, …), master caution strip. Speaker via `aplay` PCM. Refs #90.
- **Startup:** real CMFD power-on (black → blank + MLU OSB FCR/HSD/SMS); probe off-glass; adaptive pages omit NOGO options. Refs #88.

### Changed

- **Auto-centric CMFD:** demo is vehicle-only (jet formats stay in lib). Systems pages ENG/FUEL/FLUID/ELEC/DRV/CHAS/BODY/LITE/CLIM/CAM/RNG/ATT/MAP/DTC/BUS/OWN/SET; dense numerics; channel dump. Refs #86.

### Added

- **Vehicle profile:** 2019 SuperCrew 2.7 EB 4×4 · Sync 3; SETUP shows As-Built **feature labels** (read-only). `docs/vehicle.md`. Refs #84.
- **FORScan As-Built export:** **2019-only**; **APIM = Sync 3** (drop non–Sync 3 APIM tabs) + `live_parameters.csv`. Expanded Ford DID catalog. Refs #79.
- **Ford F-150 UDS (read-only):** DID catalog, Mode 0x22 poll/decode, doc `ford-f150-uds-readonly.md`, capture probe, UDS `0x19` allow. Refs #79.
- **VIN / ownship:** `VehicleSnapshot.vin` from Mode 09; chrome `OS ……` + SETUP/OBD full VIN. Refs #77.
- **FAULT CODES** page (`f` / OSB DTC): loads Mode 03/07/0A codes immediately (read-only; no clear). Refs #75.

### Changed

- **Display-only CMFD (safety):** never write vehicle bus; UDS allow-list `0x10`/`0x22`/`0x3E` only; remove write override. Refs #73.
- **Hardware target docs:** physical ~4×4 **color** CMFD + OSB; attitude/heading from **vehicle OBD/CAN/UDS first**; on-box gyro/compass fallback only. `docs/hardware.md`. Refs #71.
- **Native OBD** (`mfd::obd`): Bluetooth SPP + serial ELM, J1979, UDS read path, ISO-TP helpers, capture/replay. Drops path dep on obdtui. Tool: `mfd-obd-capture`. Refs #68.
- **ATT** sphere: limb shading, bank pointer, pitch ladder, **compass marks on the horizon** (N/NE/E…). **MAP** demo scrolls world under heading-up ownship. **Tape gauges** use filled bars (F-16 FUEL language — keep tapes). Refs #67.

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
