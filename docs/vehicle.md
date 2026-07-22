# Vehicle under test (this CMFD install)

**Issue:** [#84](https://github.com/theesfeld/mfd/issues/84)

| Fact | Value |
|------|--------|
| Model year | **2019** |
| Platform | F-150 **P552** |
| Cab | **SuperCrew** (4-door, full rear seat — “Crew Cab” in casual speech) |
| Driveline | **4×4** |
| Engine | **2.7L EcoBoost** V6 twin-turbo |
| Infotainment | **APIM = Sync 3** |
| Sample VIN | `1FTEW1EP9KFC73499` (from capture; ownship when live Mode 09 works) |
| OBD adapter | Bluetooth ELM327 **`00:04:3E:96:B8:F1`** |
| Policy | **Display only** — never write As-Built or UDS |

### SuperCrew vs Crew Cab

On F-150, the **4-door long rear seat** is **SuperCrew**. SuperCab is the smaller extended cab. Your “4 door, biggest rear seating” = **SuperCrew**.

### FORScan As-Built sheets

`docs/reference/ford-f150-forscan/` is filtered for **2019 + Sync 3**.  
SETUP can show **feature labels** from `Common.csv` as help text only — **not** values written to the truck.

### Live glass priority (all on)

Powertrain · gear/TFT · attitude/heading · TPM · body · fault codes.  
Attitude/heading still need deep UDS capture while the dash shows that mode.

### Data stack

1. **J1979 OBD-II** (Mode 01 / DTC / VIN)  
2. **UDS/CAN** (Mode 0x22 DIDs, session, keep-alive)  
3. **Ford-specific** DID catalog + FORScan As-Built **labels** (2019 · Sync 3)

See `docs/auto-pages.md` for systems page banks.
