# Auto CMFD — pages and data stack

**Product:** vehicle color MFD only. Jet **formats** are not in the demo path.  
**Widgets** (tapes, gauges, lists, attitude, …) remain in `mfd::widget` / `mfd::jet` for later reuse.

## Data stack (all three)

| Layer | What | Source |
|-------|------|--------|
| **1. SAE J1979 OBD-II** | Mode 01 live · 03/07/0A DTC · 09 VIN | Universal; ELM `01xx` |
| **2. UDS / CAN** | ISO 14229 + ISO-TP · `0x22` DID · `0x10`/`0x3E` | HS-CAN (MS later) |
| **3. Ford-specific** | DID catalog · FORScan As-Built **labels** | 2019 SuperCrew · Sync 3 |

The spreadsheet is **Ford As-Built** vocabulary, not a full live PID dump. Live glass still needs Mode 01 + Mode 22.

## Systems grouping (fighter-style banks)

| Bank | Pages | Content |
|------|-------|---------|
| **ENG** | ENG | Large RPM gauge + dense ENG numerics |
| **FUEL** | FUEL | Big fuel % + FP + tape + matrix |
| **FLUID** | FLUID | Four temp gauges (OIL/ECT/TFT/IAT) + matrix |
| **ELEC** | ELEC | Battery gauge + load tape + strip |
| **DRV** | DRV | Speed gauge + gear + RPM tape + matrix |
| **CHAS** | CHAS | TPM, wheel speeds, brake/park |
| **BODY** | BODY | Doors, belts |
| **LITE** | LITE | Exterior/interior lamps |
| **CLIM** | CLIM | Cabin / HVAC |
| **SA** | ATT · MAP | Attitude + heading · schematic map |
| **SENS** | CAM · RNG | Camera/FLIR · park range |
| **BIT** | DTC · BUS | Fault codes · all channels numeric |
| **OWN** | OWN · SET | Ownship VIN/profile · setup |

Same signal may appear on multiple pages (e.g. RPM on ENG and DRV).

## Demo keys

`n`/`p` cycle · number keys and letters jump (see `mfd-demo` banner).  
Default domain: **auto only**.

## Startup (real CMFD power-on)

Glass matches **real F-16 CMFD power-on** (see `docs/reference/cmfd-power-on.md`):

1. **Black** face (power apply)  
2. **BLANK** content + MLU OSB chrome (`SWAP` · `FCR` · `HSD` · `SMS` · `DCLT`)  
3. When vehicle probe finishes → systems pages  

No invented loading splash or GO/NOGO list on the face. Probe runs **off-glass**.

Probe (read-only, background):

1. Link / modules (PCM, BCM, ABS, …)  
2. J1979 PID support  
3. UDS Mode 0x22 DIDs  
4. Comfort options (fog, heated seats, HSWM, TPMS, …)

**Only GO features appear** on systems pages.  
Unavailable pages are omitted from the cycle list.
