/* SPDX-License-Identifier: MIT
 * VGE — calligraphic vector graphics engine
 *
 * PRODUCT: pure assembly library (libvge) with this C ABI.
 * Link: -lvge -lm
 * Language bindings (Rust, etc.) are thin FFI only — not the core.
 *
 * Geometry → individual pixels. Color format: 0xAARRGGBB.
 */
#ifndef VGE_H
#define VGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Packed color: 0xAARRGGBB (alpha in high byte; 0 = transparent). */
typedef uint32_t vge_color;

/** Pixel surface: 32-bit pixels (0xAARRGGBB), row-major. */
typedef struct VgeSurface {
    uint32_t width;  /* pixels */
    uint32_t height; /* pixels */
    uint32_t stride; /* bytes per row; must be >= width * 4 */
    uint32_t _pad;
    uint8_t *pixels; /* length >= stride * height */
} VgeSurface;

/** 2D affine transform: [x'] = [a b tx] [x]
 *                       [y']   [c d ty] [y]
 *                                       [1] */
typedef struct VgeXform {
    float a, b, tx;
    float c, d, ty;
} VgeXform;

/* --- Surface / clear / plot --- */

/** Fill every pixel with color. */
void vge_clear(VgeSurface *s, vge_color color);

/** Light one pixel if (x,y) is inside the surface. */
void vge_plot(VgeSurface *s, int32_t x, int32_t y, vge_color color);

/* --- Integer screen-space vectors (hot path) --- */

/** Bresenham line: light every pixel from (x0,y0) to (x1,y1). */
void vge_line(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
              vge_color color);

/** Thick line (integer thickness, multi-pass). */
void vge_line_thick(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                    vge_color color, int32_t thickness);

/** Midpoint circle outline. */
void vge_circle(VgeSurface *s, int32_t cx, int32_t cy, int32_t r, vge_color color);

/** Filled axis-aligned rect [x0..x1]×[y0..y1] inclusive ends (clamped). */
void vge_rect_fill(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                   vge_color color);

/* --- Transform helpers (assembly; rotate uses libm sinf/cosf) --- */

void vge_xform_identity(VgeXform *m);
void vge_xform_translate(VgeXform *m, float tx, float ty);
void vge_xform_scale(VgeXform *m, float sx, float sy);
/** Rotate counter-clockwise by radians around origin, then current matrix. */
void vge_xform_rotate(VgeXform *m, float radians);
void vge_xform_apply(const VgeXform *m, float x, float y, float *ox, float *oy);

/** Transformed float endpoints → integer Bresenham on surface. */
void vge_line_xf(VgeSurface *s, const VgeXform *m, float x0, float y0, float x1,
                 float y1, vge_color color);

/** Polyline through n points (screen ints). n>=2. */
void vge_polyline(VgeSurface *s, const int32_t *xy, int32_t n, vge_color color);

/** Export RGB888 tightly packed (for display protocols). dest len = w*h*3. */
void vge_export_rgb24(const VgeSurface *s, uint8_t *dest);

/**
 * Copy src into dst (min width/height). Use for double-buffer present:
 * draw into a system-RAM surface, then blit once to the display surface.
 */
void vge_blit(VgeSurface *dst, const VgeSurface *src);

/**
 * Phosphor-style fade: each channel *= factor_256/256 (0..256).
 * Call instead of full clear for smooth vector trails. factor 220–245 is typical.
 */
void vge_decay(VgeSurface *s, uint32_t factor_256);

/** Engine version string (static). */
const char *vge_version(void);

#ifdef __cplusplus
}
#endif

#endif /* VGE_H */
