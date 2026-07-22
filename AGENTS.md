# MFD — project facts (not a second process constitution)

User-global process: `~/.config/agents/AGENTS.md`.

## Product

- **Name:** mfd (multi-function display library)
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
- `src/auto/` — automotive reuse + OBD stubs
- `docs/reference/mfd-photo-index.md` — public study index
