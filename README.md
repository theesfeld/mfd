# VGE — Vector Graphics Engine

<!-- agents:status:begin -->
> **Status:** active · Version: `0.1.0-dev.1` · License: MIT · [Issues](https://github.com/theesfeld/vge/issues)
<!-- agents:status:end -->

VGE is a **calligraphic stroke engine** for modern hosts.

Aircraft HUDs and 1970s vector CRTs did not paint full bitmaps as their native model.  
They held a **display list** of beam commands (MOVE / DRAW). A refresh processor retraced the list. Phosphor held the glow.

VGE uses that model in 2026:

1. **`DisplayList`** — live stroke commands (source of truth)
2. **`refresh`** — phosphor decay + software beam through the list → scanout pixels
3. **Present** — Kitty / half-block / Linux FB shows the scanout only

Hot path on **x86_64**: GNU assembly for beam pixel stores (`asm/x86_64/vge.s`).  
Other targets use a portable C path with the same C ABI.

<p align="center">
  <img src="docs/demo-hud.png" alt="Vector HUD sample from VGE" width="640" />
</p>

---

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

```toml
# Cargo.toml
vge = { git = "https://github.com/theesfeld/vge" }
```

C header: `include/vge.h`  
Link the static/shared library from `cargo build --release` (`libvge.a` / `libvge.so`).

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
# Default: smooth 120 Hz overlay (Ghostty / Kitty / xterm / …)
cargo run --release --bin vge-demo

# 60 Hz (often smoothest on 60 Hz panels)
VGE_HZ=60 cargo run --release --bin vge-demo

# Uncapped (max throughput; can look choppier in terminals)
VGE_HZ=0 cargo run --release --bin vge-demo

# Effects (optional; costs extra CPU)
VGE_EFFECTS=glow,radar cargo run --release --bin vge-demo

# Linux video RAM path
cargo run --release --bin vge-demo -- --fb

# Full alternate screen
cargo run --release --bin vge-demo -- --full
```

| Flag / env | Effect |
|------------|--------|
| (default) | Overlay viewport; text chrome around it |
| `--fb` | RAM draw + blit to `/dev/fb0` |
| `--full` | Alternate screen, full area |
| `VGE_HZ=120` | Default: phase-lock 120 Hz (smooth) |
| `VGE_HZ=60` | Phase-lock 60 Hz |
| `VGE_HZ=0` | Uncapped (throughput test; can stutter) |
| `VGE_WIDTH=N` | Stroke width in pixels (default 1; no artificial max) |
| `VGE_TERM=kitty\|half\|ascii` | Force present backend |
| `VGE_MAX_W` / `VGE_MAX_H` | Cap pixel buffer (default 960×540) |
| `VGE_EFFECTS=…` | `glow`, `bloom`, `radar`, `scan` |
| `VGE_PHOSPHOR=1` | Decay trail instead of hard clear |

Quit: `q`, Esc, or Ctrl+C.

---

## API surface

### Stroke display list (calligraphic core)

| Type / method | Description |
|---------------|-------------|
| `DisplayList` | Refresh memory for beam commands |
| `set_color` / `move_to` / `line_to` | Beam state + strokes |
| `line` / `line_thick` / `circle` / `polyline` | Draw commands |
| `stroke(surface)` | Execute beam (no clear) |
| `set_width(px)` | Stroke width in pixels (≥ 1) |
| `refresh(surface)` | Transparent clear + full retrace (overlay-ready) |

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
include/vge.h           C ABI
asm/x86_64/vge.s        assembly hot path
c/vge_portable.c        transforms, blit, decay, portable raster
src/lib.rs              Rust API
src/stroke.rs           DisplayList (calligraphic core)
src/term.rs             terminal present + viewport overlay
src/fb.rs               Linux framebuffer
src/frame.rs            display refresh lock
src/effects.rs          glow / bloom / radar / scanlines
src/bin/vge-demo.rs     stroke-list HUD demo
examples/bench.rs       FPS bench
docs/demo-hud.png       sample image
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
