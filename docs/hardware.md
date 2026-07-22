# Hardware CMFD target

**SoT issues:** hardware [#71](https://github.com/theesfeld/mfd/issues/71) · display-only safety [#73](https://github.com/theesfeld/mfd/issues/73)

This project is not “desktop demo only.” The end product is a **physical color multi-function display (CMFD)** — F-16-class face for vehicle (and jet-style) use.

## Safety — display only (hard rule)

| Rule | Meaning |
|------|---------|
| **Display only** | The CMFD **only shows** data. It does **not** control the vehicle. |
| **No vehicle writes** | Never clear DTCs, write DIDs, security-unlock, program, or command actuators. |
| **Read path only** | OBD Mode 01/09 and UDS **read** services (`0x22`; session `0x10` / keep-alive `0x3E` only as needed to **read**). |
| **No override** | There is no env flag to enable writes. Software rejects write-class SIDs. |

This is intentional and permanent. The unit is safe because it is a **glass**, not a programmer or body controller.

## Device goals

| Element | Target |
|---------|--------|
| Face | ~**4×4 in** **color** glass (MLU **CMFD** class) — software already sizes for physical inches |
| Bezel | **Physical OSB buttons** (20 softkeys) + corner rockers (BRT / CON / SYM / GAIN) |
| Input ABI | `mfd::bezel` (`BezelSource`) — keyboard today; GPIO / matrix later **without page rewrites** |
| Reachability | **All formats + prefs via OSB only** — see [`vehicle-cmfd-design.md`](reference/vehicle-cmfd-design.md) § Hardware I/O freeze. No dedicated n/p/color hardware. |
| Link | On-unit **Bluetooth** (SPP to ELM/STN or phone path) is allowed and expected |
| Host | Embedded Linux (or similar) driving the panel + buttons |

Software rule: **pages draw from `VehicleSnapshot` + bezel events.** Hardware only replaces the feeds and the `BezelSource`.

## Sensor and data hierarchy (attitude / heading)

The truck dash already shows pitch / roll / heading-class data in some modes. That means the signals exist on **vehicle networks**. Prefer those over adding sensors to the MFD box.

| Priority | Source | Role |
|----------|--------|------|
| **1 (preferred)** | **OBD-II / CAN / UDS** from the vehicle | Pitch, roll, heading, speed, powertrain, body — same bus the OEM dash uses |
| **2** | Host fusion of bus + time | Smooth display, hold last good, unit conversion |
| **3 (fallback only)** | On-MFD **gyro / IMU** | If bus has no attitude path or link is down |
| **4 (fallback only)** | On-MFD **magnetometer / compass** | If bus has no heading path |
| Demo | Synthetic sinusoids | Lab without truck or sensors |

**Do not** treat on-box IMU/compass as the primary design. Add chips only when capture proves the vehicle will not give usable rates/angles/heading, or as fail-soft backup.

### Why vehicle-first

- OEM dash already displays angles and heading in a mode → modules already compute and publish them.
- One truth with the instrument cluster (no dual-compass drift vs vehicle).
- MFD hardware stays simpler: screen, buttons, BT/serial to adapter, optional camera.

### What we still need (software)

1. **Deep capture** on the live truck (`mfd-obd-capture --uds`, and later raw CAN if available).
2. Map **DIDs / PIDs / CAN IDs** for pitch, roll, yaw/heading (Ford-class; VIN in `docs/odbii-session`).
3. Extend `VehicleSnapshot` + ATT/MAP pages to consume live bus fields (demo path stays for offline).
4. Keep `BezelSource` for physical button matrix when the panel exists.

Standard Mode 01 alone does **not** carry attitude. Expect UDS `0x22` on body/ABS/IPC modules or proprietary HS/MS-CAN frames — discover via capture, do not invent.

## Optional on-box modules

| Module | When to add |
|--------|-------------|
| Bluetooth classic (SPP) | Default path to ELM327/STN dongle or integrated radio |
| Serial UART | Wired ELM/STN or debug |
| Camera CSI/USB | FLIR / reverse / situational glass |
| IMU / gyro | Fallback if bus attitude missing after capture campaign |
| Magnetometer | Fallback if bus heading missing |
| GPIO matrix | Physical OSB + rockers |

## Related docs

- Sensors env and OBD: [`auto-sensors.md`](auto-sensors.md)
- Ford F-150 UDS read path: [`reference/ford-f150-uds-readonly.md`](reference/ford-f150-uds-readonly.md)
- Bezel ABI: [`API.md`](API.md) § Bezel
- Truck capture sample: [`odbii-session/`](odbii-session/)
