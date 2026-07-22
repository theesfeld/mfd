# Drive day — one command

## Bezel (lab keys → production buttons)

See **[`hardware-bezel.md`](hardware-bezel.md)** for production OSB/rocker types.

| Side | Lab keys | Production |
|------|----------|------------|
| Top options | `1`–`5` | OSB 1–5 |
| Right options | `6`–`9` `0` | OSB 6–10 |
| Bottom | `q` `w` `e` `r` `t` | OSB 15–11 (OWN · formats · DCLT) |
| Left | `a` `s` `d` `f` `g` | OSB 16–20 (DTC · SET · BUS) |
| BRT | `[` `]` | BRT rocker |

Format change: press bottom format slots or **`m`** Master Menu (lab). Hardware: active format OSB only.

## Command

```sh
cd ~/Projects/mfd
./cmfd.sh
```

This does a **release build**, then starts the live CMFD glass with Bluetooth OBD and a **comprehensive capture** under `captures/drive-TIMESTAMP/`.

Quit with **Esc**. Capture files finalize on exit.

## What you get on disk

| File | Content |
|------|---------|
| `frames.ndjson` | Every TX/RX hex frame (Mode 01, Mode 09, DTC, UDS) |
| `signals.csv` | Decoded name/value/unit time series |
| `meta.toml` | Session meta (VIN, adapter, times) |
| `session.json` | Session summary + frame count |
| `cmfd-run.txt` | Shell pointer for this run |

After the drive, parse that directory (or hand it to the agent).

## Modes

| Command | Role |
|---------|------|
| `./cmfd.sh` or `./cmfd.sh drive` | Glass + capture (commute default) |
| `./cmfd.sh capture` | Headless crush (DID range scan + long poll) |
| `./cmfd.sh glass` | Glass only, no capture files |
| `./cmfd.sh build` | Release bins only |

Headless long capture example:

```sh
./cmfd.sh capture --seconds 7200
```

## Adapter note

One ELM Bluetooth SPP client at a time. Drive mode runs **capture inside `cmfd`** (`MFD_OBD_CAPTURE` + `MFD_OBD_CRUSH`). Do not start a second `mfd-obd-capture` process against the same adapter while the glass is live.

## Env (defaults for this truck)

| Variable | Default |
|----------|---------|
| `MFD_OBD_BT` | `00:04:3E:96:B8:F1` |
| `MFD_OBD_BT_CHANNEL` | `1` |
| `MFD_OBD_CRUSH` | `1` (drive mode) |
| `MFD_OBD_CAPTURE_FULL` | unset = sample wire frames + change-gated signals (default; keeps long drives light). Set `1` for every TX/RX and every signal sample |
| `MFD_HZ` | `30` |
| `MFD_CAMERA` | optional `/dev/videoN` or `auto` |
| `MFD_SKIP_BUILD=1` | skip cargo when bins are already built |

## Crush coverage

**Drive (glass) process**

1. Capability probe (BIT)
2. Mode 09 VIN + extras
3. Mode 03 / 07 / 0A DTCs
4. Mode 01 PID support discover + continuous poll
5. Multi-module UDS known DIDs (PCM, BCM, ABS, IPC, PSCM)
6. Ford catalog DIDs on continuous rotation
7. Capture → `captures/…` (`frames.ndjson` sampled unless `MFD_OBD_CAPTURE_FULL=1`; `signals.csv` change-gated; buffered flush)

**Headless `./cmfd.sh capture`**

Same as above, plus DID **range scans** (`F400–F4FF`, `1E00–1EFF`, `2B00–2B7F` on select modules). That scan is slow; use when you can leave the truck parked at start.

## Display-only

Never clear DTCs, write DIDs, or security unlock.

## Pages for dense data

| Page | Layout intent |
|------|---------------|
| ENG | Large RPM gauge + ENG channel numerics |
| FUEL | Big % + FP + fuel tape + matrix |
| FLUD | Four temp gauges (OIL/ECT/TFT/IAT) + matrix |
| ELEC | Battery gauge + load tape |
| DRV | Speed gauge + gear + RPM tape + matrix |
| **OWN** | **Bluetooth / bus link** (state, MAC, channel, adapter, protocol, capture) |
| SET | Link block + As-Built feature labels |
| BUS | Link header + full channel dump (2–3 columns) |
| DTC | Fault list (read-only) |

Bottom status strip on every page: `BT LIVE · …MAC…` (or `BT ERR` / `SIM` offline).  
Open **OWN** (`o` / `w`) for the full link block.

**Naming:** `cmfd` is the live product glass. **SIM** means offline synthetic data only (no adapter).

## After work

```sh
ls -la captures/drive-*/
# agent: parse frames.ndjson + signals.csv
```
