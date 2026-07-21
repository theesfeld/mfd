# VGE — Vector Graphics Engine

<!-- agents:status:begin -->
> **Status:** active · Phase: [#1](https://github.com/theesfeld/vge/issues/1) · Version: `0.1.0-dev.1` · MIT
<!-- agents:status:end -->

True **vector** graphics: you give geometry (`line`, `circle`, transform). The engine lights **each pixel** on that path. This is not a bitmap or sprite display library.

Hot path on **x86_64 Linux**: GNU assembly (`asm/x86_64/vge.s`). Other targets use a portable C path with the same API.

## What this is

| Does | Does not |
|------|----------|
| Bresenham / midpoint geometry → pixels | Load and show PNG/JPEG sprites as the draw model |
| Rotate / scale / translate then stroke | Fake “vectors” with character cells |
| C ABI + Rust crate | Depend on a GPU frame for the math |

A display layer (window, Kitty protocol, framebuffer) only needs the pixel buffer after VGE draws. The engine itself is pure vector math and pixel stores.

## Install (Rust)

```toml
vge = { git = "https://github.com/theesfeld/vge" }
```

## Demo — every terminal + TTY

Same engine (assembly geometry → pixels). Present path depends on the host:

### Terminal window (default) — Ghostty, Kitty, xterm, …

```bash
cargo run --release --bin vge-demo
```

| `VGE_TERM` | Present |
|------------|---------|
| (auto) | Kitty graphics on Ghostty/Kitty/WezTerm; else half-block truecolor |
| `kitty` | RGB pixels via Kitty protocol |
| `half` | Half-block truecolor (wide TTY/emulator support) |
| `ascii` | Density chars for dumb hosts |

### Direct glass (Linux VT / frame buffer)

```bash
# Real virtual console recommended:
#   Ctrl+Alt+F3 → login →
cargo run --release --bin vge-demo -- --fb
```

Assembly stores into `mmap(/dev/fb0)` video RAM. Needs RW on the FB device.

Quit: `q`, Esc, or Ctrl+C.

## Quick use (Rust)

```rust
use vge::{Surface, Xform, GREEN, BLACK};
use vge::term::{detect_backend, present};

let mut s = Surface::new(640, 480);
s.clear(BLACK);
s.line(10, 10, 600, 400, GREEN);
s.circle(320, 240, 80, GREEN);

let m = Xform::identity()
    .translate(320.0, 240.0)
    .rotate_deg(15.0)
    .translate(-320.0, -240.0);
s.line_xf(&m, 100.0, 240.0, 540.0, 240.0, GREEN);

present(&s, detect_backend()).unwrap();
```

## C API

Header: `include/vge.h`

```c
#include "vge.h"

uint8_t buf[640 * 480 * 4];
VgeSurface s = { .width = 640, .height = 480, .stride = 640 * 4, .pixels = buf };
vge_clear(&s, 0x000000);
vge_line(&s, 0, 0, 639, 479, 0x00FF46);
```

Link the static library from `cargo build` (`libvge.a` / `libvge.so`) or compile `c/vge_portable.c` and, on x86_64, `asm/x86_64/vge.s` with `as --64`.

## Build

```bash
cargo test
cargo build --release
```

Force portable C on x86_64:

```bash
VGE_FORCE_C=1 cargo test
```

Needs: Rust stable, `cc`, GNU `as` (binutils) for the assembly path, `libm`.

## Layout

```
include/vge.h          C ABI
asm/x86_64/vge.s       assembly hot path (plot/line/circle/clear)
c/vge_portable.c       transforms + portable raster + export
src/lib.rs             Rust safe API
src/fb.rs              Linux /dev/fb0 mmap — direct video RAM
src/term.rs            terminal emulator present (Kitty / half / ASCII)
src/bin/vge-demo.rs    live demo (--fb or --term)
```

## SemVer

Version is `0.1.0-dev.1`. **0.x minors may include breaking changes.**

## License

MIT — see `LICENSE`.
