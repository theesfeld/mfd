/* Smoke test: pure libvge (assembly), no Rust. */
#include "vge.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const uint32_t W = 64, H = 64;
    uint8_t *buf = calloc(W * H, 4);
    if (!buf)
        return 1;

    VgeSurface s = {.width = W, .height = H, .stride = W * 4, .pixels = buf};

    printf("vge_version: %s\n", vge_version());

    vge_clear(&s, 0xFF000000u); /* transparent/black alpha */
    vge_line(&s, 0, 0, 63, 0, 0xFF00FF46u);
    vge_circle(&s, 32, 32, 10, 0xFF28DCFF);
    vge_line_thick(&s, 0, 63, 63, 63, 0xFFFFC828u, 3);

    uint32_t p;
    memcpy(&p, buf, 4);
    printf("pixel(0,0)=0x%08X (expect green stroke)\n", p);
    if ((p & 0x00FFFFFF) != 0x00FF46) {
        fprintf(stderr, "FAIL: line did not light pixel\n");
        free(buf);
        return 2;
    }

    printf("OK — pure assembly libvge\n");
    free(buf);
    return 0;
}
