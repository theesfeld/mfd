# MFD — project facts (not a second process constitution)

User-global process: `~/.config/agents/AGENTS.md`.

## Product

- **Name:** mfd (multi-function display library)
- **End target:** physical ~**4×4 in color CMFD** (screen + OSB + rockers); see `docs/hardware.md` / Issues #71 · #73
- **Display only:** never write the vehicle (no clear DTC, DID write, security unlock, programming). Read path only.
- **Bezel:** `BezelSource` is the only page input path (keyboard → GPIO later)
- **Sensors:** prefer **vehicle OBD/CAN/UDS** for pitch/roll/heading; on-box gyro/compass only as fallback
- **Low-level draw:** pure asm `libmfd` (`make` → `build/libmfd.a`)
- **Text:** baked atlas `src/font_atlas_data.rs` from B612 Mono; re-bake with `--features bake_font`
- **Live glass:** `./cmfd.sh` or `cargo run --release --bin cmfd` — **vehicle systems only** (jet formats remain in `src/jet/` for widgets reuse, not product path). Offline without OBD = SIM data.

## Commands

```bash
make
cargo test
./cmfd.sh
cargo run --release --bin cmfd
MFD_TERM=kitty cargo run --release --bin cmfd
```

## Layout

- `src/widget/` — softkeys, tape, round gauge, label, bezel
- `src/page.rs` — page compositor
- `src/jet/` — fighter page calls
- `src/auto/` — automotive pages + `VehicleSnapshot`
- `src/obd/` — native ELM/BT/J1979/UDS + Ford DID catalog (no obdtui; display-only)
- `docs/reference/ford-f150-uds-readonly.md` — UDS/CAN read path for truck
- Truck: **2019 SuperCrew 4×4 · 2.7L EcoBoost · APIM Sync 3** · BT ELM `00:04:3E:96:B8:F1`
- As-Built CSVs (2019/Sync3): `docs/reference/ford-f150-forscan/` · profile `docs/vehicle.md`
- `docs/hardware.md` — physical MFD + sensor hierarchy
- `docs/auto-sensors.md` — env and feeds
- `docs/reference/mfd-photo-index.md` — public study index
