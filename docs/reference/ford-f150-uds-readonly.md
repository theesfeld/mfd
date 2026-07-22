# Ford F-150 — UDS on CAN (read-only CMFD)

**SoT issue:** [#79](https://github.com/theesfeld/mfd/issues/79)  
**Safety:** display-only — see Issue [#73](https://github.com/theesfeld/mfd/issues/73) and `docs/hardware.md`.  
**Truck class:** VIN `1FTEW1EP9KFC73499` (capture in `docs/odbii-session/`) — **2019** F-150 P552 · **APIM = Sync 3**; verify DIDs on your build.

This note records the **read-only** UDS/CAN protocol path for the color MFD.  
DIDs and scalings from community / FORScan-style reverse engineering are **hints** until live capture confirms them.

---

## Core protocol

| Layer | Standard | Role |
|-------|----------|------|
| Diagnostic | ISO 14229 (UDS) | Services (session, DID read, DTC) |
| Transport | ISO 15765-2 (ISO-TP) | Single / multi-frame on CAN |
| Physical | ISO 15765-4 | OBD pins, 11-bit or 29-bit IDs |

### Addressing (11-bit, common)

| Role | ID | Notes |
|------|-----|--------|
| Functional / broadcast | `0x7DF` | “Any tester request” |
| PCM request (example) | `0x7E0` | Physical; ELM `ATSH7E0` |
| PCM response | `0x7E8` | Often request + 8 |
| Other ECUs | `0x7E1`–`0x7E7` → resp +8 | BCM/ABS/IPC vary — log to confirm |

Response shape:

- **Positive:** SID + `0x40` (e.g. request `0x22` → `0x62`)
- **Negative:** `0x7F` + request SID + NRC (e.g. `0x31` out of range)

ISO-TP multi-frame: First Frame `0x1N`, Consecutive `0x2x`, Flow Control `0x30`. ELM/STN often hide FC; host still reassembles long hex.

---

## Buses (Ford)

| Bus | Rate | OBD pins | Typical modules |
|-----|------|----------|-----------------|
| **HS-CAN** | 500 kbps | 6 / 14 | PCM, powertrain, many display signals |
| **MS-CAN** | 125 kbps | 3 / 11 | Body / interior — needs adapter MS-CAN support |

Start on **HS-CAN**. Switch to MS-CAN only when body DIDs require it (STN / OBDLink class adapters).

---

## Allowed services (CMFD display-only)

| SID | Name | Use |
|-----|------|-----|
| `0x10` | DiagnosticSessionControl | `10 03` extended **read** session when needed |
| `0x3E` | TesterPresent | Keep-alive: `3E 00` or `3E 80` (suppress) |
| `0x22` | ReadDataByIdentifier | Primary for Ford proprietary data |
| `0x19` | ReadDTCInformation | Optional DTC detail (subfn) — **read** |
| Mode `01`–`0A` | J1979 over ELM | Standard PIDs + DTC inventory (`03`/`07`/`0A`) |

### Forbidden (hard block in `mfd::obd::uds`)

| SID | Name | Why |
|-----|------|-----|
| `0x27` | SecurityAccess | Unlock / risk |
| `0x2E` | WriteDataByIdentifier | Mutation |
| `0x2F` | InputOutputControl | Actuators |
| `0x31` | RoutineControl | Side effects |
| `0x34`+ | Download / programming | Flash |
| Mode `04` | Clear DTCs | Mutation |

No env override.

---

## Session & keep-alive (examples)

```
# Extended diagnostic session (functional 7DF or physical 7E0)
TX  10 03
RX  50 03 …

# Tester present (suppress positive response)
TX  3E 80
```

ELM style:

```
ATSH7E0
1003
3E80
```

---

## Mode 0x22 — primary “extra” data

Request: `22 <DID_hi> <DID_lo>`  
Response: `62 <DID_hi> <DID_low> <data…>`

### PCM examples (test on vehicle; 2015–2020 F-150 community hints)

| Signal | DID | Request | Decode note (verify) |
|--------|-----|---------|----------------------|
| VIN | `F190` | `22 F1 90` | ASCII after SID/DID |
| Coolant (ECT) | `F405` | `22 F4 05` | often `(B0 - 40)` °C |
| Intake air (IAT) | `F40F` | `22 F4 0F` | often `(B0 - 40)` °C |
| Trans fluid (TFT) | `1E1C` | `22 1E 1C` | often 16-bit scaled (e.g. /16) — **log to confirm** |

Other modules (set header first, e.g. BCM):

| Area | Example DID series | Notes |
|------|-------------------|--------|
| Brake / park | `2B00` class | Body / ABS |
| TPMS | ABS / TPMS module DIDs | Prefer capture map |
| Steering / wheel speed | `2813` class / broadcast | May be HS broadcast, not only UDS |

Multi-DID on some ECUs: `22 F190 F405` — support varies; prefer one DID per request for ELM simplicity.

### Full CAN single-frame sketch (11-bit)

```
TX  ID 0x7DF   data  03 22 F4 05   (ISO-TP SF len=3)
RX  ID 0x7E8   data  04 62 F4 05 5A  → 0x5A → 90 °C if (x-40)
```

---

## Standard OBD (always safe baseline)

| Mode | Example | Use |
|------|---------|-----|
| `01 0C` | RPM | Cluster |
| `01 0D` | Speed | Cluster / map |
| `01 05` / `0F` / `11` / `04` | Temps, TPS, load | Temps / tapes |
| `03` / `07` / `0A` | DTCs | FAULT page |
| `09 02` | VIN | Ownship ID |

---

## Mode 0x19 (optional DTC detail)

Example: `19 01 FF` / `19 02 FF` style subfunctions (status mask) — implement only as **read**.  
CMFD already uses Mode `03`/`07`/`0A` for the FAULT list; `0x19` is for richer status later.

---

## Polling strategy (glass)

| Priority | Cadence | Signals |
|----------|---------|---------|
| High | 100–500 ms | RPM, speed, TPS, load |
| Medium | 0.5–2 s | Coolant, IAT, oil, TFT, voltage |
| Low | 5–30 s | Fuel level, TPMS, body states, DID discovery |
| On connect | once + periodic | DTCs, VIN, supported DID probe |

Do not spam; back off on NRC `0x31` / timeout.

---

## Reverse-engineering workflow (safe)

1. Adapter with ISO 15765 + ideally MS-CAN (OBDLink EX/MX+ / STN class).
2. Log HS-CAN while OEM dash shows the value you want (angles, TFT, …).
3. Use FORScan (or similar) on a **laptop** to query known PIDs → capture request/response.
4. Map `22 xx xx` → `62 xx xx <data>`; fit scale against known dash value.
5. Record proven DIDs in `src/obd/ford.rs` + capture files under `docs/odbii-session/` or new `docs/odbii-session-deep/`.

Code entry points:

| Path | Role |
|------|------|
| `mfd::obd::ford` | DID catalog + decode helpers |
| `mfd::obd::uds` | Allow-list + 0x10 / 0x22 / 0x3E / 0x19 |
| `mfd-obd-capture --uds` | Live probe log |
| `docs/odbii-session/` | Existing Mode 01 capture |

---

## What this does **not** claim

- Complete FORScan DID database for every F-150 option package.
- Confirmed scaling for every table row without a successful live response on **your** truck.
- That attitude / heading DIDs are known yet — still to discover via deep capture while dash is in that mode.

---

## FORScan spreadsheet (what we actually got)

The public Google Sheet  
[1uDSQ1Z5a2Wt8-kjrSiVSlDFGFHnfeuhb3RTMVz95730](https://docs.google.com/spreadsheets/d/1uDSQ1Z5a2Wt8-kjrSiVSlDFGFHnfeuhb3RTMVz95730/edit)  
is a **FORScan As-Built / feature-config** workbook (module addresses like `726-48-02`), **not** a full Mode `0x22` live-data dump.

Exported under **`docs/reference/ford-f150-forscan/`** (**2019-only** filter):

| File | Contents |
|------|----------|
| `INDEX.md` | Kept tabs for **2019** F-150 (32 sheets; 2015–17 / -old dropped) |
| `NN_*.csv` | Per-module As-Built sheets in 2019 scope |
| `modules_index.csv` / `modules_can.csv` | Module ↔ CAN hint |
| `asbuilt_address_prefixes.csv` | Address prefixes from kept sheets only |
| **`live_parameters.csv`** | Live glass parameters (Mode 01 + Mode 22) for the CMFD |

**No public raw FORScan live-data dump** for the exact 2019 Crew Cab 2.7L XLT was found online (logs are usually private / VIN-bound).  
To get “everything” for **your** truck: run `mfd-obd-capture --uds` and optional DID range scan on the live adapter.

### FORScan protocol (for context)

FORScan uses Ford HS-CAN / MS-CAN + UDS (ISO 14229 / ISO 15765), mainly Mode **0x22**, plus session / keep-alive.  
It also supports **As-Built write** and programming — **mfd never does that**.

## Related

- `docs/hardware.md` — vehicle-first sensors, display-only product
- `docs/auto-sensors.md` — env and feed
- `docs/reference/ford-f150-forscan/` — spreadsheet export + live parameter table
- Issue #68 native OBD · #73 display-only · #75 DTC page · #77 VIN · #79 Ford UDS
