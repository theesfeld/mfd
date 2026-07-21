# Changelog

## [Unreleased]

### Fixed

- `vge-demo --fb` release segfault: do not use `dyn Canvas` with the asm FFI path (monomorphize)
- Terminal default for emulators (Ghostty/Kitty/xterm); FB only on real VT or `--fb`
- `poll_quit` ignores non-tty stdin (cargo pipes)

### Added

- True vector engine: geometry lights individual pixels (no sprite/bitmap draw path)
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
