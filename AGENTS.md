# MFD — project facts (not a second process constitution)

User-global process: `~/.config/agents/AGENTS.md`.

## Product

- **Name:** mfd (multi-function display library)
- **End target:** physical ~**4×4 in** MFD (screen + OSB buttons + rockers); see `docs/hardware.md` / Issue #71
- **Bezel:** `BezelSource` is the only page input path (keyboard → GPIO later)
- **Sensors:** prefer **vehicle OBD/CAN/UDS** for pitch/roll/heading; on-box gyro/compass only as fallback
- **Low-level draw:** pure asm `libmfd` (`make` → `build/libmfd.a`)
- **Text:** baked atlas `src/font_atlas_data.rs` from B612 Mono; re-bake with `--features bake_font`
- **Demo:** `cargo run --release --bin mfd-demo`

## Commands

```bash
make
cargo test
cargo run --release --bin mfd-demo
MFD_TERM=kitty cargo run --release --bin mfd-demo
```

## Layout

- `src/widget/` — softkeys, tape, round gauge, label, bezel
- `src/page.rs` — page compositor
- `src/jet/` — fighter page calls
- `src/auto/` — automotive pages + `VehicleSnapshot`
- `src/obd/` — native ELM/BT/J1979/UDS (no obdtui)
- `docs/hardware.md` — physical MFD + sensor hierarchy
- `docs/auto-sensors.md` — env and feeds
- `docs/reference/mfd-photo-index.md` — public study index
