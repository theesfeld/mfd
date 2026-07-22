# F-16 MFD format and widget catalog

**Purpose:** Public, study-only catalog of Multifunction Display formats and widgets.  
**Sources:** open web, Internet Archive texts, public training materials, open sim documentation.  
**Not** a dump of classified TO page images. Do not treat as certified flight data.

## Public sources (start here)

| Source | Notes |
|--------|--------|
| **`docs/HAF-F16.pdf`** = T.O. **GR1F16CJ-1** | HAF F-16C/D **basic** flight manual. **Does not contain MFD page art.** § Multfunction Display (1-222) only covers video select on the right MFD and defers detail to **T.O. GR1F16CJ-34-1-1** (Avionics and Nonnuclear Weapons Delivery). |
| T.O. **GR1F16CJ-34-1-1** | Detailed MFD / SMS / FCR / TGP book. Often restricted; not in this repo. |
| [DCS F-16C Early Access Guide](https://www.digitalcombatsimulator.com/) (Eagle Dynamics, public PDF) | Full **public** MFD format descriptions + OSB maps (study layout only). Distilled: `docs/reference/dcs-f16-mfd-notes.md` |
| [Chuck’s F-16C Viper Guide](https://chucksguides.com/aircraft/dcs/f-16cm/) | Public training screenshots of FCR/HSD/SMS/TGP |
| [Hoggit MFD wiki](https://wiki.hoggitworld.com/view/MFDs_(Typical)) | Master Menu OSB map |
| [Internet Archive: F-16 flight manual](https://archive.org/search?query=F-16+flight+manual) | Search `1F-16`, `GR1F16`, `34-1-1` |
| USAF / DVIDS / commons cockpit photos | Real chrome / color only — not full format dumps |

**Library implementation is a study approximation** of public training layouts. It is **not** certified flight data and **not** a dump of classified TO figures.

## Bezel layout (library model)

Standard **20 OSB** ring used by this library (F-16-class / OpenMFD convention):

```
        OSB01  OSB02  OSB03  OSB04  OSB05
OSB20                              OSB06
OSB19                              OSB07
OSB18          [ GLASS ]           OSB08
OSB17                              OSB09
OSB16                              OSB10
        OSB15  OSB14  OSB13  OSB12  OSB11
```

**Corner / edge rockers** (plug-in knobs, not OSB):

| Control | Typical function |
|---------|------------------|
| BRT | Display brightness |
| CON | Contrast |
| SYM | Symbology intensity |
| GAIN | Sensor/radar gain (format-dependent) |

Library: `mfd::bezel` — events only. Hardware later implements `BezelSource`.

## Format (page) list — library calls

| Format | Role | Call |
|--------|------|------|
| BLANK | Blank glass + chrome | `jet::blank` |
| SMS | Stores management | `jet::sms` |
| HSD | Horizontal situation | `jet::hsd` |
| TGP | Targeting pod | `jet::tgp` |
| FCR | Fire-control radar | `jet::fcr` |
| FCR-GM | Ground map style | `jet::fcr_gm` |
| WPN | Weapons | `jet::wpn` |
| HAD | HAS / HAD | `jet::had` |
| FLIR | FLIR / IR video + gate | `jet::flir` |
| DTE | Data transfer | `jet::dte` |
| TEST | BIT / test | `jet::test` |
| ENG | Engine | `jet::eng` |
| FUEL | Fuel | `jet::fuel` |
| CNI | Comm/nav/ident summary | `jet::cni` |
| RESET | Reset / status | `jet::reset` |
| ECM | EW / ECM summary | `jet::ecm` |
| TFR | Terrain follow cue | `jet::tfr` |
| HUD | HUD repeater style | `jet::hud_rpt` |
| UFC | UFC/DED-style lines | `jet::ufc` |
| PFL | Pilot fault list | `jet::pfl` |
| FCR-SEA | Sea search style | `jet::fcr_sea` |
| STORES | Stores summary alt | `jet::stores` |

## Widget types — library calls

| Widget | Call | Used by |
|--------|------|---------|
| Softkey / OSB labels | `softkey_row`, `osb_chrome` | All formats |
| Bezel frame | `bezel_frame` | All |
| Tape gauge | `tape_gauge` | FUEL ENG auto |
| Round / arc gauge | `round_gauge` | ENG auto |
| Label / text | `label` | All |
| Range rings | `range_rings` | HSD |
| Bearing pointer | `bearing_pointer` | HSD HUD |
| Track gate | `track_gate` | TGP FLIR |
| Crosshair | `crosshair` | TGP FLIR |
| B-scope grid | `bscope_grid` | FCR |
| List menu | `list_menu` | DTE PFL CNI |
| Station grid | `station_grid` | SMS STORES |
| Numeric readout | `numeric_readout` | Many |
| Caution box | `caution_box` | PFL TEST |
| Horizon cue | `horizon_cue` | HUD TFR |
| Progress strip | `progress_strip` | LOAD BIT |
| Video frame | `video_frame` | TGP FLIR |

## Color modes

| Mode | Use |
|------|-----|
| `ColorMode::GreenMono` | Classic monochrome green |
| `ColorMode::ColorMfd` | Color LCD (green/cyan/amber/red/white/magenta) |
| `ColorMode::HighVis` | High-visibility yellow-dominant |

Real jets may DTC-program colors; library exposes **palettes**, not aircraft DTC files.

## Automotive reuse

Same bezel + widgets:

| Auto page | Reuses |
|-----------|--------|
| CLUSTER | round_gauge, tape, softkeys, readout |
| POWER | tapes, readout |
| TEMPS | tapes |
| OBD | list_menu, readout |
| SETUP | list_menu, OSB |

OSB mapping for auto is defined in `auto::osb_map` — swap `KeyboardBezel` for GPIO later.
