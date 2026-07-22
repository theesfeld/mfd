# Auto sensors — camera, OBD, range

## Camera / FLIR

| Env | Meaning |
|-----|---------|
| `MFD_CAMERA=/dev/video0` | V4L2 device (GREY or YUYV luma) |
| `MFD_CAMERA=auto` | First working `/dev/videoN` |
| `MFD_FLIR_PATH=file.pgm` | Binary PGM still (P5) |

Live frames paint the **FLIR** auto page (green-hot). MJPEG-only webcams need host convert to YUYV/GREY.

## OBD-II

Build with feature `obd` (default). Path dep: `../obdtui/crates/obd-io`.

| Env | Meaning |
|-----|---------|
| `MFD_OBD_PORT=/dev/ttyUSB0` | ELM327/STN serial |
| `MFD_OBD_BAUD=115200` | Baud (optional) |
| `MFD_OBD_REPLAY=path` | Capture replay |

Maps Mode 01 PIDs (RPM, speed, throttle, load, temps, fuel, voltage, MAF) into `VehicleSnapshot`.

## Collision / park range

Auto page **RNG** (right OSB 10). Arcs + meters for F/FL/FR/R.

| Env | Meaning |
|-----|---------|
| `MFD_RANGE=2.1,3.0,2.8,1.2` | front, fl, fr, rear [,rl,rr] meters |

Synthetic ranges animate when unset. Wire ultrasonic/radar/host depth here later.
