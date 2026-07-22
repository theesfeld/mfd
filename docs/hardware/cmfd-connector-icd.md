# CMFD connector ICD

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)  
**Board:** Board B carrier (`hardware/elec/carrier-som/`)

Pin tables are **design baseline** for the fab package. Confirm continuity on first article before harness production.

## Port summary

| Ref | Name | Location | Mating |
|-----|------|----------|--------|
| J3 | USB-C primary | Rear deck | USB-C PD charger / host PC |
| J4 | USB-C aux | Rear deck | Peripherals / second host |
| J5 | Ethernet | Rear deck | RJ45 Cat5e |
| J6 | Multi-IO | Rear deck | 2×5 2.54 mm or pigtail to M12 |
| J7 | Audio | Rear deck | 3.5 mm TRRS or header |
| J8 | Battery | Internal | 18650 tray sense/power |
| J9 | Display FPC | Internal | 40-pin 0.5 mm panel |
| J2 | B2B | Internal | Board A 2×10 |
| J1 (A) | B2B | Board A | mates J2 |

Optional field upgrade: replace header pigtails with **M12** coded bulkheads (A-code signal, D/X Ethernet, L/T power).

## J3 / J4 — USB-C (functional)

| Function | Notes |
|----------|--------|
| VBUS | Charge input (J3) via charger IC |
| CC1/CC2 | PD sink where fitted |
| D+/D− | USB 2.0 device/host per SoM role |
| SBU/SS | Route if SoM supports USB3; else NC |
| Shield | Chassis via RC/ferrite |

**Flash:** SoM recovery and Board A DFU use J3 when in bootloader mode (see `cmfd-flash.md`).

## J5 — Ethernet

Standard magjack magnetics to PHY. 10/100 baseline (RTL8201 class). 1G if SoM + PHY allow.

## J6 — Multi-IO (2×5 header baseline)

| Pin | Signal | Notes |
|-----|--------|-------|
| 1 | +5V_OUT | Fused ≤ 500 mA accessory (optional DNP) |
| 2 | GND | |
| 3 | CAN_H | Isolated side of ISO1042 |
| 4 | CAN_L | Isolated side |
| 5 | CAN_GND | Isolated ground |
| 6 | UART_TX | 3.3 V logic · SoM → external |
| 7 | UART_RX | 3.3 V logic · external → SoM |
| 8 | UART_GND | Same as system GND |
| 9 | nRESET_EXT | Open-drain optional |
| 10 | IO_3V3 | Reference only · do not power bus |

**OBD adapter cable:** J6 CAN_* → J1962 pins 6/14 + chassis ground; **no** power from vehicle required if unit runs on 18650; if vehicle power adapter used, fuse at source. Software remains **read-only**.

## J7 — Audio

| Pin | Signal |
|-----|--------|
| 1 | SPK+ |
| 2 | SPK− |
| 3 | MIC_IN |
| 4 | MIC_GND |

On-board speaker may parallel SPK with series resistor.

## J8 — Battery tray

| Pin | Signal |
|-----|--------|
| 1 | PACK+ |
| 2 | PACK− |
| 3 | NTC |
| 4 | ID / mid (if 2S) |

## J2 / Board A J1 — B2B 2×10

| Pin | Signal | Pin | Signal |
|-----|--------|-----|--------|
| 1 | 3V3 | 2 | 3V3 |
| 3 | GND | 4 | GND |
| 5 | UART_A_TX | 6 | UART_A_RX |
| 7 | I2C_SDA | 8 | I2C_SCL |
| 9 | IRQ_BEZEL | 10 | nRESET_A |
| 11 | BOOT0_A | 12 | SWD_SENSE |
| 13 | 5V (opt) | 14 | GND |
| 15 | ALS_INT | 16 | IMU_INT |
| 17 | PWM_BL | 18 | GND |
| 19 | SPARE | 20 | GND |

## Display J9 — FPC 40

Panel-specific. Pin 1 orientation marked on silk. Populate for chosen MIPI/RGB module; leave DNP until panel SKU frozen on first order.

## Coding recommendation (field M12 kits)

| Use | Code |
|-----|------|
| DC power adapter | M12 **L** or **T** |
| Ethernet umbilical | M12 **X** or **D** |
| CAN / UART | M12 **A** 8-pin |

Do not share a single unkeyed shell for power and data.
