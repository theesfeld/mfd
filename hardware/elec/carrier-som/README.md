# Board B — SoM carrier

**Fab package:** [`../fab/board-b-carrier/`](../fab/board-b-carrier/)  
**Size:** 120 × 90 mm, 2-layer  
**SoM:** RK3566-class mezzanine (2 GB RAM target)  
**Role:** Linux host, panel FPC, USB-C charge/data, Eth, CAN, UART, audio, battery, RF

## Port map (see also docs/hardware/cmfd-connector-icd.md)

| Ref | Function |
|-----|----------|
| J3 | USB-C primary — PD sink + data + flash |
| J4 | USB-C aux |
| J5 | Ethernet RJ45 |
| J6 | CAN-H/L + UART + GND (isolated CAN) |
| J7 | Audio out / mic |
| J8 | 18650 pack sense/power |
| J9 | Display FPC 40-pin |
| J2 | B2B from Board A |

## Power

USB-C → BQ25895-class charger → 1S/2S 18650 → system buck → SoM 5V/3.3V.  
Optional DC jack pads for vehicle adapter (TVS + fuse).
