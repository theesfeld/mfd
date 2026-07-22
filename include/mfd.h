/* SPDX-License-Identifier: MIT
 * VGE — pure assembly vector graphics engine
 *
 * PRODUCT = libmfd (asm/x86_64/*.s only). No C. No Rust. No libc in the lib.
 * This header is the calling convention document (System V AMD64).
 *
 *   make && make install
 *   link: -lvge
 *
 * Color: 0xAARRGGBB. Geometry → individual pixels.
 */
#ifndef MFD_H
#define MFD_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Packed color: 0xAARRGGBB (alpha in high byte; 0 = transparent). */
typedef uint32_t mfd_color;

/** Pixel surface: 32-bit pixels (0xAARRGGBB), row-major. */
typedef struct MfdSurface {
    uint32_t width;  /* pixels */
    uint32_t height; /* pixels */
    uint32_t stride; /* bytes per row; must be >= width * 4 */
    uint32_t _pad;
    uint8_t *pixels; /* length >= stride * height */
} MfdSurface;

/** 2D affine transform: [x'] = [a b tx] [x]
 *                       [y']   [c d ty] [y]
 *                                       [1] */
typedef struct MfdXform {
    float a, b, tx;
    float c, d, ty;
} MfdXform;

/* --- Surface / clear / plot --- */

/** Fill every pixel with color. */
void mfd_clear(MfdSurface *s, mfd_color color);

/** Light one pixel if (x,y) is inside the surface. */
void mfd_plot(MfdSurface *s, int32_t x, int32_t y, mfd_color color);

/* --- Integer screen-space vectors (hot path) --- */

/** Bresenham line: light every pixel from (x0,y0) to (x1,y1). Fast, aliased. */
void mfd_line(MfdSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
              mfd_color color);

/**
 * Xiaolin Wu antialiased line (crisp hairlines). Coverage blends into
 * 0xAARRGGBB. Prefer this for width-1 strokes.
 */
void mfd_line_aa(MfdSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                 mfd_color color);

/** Blend one pixel with coverage 0..255 (used by AA). */
void mfd_plot_blend(MfdSurface *s, int32_t x, int32_t y, mfd_color color,
                    uint32_t coverage);

/** Thick line (integer thickness, multi-pass). */
void mfd_line_thick(MfdSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                    mfd_color color, int32_t thickness);

/** Midpoint circle outline. */
void mfd_circle(MfdSurface *s, int32_t cx, int32_t cy, int32_t r, mfd_color color);

/** Filled axis-aligned rect [x0..x1]×[y0..y1] inclusive ends (clamped). */
void mfd_rect_fill(MfdSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                   mfd_color color);

/* --- Transform helpers (pure assembly, no libm) --- */

void mfd_xform_identity(MfdXform *m);
void mfd_xform_translate(MfdXform *m, float tx, float ty);
void mfd_xform_scale(MfdXform *m, float sx, float sy);
/** Rotate counter-clockwise by radians around origin, then current matrix. */
void mfd_xform_rotate(MfdXform *m, float radians);
void mfd_xform_apply(const MfdXform *m, float x, float y, float *ox, float *oy);

/** Transformed float endpoints → integer Bresenham on surface. */
void mfd_line_xf(MfdSurface *s, const MfdXform *m, float x0, float y0, float x1,
                 float y1, mfd_color color);

/** Polyline through n points (screen ints). n>=2. */
void mfd_polyline(MfdSurface *s, const int32_t *xy, int32_t n, mfd_color color);

/** Export RGB888 tightly packed (for display protocols). dest len = w*h*3. */
void mfd_export_rgb24(const MfdSurface *s, uint8_t *dest);

/**
 * Copy src into dst (min width/height). Use for double-buffer present:
 * draw into a system-RAM surface, then blit once to the display surface.
 */
void mfd_blit(MfdSurface *dst, const MfdSurface *src);

/**
 * Phosphor-style fade: each channel *= factor_256/256 (0..256).
 * Call instead of full clear for smooth vector trails. factor 220–245 is typical.
 */
void mfd_decay(MfdSurface *s, uint32_t factor_256);

/** Engine version string (static). */
const char *mfd_version(void);

#ifdef __cplusplus
}
#endif

#endif /* MFD_H */
