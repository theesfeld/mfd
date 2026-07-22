/* SPDX-License-Identifier: MIT
 * VGE hot path — System V AMD64 ABI (GNU as, intel syntax)
 *
 * MfdSurface:
 *   0  width   u32
 *   4  height  u32
 *   8  stride  u32
 *  12  _pad    u32
 *  16  pixels  u64
 *
 * Pixel: 4-byte dword color 0x00RRGGBB.
 *
 * Design for speed:
 *  - clear: single rep stosd when tightly packed (stride == w*4)
 *  - line: Bresenham with INLINED pixel stores (no call per pixel)
 */
        .text
        .intel_syntax noprefix
        .file   "vge.s"

/*--------------------------------------------------------------------
 * mfd_plot(MfdSurface *s, int32_t x, int32_t y, mfd_color color)
 * rdi=s  esi=x  edx=y  ecx=color
 *------------------------------------------------------------------*/
        .globl  mfd_plot
        .type   mfd_plot, @function
        .align  16
mfd_plot:
        test    rdi, rdi
        jz      .Lp_ret
        test    esi, esi
        js      .Lp_ret
        test    edx, edx
        js      .Lp_ret
        cmp     esi, dword ptr [rdi]
        jae     .Lp_ret
        cmp     edx, dword ptr [rdi + 4]
        jae     .Lp_ret
        mov     r8, qword ptr [rdi + 16]
        test    r8, r8
        jz      .Lp_ret
        mov     r9d, dword ptr [rdi + 8]
        mov     eax, edx
        imul    eax, r9d
        lea     eax, [rax + rsi*4]
        mov     dword ptr [r8 + rax], ecx
.Lp_ret:
        ret
        .size   mfd_plot, .-mfd_plot

/*--------------------------------------------------------------------
 * mfd_clear — fill surface
 * rdi=s  esi=color
 * Tight packing (stride == width*4): one rep stosd over all pixels.
 *------------------------------------------------------------------*/
        .globl  mfd_clear
        .type   mfd_clear, @function
        .align  16
mfd_clear:
        push    rbx
        test    rdi, rdi
        jz      .Lc_done
        mov     r8, qword ptr [rdi + 16]
        test    r8, r8
        jz      .Lc_done
        mov     r9d, dword ptr [rdi]           /* width */
        mov     r10d, dword ptr [rdi + 4]      /* height */
        mov     r11d, dword ptr [rdi + 8]      /* stride */
        test    r9d, r9d
        jz      .Lc_done
        test    r10d, r10d
        jz      .Lc_done
        mov     eax, esi                       /* color */
        /* tight? stride == width * 4 */
        mov     ebx, r9d
        shl     ebx, 2
        cmp     ebx, r11d
        jne     .Lc_rows
        /* bulk: count = width * height */
        mov     ecx, r9d
        imul    ecx, r10d
        mov     rdi, r8
        rep     stosd
        jmp     .Lc_done
.Lc_rows:
        xor     edx, edx                       /* y */
.Lc_y:
        cmp     edx, r10d
        jae     .Lc_done
        mov     ecx, edx
        imul    ecx, r11d
        mov     rdi, r8
        add     rdi, rcx
        mov     ecx, r9d
        rep     stosd
        inc     edx
        jmp     .Lc_y
.Lc_done:
        pop     rbx
        ret
        .size   mfd_clear, .-mfd_clear

/*--------------------------------------------------------------------
 * mfd_line — Bresenham, inlined pixel store
 * rdi=s  esi=x0  edx=y0  ecx=x1  r8d=y1  r9d=color
 *------------------------------------------------------------------*/
        .globl  mfd_line
        .type   mfd_line, @function
        .align  16
mfd_line:
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        sub     rsp, 24

        test    rdi, rdi
        jz      .Ll_done
        mov     r10, qword ptr [rdi + 16]      /* pixels base */
        test    r10, r10
        jz      .Ll_done
        mov     r11d, dword ptr [rdi]          /* width */
        mov     r12d, dword ptr [rdi + 4]      /* height */
        mov     r13d, dword ptr [rdi + 8]      /* stride */
        mov     r14d, r9d                      /* color */

        mov     r15d, esi                      /* x */
        mov     ebx, edx                       /* y */
        mov     dword ptr [rbp - 48], ecx      /* x1 */
        mov     dword ptr [rbp - 52], r8d      /* y1 */

        /* dx = abs(x1-x0), sx */
        mov     eax, dword ptr [rbp - 48]
        sub     eax, r15d
        mov     edx, 1
        jge     1f
        neg     eax
        mov     edx, -1
1:      mov     dword ptr [rbp - 56], eax      /* dx */
        mov     dword ptr [rbp - 60], edx      /* sx */

        /* dy = -abs(y1-y0), sy */
        mov     eax, dword ptr [rbp - 52]
        sub     eax, ebx
        mov     edx, 1
        jge     2f
        neg     eax
        mov     edx, -1
2:      neg     eax
        mov     dword ptr [rbp - 64], eax      /* dy */
        mov     r9d, edx                       /* sy in r9d */

        /* err = dx + dy */
        mov     eax, dword ptr [rbp - 56]
        add     eax, dword ptr [rbp - 64]
        /* eax = err; keep in eax across loop carefully */

.Ll_loop:
        /* plot (r15d, ebx) if in bounds — inlined */
        test    r15d, r15d
        js      .Ll_nopixel
        test    ebx, ebx
        js      .Ll_nopixel
        cmp     r15d, r11d
        jae     .Ll_nopixel
        cmp     ebx, r12d
        jae     .Ll_nopixel
        mov     ecx, ebx
        imul    ecx, r13d
        lea     ecx, [rcx + r15*4]
        mov     dword ptr [r10 + rcx], r14d
.Ll_nopixel:

        /* done? */
        cmp     r15d, dword ptr [rbp - 48]
        jne     3f
        cmp     ebx, dword ptr [rbp - 52]
        je      .Ll_done
3:
        mov     edx, eax
        add     edx, edx                       /* e2 = 2*err */

        /* if e2 >= dy */
        cmp     edx, dword ptr [rbp - 64]
        jl      4f
        add     eax, dword ptr [rbp - 64]
        add     r15d, dword ptr [rbp - 60]     /* x += sx */
4:
        /* if e2 <= dx */
        cmp     edx, dword ptr [rbp - 56]
        jg      5f
        add     eax, dword ptr [rbp - 56]
        add     ebx, r9d                       /* y += sy */
5:
        jmp     .Ll_loop

.Ll_done:
        add     rsp, 24
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   mfd_line, .-mfd_line

/*--------------------------------------------------------------------
 * mfd_circle — midpoint, inlined 8-way plots
 * rdi=s  esi=cx  edx=cy  ecx=r  r8d=color
 *------------------------------------------------------------------*/
        .globl  mfd_circle
        .type   mfd_circle, @function
        .align  16
mfd_circle:
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        sub     rsp, 40

        test    rdi, rdi
        jz      .Lci_done
        test    ecx, ecx
        jle     .Lci_done
        mov     r10, qword ptr [rdi + 16]
        test    r10, r10
        jz      .Lci_done
        mov     r11d, dword ptr [rdi]          /* w */
        mov     r12d, dword ptr [rdi + 4]      /* h */
        mov     r13d, dword ptr [rdi + 8]      /* stride */
        mov     r14d, esi                      /* cx */
        mov     r15d, edx                      /* cy */
        mov     ebx, ecx                       /* x = r */
        xor     eax, eax
        mov     dword ptr [rbp - 48], eax      /* y = 0 */
        mov     eax, 1
        sub     eax, ecx
        mov     dword ptr [rbp - 52], eax      /* err */
        mov     dword ptr [rbp - 56], r8d      /* color */

        /* helper macro-style: plot at (r14d+ox, r15d+oy) — use stack ox,oy */
.Lci_loop:
        cmp     ebx, dword ptr [rbp - 48]
        jl      .Lci_done

        /* 8 plots via a tiny inlined sequence */
        /* +x +y */
        mov     esi, r14d
        add     esi, ebx
        mov     edx, r15d
        add     edx, dword ptr [rbp - 48]
        call    .Lci_plot1
        /* +y +x */
        mov     esi, r14d
        add     esi, dword ptr [rbp - 48]
        mov     edx, r15d
        add     edx, ebx
        call    .Lci_plot1
        /* -y +x */
        mov     esi, r14d
        sub     esi, dword ptr [rbp - 48]
        mov     edx, r15d
        add     edx, ebx
        call    .Lci_plot1
        /* -x +y */
        mov     esi, r14d
        sub     esi, ebx
        mov     edx, r15d
        add     edx, dword ptr [rbp - 48]
        call    .Lci_plot1
        /* -x -y */
        mov     esi, r14d
        sub     esi, ebx
        mov     edx, r15d
        sub     edx, dword ptr [rbp - 48]
        call    .Lci_plot1
        /* -y -x */
        mov     esi, r14d
        sub     esi, dword ptr [rbp - 48]
        mov     edx, r15d
        sub     edx, ebx
        call    .Lci_plot1
        /* +y -x */
        mov     esi, r14d
        add     esi, dword ptr [rbp - 48]
        mov     edx, r15d
        sub     edx, ebx
        call    .Lci_plot1
        /* +x -y */
        mov     esi, r14d
        add     esi, ebx
        mov     edx, r15d
        sub     edx, dword ptr [rbp - 48]
        call    .Lci_plot1

        mov     eax, dword ptr [rbp - 48]
        inc     eax
        mov     dword ptr [rbp - 48], eax
        cmp     dword ptr [rbp - 52], 0
        jge     .Lci_epos
        mov     eax, dword ptr [rbp - 48]
        add     eax, eax
        inc     eax
        add     dword ptr [rbp - 52], eax
        jmp     .Lci_loop
.Lci_epos:
        dec     ebx
        mov     eax, dword ptr [rbp - 48]
        sub     eax, ebx
        add     eax, eax
        inc     eax
        add     dword ptr [rbp - 52], eax
        jmp     .Lci_loop

/* local: plot esi=x edx=y using r10 base, r11 w, r12 h, r13 stride, color [rbp-56] */
.Lci_plot1:
        test    esi, esi
        js      .Lci_p1r
        test    edx, edx
        js      .Lci_p1r
        cmp     esi, r11d
        jae     .Lci_p1r
        cmp     edx, r12d
        jae     .Lci_p1r
        mov     eax, edx
        imul    eax, r13d
        lea     eax, [rax + rsi*4]
        mov     ecx, dword ptr [rbp - 56]
        mov     dword ptr [r10 + rax], ecx
.Lci_p1r:
        ret

.Lci_done:
        add     rsp, 40
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   mfd_circle, .-mfd_circle

        .section .note.GNU-stack, "", @progbits
