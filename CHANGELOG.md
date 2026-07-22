# Changelog

## [Unreleased]

### Added

- **Per-stroke lifespan** on `DisplayList`: `set_lifespan`, `tick`, `TimedStroke`, `stroke_life` / `refresh_life` (fade by remaining life)
- Demo: **needle gauges + tape gauges** with several draw types; optional `VGE_TTL` needle tip trail
- Center mark uses **`vge_xform_rotate` + `vge_line_xf`**

### Changed

- Demo is instrument-oriented (clear → many independent draws → present), not a full HUD scene/sweep path

### Fixed

- `vge-demo --fb` release segfault: do not use `dyn Canvas` with the asm FFI path (monomorphize)
- Terminal default for emulators (Ghostty/Kitty/xterm); FB only on real VT or `--fb`
- `poll_quit` ignores non-tty stdin (cargo pipes)
- **Present path FPS**: buffered half-block/ascii (no per-cell `write!`); Kitty density capped; report draw_us vs present_us
- **Choppy high-FPS**: absolute deadline frame lock, wall-clock animation, default 120 Hz (uncapped floods terminals)
- Remove phosphor trail default (crisp strokes only)
- Transparent overlay: RGBA scanout; present skips alpha=0 so terminal background shows
- `DisplayList::set_width` for 1px…N stroke width

### Changed

- Default demo is **overlay** (cell viewport); text can sit around vectors
- Default display lock **120 Hz** phase-locked; `VGE_HZ=0` for uncapped throughput tests
- Terminal demo default phosphor on (`VGE_PHOSPHOR=0` to disable)

### Added

- **Xiaolin Wu AA lines** in pure asm (`vge_line_aa` / `vge_plot_blend`) — crisp hairlines
- Default hairline strokes and `Surface::line` use AA; `line_fast` keeps Bresenham

### Changed

- **Core is pure assembly `libvge`** (`make` → `.a`/`.so`); Rust is optional FFI only
- Kitty present density raised for sharper on-screen vectors

### Added

- `Makefile`, `examples/c/smoke.c`, full public ABI in `asm/x86_64/vge_extra.s`
- **`DisplayList` / stroke commands** — calligraphic refresh model (1970s vector CRT / HUD style)
- True vector engine: geometry lights individual pixels (no sprite/bitmap draw path)
- **Fast path:** inlined Bresenham stores, bulk clear, RAM back-buffer + single blit present
- `vge_blit` / `Surface::blit_to` / `Framebuffer::present_from`
- `vge_decay` phosphor fade (opt-in smooth trails)
- `frame::FramePacer` for locked target Hz
- `examples/bench` FPS measurement
- `Viewport` / `present_at` terminal overlay API
- Effects: `glow`, `bloom`, `radar_fade`, `scanlines`
- `examples/profile_present` present-backend FPS
- x86_64 GNU assembly hot path: `plot`, `clear`, `line` (Bresenham), `circle`, `rect_fill`, `line_thick`
- Portable C path for other targets (`VGE_FORCE_C=1` forces C on x86_64)
- Affine transform helpers (translate, scale, rotate) + transformed lines
- C ABI (`include/vge.h`) and Rust crate (`vge`)
- RGB24 export for display protocols
- Terminal present path: Kitty graphics, half-block truecolor, ASCII fallback
- **Linux framebuffer present:** `mmap(/dev/fb0)` — assembly stores into video RAM (real TTY)
- `vge-demo --fb` / `VGE_PRESENT=fb` direct screen; `--term` for emulator path

## [0.1.0-dev.1] — 2026-07-21

- Initial development release
