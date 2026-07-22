# Auto sensors — camera, OBD, range

**Hardware product target** (4×4 **color** CMFD, physical OSB, vehicle-first attitude): see [`hardware.md`](hardware.md) and Issue [#71](https://github.com/theesfeld/mfd/issues/71).

## Display only (safety)

The CMFD **only displays** information. It **never** writes the vehicle bus.

- Allowed: Mode 01/09 reads, Mode **03 / 07 / 0A** DTC inventory (read), UDS `0x22` (and `0x10` / `0x3E` only to support reads).
- Forbidden: **clear** DTC (Mode 04), write DID, SecurityAccess (`0x27`), programming, actuators.
- FAULT page (`f` / OSB **DTC**): loads **all** codes as soon as OBD connects; refreshes on poll.
- **No** `MFD_OBD_ALLOW_WRITE` or other write override. See Issue [#73](https://github.com/theesfeld/mfd/issues/73).

## Attitude / heading source order

1. **Vehicle OBD / CAN / UDS** (preferred — dash already shows angles/heading in mode)
2. Host smoothing / last-good
3. On-MFD gyro / IMU (fallback only)
4. On-MFD compass (fallback only)
5. Demo synthetic motion

Mode 01 does not provide pitch/roll/heading. Discover DIDs with `mfd-obd-capture --uds` on the truck.

## Camera / FLIR

| Env | Meaning |
|-----|---------|
| `MFD_CAMERA=/dev/video0` | V4L2 device (GREY or YUYV luma) |
| `MFD_CAMERA=auto` | First working `/dev/videoN` |
| `MFD_FLIR_PATH=file.pgm` | Binary PGM still (P5) |

Live frames paint the **FLIR** auto page (green-hot). MJPEG-only webcams need host convert to YUYV/GREY.

## OBD-II (native `mfd::obd`)

Build with feature `obd` (default). **No dependency on obdtui.** Stack is new in-tree:

- Bluetooth classic SPP (Linux RFCOMM) or serial ELM327/STN
- SAE J1979 Mode 01 PIDs
- UDS **read path only**: session `0x10`, tester present `0x3E`, ReadDataByIdentifier `0x22`
- ISO-TP multi-frame reassembly helpers
- Capture + replay (`frames.ndjson`)
- Write-class UDS is **hard-blocked** (display-only CMFD)

| Env | Meaning |
|-----|---------|
| `MFD_OBD_BT=AA:BB:CC:DD:EE:FF` | Bluetooth SPP MAC (truck example: `00:04:3E:96:B8:F1`) |
| `MFD_OBD_BT_CHANNEL=1` | RFCOMM channel (default 1) |
| `MFD_OBD_PORT=/dev/ttyUSB0` | Serial or bound rfcomm node |
| `MFD_OBD_BAUD=115200` | Serial baud |
| `MFD_OBD_REPLAY=docs/odbii-session` | Capture dir or `frames.ndjson` |

### Capture tool

```sh
# Live truck, Mode 01 + deep UDS probe
cargo run --release --bin mfd-obd-capture -- \
  --bt 00:04:3E:96:B8:F1 --uds --seconds 120 -o ./obd-cap

# Replay known capture
cargo run --release --bin mfd-obd-capture -- \
  --replay docs/odbii-session --seconds 5 -o /tmp/replay-test
```

Creates log files: `frames.ndjson`, `signals.csv`, `meta.toml`, `session.json` (host disk only — not vehicle writes).

### What the truck capture already has

`docs/odbii-session/` (VIN `1FTEW1EP9KFC73499`, ELM327 v1.4b, ISO 15765-4 CAN 11/500):

| Mode | PID | Signal |
|------|-----|--------|
| 01 | 0C | engine RPM |
| 01 | 0D | vehicle speed |
| 01 | 05 | coolant temp |
| 01 | 0F | intake air temp |
| 01 | 11 | throttle |
| 01 | 04 | engine load |
| 03 / 07 | — | sparse DTC (empty in sample) |

**Not in that capture:** UDS `0x22` DIDs, multi-frame raw ISO-TP logs, security session, body/chassis modules, TPM, doors, lights. Run `mfd-obd-capture --uds` on the live adapter to log those.

### What J1979 can show vs needs UDS/OEM

| Page / data | Typical source |
|-------------|----------------|
| RPM, speed, throttle, load, coolant, IAT, MAF, fuel %, voltage, oil temp, ambient | Mode 01 (often available) |
| Gear, 4WD, lights, doors, belts, tire PSI | Body/chassis CAN or OEM UDS — **not** standard Mode 01 |
| VIN | Mode 09 or DID `F190` |
| Module-specific sensors | UDS `0x22` on correct CAN ID (e.g. `7E0` ECM) |

Demo pages for body/lights/TPM stay synthetic until deep capture maps DIDs.

## Collision / park range

Auto page **RNG** (right OSB 10). Arcs + meters for F/FL/FR/R.

| Env | Meaning |
|-----|---------|
| `MFD_RANGE=2.1,3.0,2.8,1.2` | front, fl, fr, rear [,rl,rr] meters |

Synthetic ranges animate when unset.
