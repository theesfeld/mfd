# Schematic block diagrams

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)  
Logical schematics for the two-board CMFD. Physical nets and fab lands: `fab/` Gerbers + connector ICD.

## Board A — bezel MCU

```
                    ┌─────────────────────────────────────┐
  OSB 1..20 ───────►│  GPIO matrix / expand               │
  rockers 4× ± ────►│  STM32G431 (U1)                     │
  ALS INT ─────────►│    I2C1 ── U3 VEML7700              │
  IMU INT ─────────►│    I2C1 ── U2 BMI270                │
                    │    TIM ─── BL_PWM                   │
                    │    USART── B2B UART to Board B      │
  SWD ─────────────►│    SWDIO/SWCLK                      │
                    │  LDO 3V3 (U4) ◄── 3V3/5V from B2B   │
                    └─────────────────────────────────────┘
```

## Board B — SoM carrier

```
  USB-C J3 ──► BQ25895 charger ──► 18650 pack ──► bucks ──► SoM 5V/3V3
  USB-C J4 ──► SoM USB host/device
  RJ45 J5  ──► ETH PHY ──► SoM RGMII/RMII
  CAN J6   ──► ISO1042 ──► SoM CAN or UART-CAN bridge
  UART J6  ──► level 3V3 ──► SoM UART
  AUD J7   ──► codec/amp ──► speaker / mic
  FPC J9   ──► MIPI/RGB bridge ──► SoM display
  B2B J2   ──► Board A (bezel events, I2C sensors mirror)
  MOD2     ──► BT/Wi-Fi (SDIO/UART)
  U8/U9    ──► BMP280 + GNSS module on I2C/UART
```

## Power tree

```
USB-C VBUS ──► input protect ──► charger ──┬── pack
                                           └── SYS ──┬── 5V buck ── SoM / USB
                                                     ├── 3V3 buck ── logic / Board A
                                                     └── LED driver ── backlight
```

## Display-only CAN

ISO1042 isolated transceiver only. No high-side vehicle actuators, no OBD write FETs.
