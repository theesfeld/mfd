# VGE — pure assembly vector engine

<!-- agents:status:begin -->
> **Status:** active · Version: `0.1.0-dev.1` · **libvge = ASM only** · first use: needle + tape gauges · lifespan API · [#25](https://github.com/theesfeld/vge/issues/25) · MIT
<!-- agents:status:end -->

## This is assembly

**The library is only `asm/x86_64/*.s`.**  
No C in the library. No Rust in the library. No libc. No libm.

```bash
make && make test     # pure-asm smoke (examples/asm/smoke.s)
make install          # libvge.a / libvge.so + vge.h
```

```
include/vge.h          calling convention (System V AMD64)
asm/x86_64/vge.s       plot clear line circle
asm/x86_64/vge_extra.s thick rect blit decay export xform polyline …
build/libvge.a         static
build/libvge.so        shared
```

Any language: load `libvge`, call the symbols in `vge.h`.

### Demo (Rust loads libvge only)

```bash
make                    # build/libvge.a
cargo run --release --bin vge-demo
```

The demo **links pure-asm libvge** and calls several draw functions in one frame
(needle gauges, tape gauges, a rotated mark). It does not reimplement the engine.

**Model:** call `vge_line` / `vge_circle` / … into a pixel surface, then present.
That surface is only scanout for the terminal or FB. The product is the draw API,
not a game framebuffer manager.

**First use:** instrument needles and tape gauges (not heavy scene effects).


## Performance (read this first)

| Stage | Role | Measured rate (release, this class of host) |
|-------|------|-----------------------------------------------|
| **Raster** | Geometry → pixels in **system RAM** | Draw-only: **~10 000 FPS** at 1280×720; **~1 700 FPS** at 2560×1600 |
| **Present** | Put pixels on glass or in a terminal | FB blit full HD: **~800 FPS**. Kitty present (capped): **thousands of FPS** at default density |
| **Pace** | **Even frame times** (absolute phase lock) | Default demo locks **120 Hz**. Uncapped max-FPS often looks choppier |

**Smooth ≠ maximum FPS.**  
Flooding Ghostty/Kitty with uncapped full-frame presents queues work in the emulator and motion stutters.  
VGE locks the **display** to a fixed period (`t0 + n·Δt`) and drives animation from **wall-clock time**, so rotation stays continuous even if a frame overruns.

**Fact:** The raster is not the bottleneck. Present bandwidth and **frame-time jitter** are.

```bash
cargo run --release --example bench
cargo run --release --example profile_present   # present FPS by backend
```

| Present backend | Size (80×24 cells) | Present rate (order of) |
|-----------------|--------------------|-------------------------|
| ASCII | 80×24 | >100 000 FPS |
| Half-block | 80×48 | >50 000 FPS |
| Kitty | 320×192 (capped density) | >3 000 FPS |

---

## Overlay model (vectors on top of text)

VGE can fill a **cell rectangle** only. Text and inputs stay around that region.

```text
┌ status / keys ─────────────────────────────┐
│  [  vector viewport — Kitty or half-block ] │
│  [  present_at(surface, backend, viewport) ]│
└ draw_us / present_us / fps ────────────────┘
```

```rust
use vge::term::{detect_backend, enter_overlay, leave_overlay, present_at, Viewport,
                surface_size_for_viewport};
use vge::{Surface, BLACK, GREEN};

let backend = detect_backend();
let vp = Viewport::centered_frac(0.7, 0.65); // cells
let (w, h) = surface_size_for_viewport(backend, vp);
let mut s = Surface::new(w, h);
s.clear(BLACK);
s.line(0, 0, w as i32 - 1, h as i32 - 1, GREEN);

enter_overlay()?;
present_at(&s, backend, vp)?;   // does not wipe the whole TTY chrome
// … print text at other cell positions …
leave_overlay()?;
```

| API | Purpose |
|-----|---------|
| `Viewport { col, row, cols, rows }` | Cell box (0-based origin) |
| `Viewport::centered_frac(fw, fh)` | Centered box as a fraction of the terminal |
| `present_at(surface, backend, vp)` | Place pixels in that box only |
| `enter_overlay` / `leave_overlay` | Hide cursor; keep main screen |
| `enter_fullscreen` / `leave_fullscreen` | Alternate screen (optional) |

---

## Install

### Primary (assembly library)

```bash
git clone https://github.com/theesfeld/vge
cd vge && make && make install
# CFLAGS: -I$HOME/.local/include
# LDFLAGS: -L$HOME/.local/lib -lvge -lm
```

### Optional Rust bindings (FFI only)

```toml
vge = { git = "https://github.com/theesfeld/vge" }
```

The Rust package **links** the assembly objects; it does not reimplement the engine.

---

## Quick start (Rust) — stroke list

```rust
use vge::{DisplayList, Surface, GREEN, CYAN};

let mut list = DisplayList::new();
list.set_width(1); // hairline; any integer ≥ 1
list.set_color(GREEN);
list.line(10, 10, 630, 350);
list.set_color(CYAN);
list.circle(320, 180, 80);

let mut scanout = Surface::new(640, 360);
// Transparent clear + crisp strokes only (no black fill, no trail)
list.refresh(&mut scanout);
// present_at_state: paints opaque pixels only over the terminal
```

Immediate beam (no list) still works:

```rust
use vge::{Surface, GREEN, BLACK};
let mut s = Surface::new(640, 360);
s.clear(BLACK);
s.line(10, 10, 630, 350, GREEN);
```

Linux frame buffer (direct glass):

```rust
// Draw in RAM, blit once per frame (do not plot into FB per pixel).
let mut fb = vge::fb::Framebuffer::open_default()?;
let mut back = Surface::new(fb.width(), fb.height());
// … draw into `back` …
fb.present_from(&back);
```

---

## Demo

```bash
# Default: needle gauges + tape gauges + rotated mark (120 Hz)
cargo run --release --bin vge-demo

# Optional needle tip trail (library lifespan)
VGE_TTL=10 cargo run --release --bin vge-demo

# 60 Hz
VGE_HZ=60 cargo run --release --bin vge-demo

# Linux video RAM path
cargo run --release --bin vge-demo -- --fb
```

| Flag / env | Effect |
|------------|--------|
| (default) | Overlay; clear once, then draw gauges (several `vge_*` calls) |
| `VGE_TTL=N` | Optional RPM needle tip trail (`N` frames, fade) |
| `--fb` | RAM draw + blit to `/dev/fb0` |
| `VGE_HZ=120` | Default: phase-lock 120 Hz |
| `VGE_HZ=60` | Phase-lock 60 Hz |
| `VGE_HZ=0` | Uncapped (throughput test) |
| `VGE_WIDTH=N` | Stroke width in pixels (default 1) |
| `VGE_TERM=kitty\|half\|ascii` | Force present backend |
| `VGE_MAX_W` / `VGE_MAX_H` | Cap pixel buffer (default 960×540) |

**Draw types in the demo:** `circle`, `line_aa`, `line_thick`, `line_fast`,
`rect_fill`, `polyline`, `line_xf` + `xform_rotate`.

Quit: `q`, Esc, or Ctrl+C.

---

## API surface

### Stroke display list (calligraphic core)

| Type / method | Description |
|---------------|-------------|
| `DisplayList` | Refresh memory for beam commands |
| `TimedStroke` | Command + `born` + `ttl` (0 = immortal) |
| `set_color` / `move_to` / `line_to` | Beam state + strokes |
| `line` / `line_thick` / `circle` / `polyline` | Draw commands |
| `set_width(px)` | Stroke width in pixels (≥ 1) |
| `set_lifespan(frames)` | Default TTL for new commands (`0` = immortal) |
| `tick()` | Advance frame clock; drop expired strokes |
| `stroke(surface)` | Draw living commands (full opacity) |
| `stroke_life(surface, fade)` | Draw living commands; optional alpha by remaining life |
| `sweep(surface, prev)` | Erase previous path, then stroke (sparse motion) |
| `refresh(surface)` | Transparent clear + stroke living (full opacity) |
| `refresh_life(surface, fade)` | Transparent clear + stroke with optional trail fade |

**Update models:** full-scene rebuild → `refresh` (1× stroke). Few vectors move → `sweep`. CRT/radar trails → `set_lifespan` + `tick` + `refresh_life` / `stroke_life`.

### Geometry scanout (C + Rust)

| Function | Description |
|----------|-------------|
| `vge_clear` / `Surface::clear` | Fill all pixels |
| `vge_plot` / `plot` | One pixel |
| `vge_line` / `line` | Bresenham (inlined stores in asm) |
| `vge_line_thick` / `line_thick` | Multi-pass thick line |
| `vge_circle` / `circle` | Midpoint circle |
| `vge_rect_fill` / `rect_fill` | Filled rectangle |
| `vge_line_xf` / `line_xf` | Line after affine transform |
| `vge_polyline` / `polyline` | Connected segments |
| `vge_xform_*` / `Xform` | Translate, scale, rotate |

### Present / buffer

| Function | Description |
|----------|-------------|
| `vge_blit` / `blit_to` / `present_from` | Copy RAM → RAM or RAM → FB |
| `vge_decay` / `decay` | Phosphor fade (`factor_256` / 256) |
| `vge_export_rgb24` | Tight RGB for protocols |
| `term::present` / `present_at` | Terminal present |
| `frame::FramePacer` | Optional target Hz |

### Effects (`vge::effects`)

| Function | Description |
|----------|-------------|
| `glow` | Expand bright pixels with falloff |
| `bloom` | Threshold + box blur add-back |
| `radar_fade` | Angular sector fade (radar beam) |
| `scanlines` | Dim every other row |

Effects run **after** geometry. They cost CPU. Leave them off for maximum rate.

---

## C API

```c
#include "vge.h"

uint8_t buf[640 * 360 * 4];
VgeSurface s = { .width = 640, .height = 360, .stride = 640 * 4, .pixels = buf };
vge_clear(&s, 0x000000);
vge_line(&s, 0, 0, 639, 359, 0x00FF46);
```

---

## Layout

```
include/vge.h              public C ABI
asm/x86_64/vge.s           plot/clear/line/circle
asm/x86_64/vge_extra.s     thick/rect/blit/decay/export/xform/…
Makefile                   builds libvge.a / libvge.so
examples/c/smoke.c         pure-C link test
c/vge_portable.c           reference only (non-x86_64 / VGE_FORCE_C)
src/                       optional Rust FFI + demos (not the engine)
```

## Architecture note

This is intentional innovation on a known foundation: **refresh vector / calligraphic** display lists (see vector CRT / aircraft HUD stroke generators).  
VGE reimplements that control path in software with a modern present stage (terminal graphics protocol or Linux frame buffer). The list is the picture. Pixels are only the scanout of the beam.

---

## SemVer

Version is `0.1.0-dev.1`. **0.x minors may include breaking changes.**

---

## License

MIT. See `LICENSE`.
