# Board A — Bezel MCU

**Fab package:** [`../fab/board-a-bezel/`](../fab/board-a-bezel/)  
**Size:** 140 × 140 mm, 2-layer, central **105 × 105 mm** display aperture  
**MCU:** STM32G431 class (LQFP-48)  
**Role:** OSB 1–20 matrix, 4 rockers, ALS, IMU, SWD, B2B to Board B

## Nets (logical)

| Group | Nets |
|-------|------|
| Matrix | OSB1..OSB20 sense lines → MCU GPIO (active low, 10k pull-up) |
| Rockers | GAIN/SYM/BRT/CON UP·COM·DN |
| Power | 3V3, GND from B2B; local LDO if needed |
| Sensors | I2C1 SDA/SCL → BMI270 + VEML7700 |
| Debug | SWDIO, SWCLK, nRST, GND |
| B2B J1 | 3V3, GND, UART_TX, UART_RX, I2C, IRQ, BOOT0, RESET |

## Firmware contract

Emit `BezelEvent` stream over UART/USB-CDC to Board B (see `docs/hardware-bezel.md`).
