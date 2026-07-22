/* SPDX-License-Identifier: MIT
 * VGE extra — pure System V AMD64 assembly (no C in the library).
 * Completes the public C ABI from include/vge.h.
 */
        .text
        .intel_syntax noprefix
        .file   "mfd_extra.s"

/*--------------------------------------------------------------------
 * mfd_line_thick(s, x0, y0, x1, y1, color, thickness)
 * rdi=s esi=x0 edx=y0 ecx=x1 r8d=y1 r9d=color  [rbp+16]=thickness
 *------------------------------------------------------------------*/
        .globl  mfd_line_thick
        .type   mfd_line_thick, @function
        .align  16
mfd_line_thick:
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        sub     rsp, 40

        mov     r12, rdi
        mov     dword ptr [rbp - 48], esi
        mov     dword ptr [rbp - 52], edx
        mov     dword ptr [rbp - 56], ecx
        mov     dword ptr [rbp - 60], r8d
        mov     r13d, r9d
        mov     eax, dword ptr [rbp + 16]
        cmp     eax, 1
        jg      .Llt_multi
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 48]
        mov     edx, dword ptr [rbp - 52]
        mov     ecx, dword ptr [rbp - 56]
        mov     r8d, dword ptr [rbp - 60]
        mov     r9d, r13d
        call    mfd_line
        jmp     .Llt_done
.Llt_multi:
        mov     r14d, eax                      /* thick */
        mov     r15d, r14d
        sar     r15d, 1
        neg     r15d                           /* o = -half */
.Llt_o:
        mov     eax, r14d
        sar     eax, 1
        cmp     r15d, eax
        jg      .Llt_done
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 48]
        add     esi, r15d
        mov     edx, dword ptr [rbp - 52]
        mov     ecx, dword ptr [rbp - 56]
        add     ecx, r15d
        mov     r8d, dword ptr [rbp - 60]
        mov     r9d, r13d
        call    mfd_line
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 48]
        mov     edx, dword ptr [rbp - 52]
        add     edx, r15d
        mov     ecx, dword ptr [rbp - 56]
        mov     r8d, dword ptr [rbp - 60]
        add     r8d, r15d
        mov     r9d, r13d
        call    mfd_line
        inc     r15d
        jmp     .Llt_o
.Llt_done:
        add     rsp, 40
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   mfd_line_thick, .-mfd_line_thick

/*--------------------------------------------------------------------
 * mfd_rect_fill(s, x0, y0, x1, y1, color)
 *------------------------------------------------------------------*/
        .globl  mfd_rect_fill
        .type   mfd_rect_fill, @function
        .align  16
mfd_rect_fill:
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        sub     rsp, 24

        test    rdi, rdi
        jz      .Lrf_done
        mov     r12, rdi
        mov     r13d, r9d                      /* color */
        cmp     esi, ecx
        jle     1f
        xchg    esi, ecx
1:      cmp     edx, r8d
        jle     2f
        xchg    edx, r8d
2:      mov     eax, dword ptr [r12]
        mov     r10d, dword ptr [r12 + 4]
        test    esi, esi
        jns     3f
        xor     esi, esi
3:      test    edx, edx
        jns     4f
        xor     edx, edx
4:      cmp     ecx, eax
        jl      5f
        lea     ecx, [rax - 1]
5:      cmp     r8d, r10d
        jl      6f
        lea     r8d, [r10 - 1]
6:      cmp     esi, ecx
        jg      .Lrf_done
        cmp     edx, r8d
        jg      .Lrf_done
        mov     dword ptr [rbp - 48], esi
        mov     dword ptr [rbp - 52], ecx
        mov     r14d, edx
        mov     r15d, r8d
.Lrf_y:
        cmp     r14d, r15d
        jg      .Lrf_done
        mov     ebx, dword ptr [rbp - 48]
.Lrf_x:
        cmp     ebx, dword ptr [rbp - 52]
        jg      .Lrf_yn
        mov     rdi, r12
        mov     esi, ebx
        mov     edx, r14d
        mov     ecx, r13d
        call    mfd_plot
        inc     ebx
        jmp     .Lrf_x
.Lrf_yn:
        inc     r14d
        jmp     .Lrf_y
.Lrf_done:
        add     rsp, 24
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   mfd_rect_fill, .-mfd_rect_fill

/*--------------------------------------------------------------------
 * mfd_blit(dst, src)  rdi=dst rsi=src
 *------------------------------------------------------------------*/
        .globl  mfd_blit
        .type   mfd_blit, @function
        .align  16
mfd_blit:
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        test    rdi, rdi
        jz      .Lb_done
        test    rsi, rsi
        jz      .Lb_done
        mov     r8, qword ptr [rdi + 16]
        mov     r9, qword ptr [rsi + 16]
        test    r8, r8
        jz      .Lb_done
        test    r9, r9
        jz      .Lb_done
        mov     r10d, dword ptr [rdi]
        mov     r11d, dword ptr [rdi + 4]
        mov     eax, dword ptr [rsi]
        mov     ecx, dword ptr [rsi + 4]
        cmp     r10d, eax
        cmova   r10d, eax
        cmp     r11d, ecx
        cmova   r11d, ecx
        test    r10d, r10d
        jz      .Lb_done
        test    r11d, r11d
        jz      .Lb_done
        mov     r12d, dword ptr [rdi + 8]
        mov     r13d, dword ptr [rsi + 8]
        mov     r14d, r10d
        shl     r14d, 2
        xor     r15d, r15d
.Lb_y:
        cmp     r15d, r11d
        jae     .Lb_done
        mov     eax, r15d
        imul    eax, r12d
        mov     rdi, r8
        add     rdi, rax
        mov     eax, r15d
        imul    eax, r13d
        mov     rsi, r9
        add     rsi, rax
        mov     ecx, r14d
        rep     movsb
        inc     r15d
        jmp     .Lb_y
.Lb_done:
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        ret
        .size   mfd_blit, .-mfd_blit

/*--------------------------------------------------------------------
 * mfd_decay(s, factor_256)  rdi=s  esi=factor
 *------------------------------------------------------------------*/
        .globl  mfd_decay
        .type   mfd_decay, @function
        .align  16
mfd_decay:
        push    rbx
        push    r12
        push    r13
        test    rdi, rdi
        jz      .Ld_done
        mov     r8, qword ptr [rdi + 16]
        test    r8, r8
        jz      .Ld_done
        mov     r9d, dword ptr [rdi]
        mov     r10d, dword ptr [rdi + 4]
        mov     r11d, dword ptr [rdi + 8]
        mov     ebx, esi
        cmp     ebx, 256
        jbe     1f
        mov     ebx, 256
1:      xor     r12d, r12d                     /* y */
.Ld_y:
        cmp     r12d, r10d
        jae     .Ld_done
        mov     eax, r12d
        imul    eax, r11d
        lea     r13, [r8 + rax]                /* row */
        xor     ecx, ecx                       /* x */
.Ld_x:
        cmp     ecx, r9d
        jae     .Ld_yn
        mov     eax, dword ptr [r13 + rcx*4]
        /* scale R G B, keep A */
        mov     edx, eax
        and     edx, 0xFF000000                /* A */
        mov     esi, eax
        shr     esi, 16
        and     esi, 0xFF
        imul    esi, ebx
        shr     esi, 8
        shl     esi, 16
        or      edx, esi
        mov     esi, eax
        shr     esi, 8
        and     esi, 0xFF
        imul    esi, ebx
        shr     esi, 8
        shl     esi, 8
        or      edx, esi
        mov     esi, eax
        and     esi, 0xFF
        imul    esi, ebx
        shr     esi, 8
        or      edx, esi
        mov     dword ptr [r13 + rcx*4], edx
        inc     ecx
        jmp     .Ld_x
.Ld_yn:
        inc     r12d
        jmp     .Ld_y
.Ld_done:
        pop     r13
        pop     r12
        pop     rbx
        ret
        .size   mfd_decay, .-mfd_decay

/*--------------------------------------------------------------------
 * mfd_export_rgb24(s, dest)  rdi=s  rsi=dest
 *------------------------------------------------------------------*/
        .globl  mfd_export_rgb24
        .type   mfd_export_rgb24, @function
        .align  16
mfd_export_rgb24:
        push    rbx
        push    r12
        test    rdi, rdi
        jz      .Le_done
        test    rsi, rsi
        jz      .Le_done
        mov     r8, qword ptr [rdi + 16]
        test    r8, r8
        jz      .Le_done
        mov     r9d, dword ptr [rdi]
        mov     r10d, dword ptr [rdi + 4]
        mov     r11d, dword ptr [rdi + 8]
        mov     r12, rsi                       /* dest */
        xor     ebx, ebx                       /* y */
.Le_y:
        cmp     ebx, r10d
        jae     .Le_done
        mov     eax, ebx
        imul    eax, r11d
        lea     rcx, [r8 + rax]
        xor     edx, edx                       /* x */
.Le_x:
        cmp     edx, r9d
        jae     .Le_yn
        mov     eax, dword ptr [rcx + rdx*4]
        /* R */
        mov     esi, eax
        shr     esi, 16
        mov     byte ptr [r12], sil
        inc     r12
        /* G */
        mov     esi, eax
        shr     esi, 8
        mov     byte ptr [r12], sil
        inc     r12
        /* B */
        mov     byte ptr [r12], al
        inc     r12
        inc     edx
        jmp     .Le_x
.Le_yn:
        inc     ebx
        jmp     .Le_y
.Le_done:
        pop     r12
        pop     rbx
        ret
        .size   mfd_export_rgb24, .-mfd_export_rgb24

/*--------------------------------------------------------------------
 * mfd_polyline(s, xy, n, color)
 * rdi=s rsi=xy edx=n ecx=color
 *------------------------------------------------------------------*/
        .globl  mfd_polyline
        .type   mfd_polyline, @function
        .align  16
mfd_polyline:
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        sub     rsp, 8
        test    rdi, rdi
        jz      .Lpl_done
        test    rsi, rsi
        jz      .Lpl_done
        cmp     edx, 2
        jl      .Lpl_done
        mov     r12, rdi
        mov     r13, rsi
        mov     r14d, edx
        mov     r15d, ecx
        xor     ebx, ebx
.Lpl_loop:
        mov     eax, r14d
        dec     eax
        cmp     ebx, eax
        jge     .Lpl_done
        mov     rdi, r12
        mov     esi, dword ptr [r13 + rbx*8]
        mov     edx, dword ptr [r13 + rbx*8 + 4]
        mov     ecx, dword ptr [r13 + rbx*8 + 8]
        mov     r8d, dword ptr [r13 + rbx*8 + 12]
        mov     r9d, r15d
        call    mfd_line
        inc     ebx
        jmp     .Lpl_loop
.Lpl_done:
        add     rsp, 8
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   mfd_polyline, .-mfd_polyline

/*--------------------------------------------------------------------
 * Pure-asm sin/cos (no libc). Taylor on reduced angle.
 * In:  xmm0 = radians
 * Out: xmm0 = sin, xmm1 = cos
 *------------------------------------------------------------------*/
        .align  16
.Lsincos:
        /* reduce roughly with fmod against 2π using trunc */
        movss   xmm2, dword ptr [rip + .Ltwo_pi]
        movss   xmm3, xmm0
        divss   xmm3, xmm2
        /* truncate toward zero via cvttss2si */
        cvttss2si eax, xmm3
        cvtsi2ss xmm3, eax
        mulss   xmm3, xmm2
        subss   xmm0, xmm3                     /* x in ~[-2π,2π] */
        /* clamp-ish: if |x| > π, x -= sign*2π already from fmod-ish */
        movss   xmm2, dword ptr [rip + .Lpi]
        movss   xmm3, xmm0
        andps   xmm3, [rip + .Labs_mask]
        comiss  xmm3, xmm2
        jbe     .Lsc_ok
        movss   xmm4, dword ptr [rip + .Ltwo_pi]
        /* if x > 0: x -= 2π else x += 2π */
        xorps   xmm5, xmm5
        comiss  xmm0, xmm5
        jb      .Lsc_neg
        subss   xmm0, xmm4
        jmp     .Lsc_ok
.Lsc_neg:
        addss   xmm0, xmm4
.Lsc_ok:
        /* sin = x - x3/6 + x5/120 - x7/5040
           cos = 1 - x2/2 + x4/24 - x6/720 */
        movss   xmm2, xmm0                     /* x */
        movss   xmm3, xmm0
        mulss   xmm3, xmm0                     /* x2 */
        movss   xmm4, xmm3
        mulss   xmm4, xmm0                     /* x3 */
        movss   xmm5, xmm4
        mulss   xmm5, xmm0                     /* x4 */
        movss   xmm6, xmm5
        mulss   xmm6, xmm0                     /* x5 */
        movss   xmm7, xmm6
        mulss   xmm7, xmm0                     /* x6 */
        movss   xmm8, xmm7
        mulss   xmm8, xmm0                     /* x7 */
        /* sin */
        movss   xmm0, xmm2
        movss   xmm9, xmm4
        mulss   xmm9, dword ptr [rip + .Linv6]
        subss   xmm0, xmm9
        movss   xmm9, xmm6
        mulss   xmm9, dword ptr [rip + .Linv120]
        addss   xmm0, xmm9
        movss   xmm9, xmm8
        mulss   xmm9, dword ptr [rip + .Linv5040]
        subss   xmm0, xmm9
        /* cos */
        movss   xmm1, dword ptr [rip + .Lone]
        movss   xmm9, xmm3
        mulss   xmm9, dword ptr [rip + .Linv2]
        subss   xmm1, xmm9
        movss   xmm9, xmm5
        mulss   xmm9, dword ptr [rip + .Linv24]
        addss   xmm1, xmm9
        movss   xmm9, xmm7
        mulss   xmm9, dword ptr [rip + .Linv720]
        subss   xmm1, xmm9
        ret

        .section .rodata
        .align  16
.Labs_mask: .long 0x7FFFFFFF, 0, 0, 0
.Ltwo_pi:   .float 6.28318530718
.Lpi:       .float 3.14159265359
.Lone:      .float 1.0
.Linv2:     .float 0.5
.Linv6:     .float 0.166666666667
.Linv24:    .float 0.0416666666667
.Linv120:   .float 0.00833333333333
.Linv720:   .float 0.00138888888889
.Linv5040:  .float 0.00019841269841
        .text

/*--------------------------------------------------------------------
 * Transforms — float SSE, pure asm (no C, no libm)
 * MfdXform: a b tx c d ty  at offsets 0 4 8 12 16 20
 *------------------------------------------------------------------*/
        .globl  mfd_xform_identity
        .type   mfd_xform_identity, @function
mfd_xform_identity:
        test    rdi, rdi
        jz      1f
        mov     dword ptr [rdi], 0x3F800000    /* 1.0f a */
        mov     dword ptr [rdi + 4], 0
        mov     dword ptr [rdi + 8], 0
        mov     dword ptr [rdi + 12], 0
        mov     dword ptr [rdi + 16], 0x3F800000 /* 1.0f d */
        mov     dword ptr [rdi + 20], 0
1:      ret
        .size   mfd_xform_identity, .-mfd_xform_identity

        .globl  mfd_xform_translate
        .type   mfd_xform_translate, @function
mfd_xform_translate:
        /* rdi=m  xmm0=tx xmm1=ty ; tx' = a*tx + b*ty + tx_old */
        test    rdi, rdi
        jz      1f
        movss   xmm2, dword ptr [rdi]          /* a */
        mulss   xmm2, xmm0                     /* a*tx */
        movss   xmm3, dword ptr [rdi + 4]      /* b */
        mulss   xmm3, xmm1                     /* b*ty */
        addss   xmm2, xmm3
        addss   xmm2, dword ptr [rdi + 8]
        movss   dword ptr [rdi + 8], xmm2
        movss   xmm2, dword ptr [rdi + 12]     /* c */
        mulss   xmm2, xmm0
        movss   xmm3, dword ptr [rdi + 16]     /* d */
        mulss   xmm3, xmm1
        addss   xmm2, xmm3
        addss   xmm2, dword ptr [rdi + 20]
        movss   dword ptr [rdi + 20], xmm2
1:      ret
        .size   mfd_xform_translate, .-mfd_xform_translate

        .globl  mfd_xform_scale
        .type   mfd_xform_scale, @function
mfd_xform_scale:
        test    rdi, rdi
        jz      1f
        /* a*=sx c*=sx  b*=sy d*=sy */
        movss   xmm2, dword ptr [rdi]
        mulss   xmm2, xmm0
        movss   dword ptr [rdi], xmm2
        movss   xmm2, dword ptr [rdi + 12]
        mulss   xmm2, xmm0
        movss   dword ptr [rdi + 12], xmm2
        movss   xmm2, dword ptr [rdi + 4]
        mulss   xmm2, xmm1
        movss   dword ptr [rdi + 4], xmm2
        movss   xmm2, dword ptr [rdi + 16]
        mulss   xmm2, xmm1
        movss   dword ptr [rdi + 16], xmm2
1:      ret
        .size   mfd_xform_scale, .-mfd_xform_scale

        .globl  mfd_xform_rotate
        .type   mfd_xform_rotate, @function
mfd_xform_rotate:
        /* rdi=m  xmm0=radians — pure asm, no libm */
        push    r12
        sub     rsp, 24
        test    rdi, rdi
        jz      .Lxr_done
        mov     r12, rdi
        call    .Lsincos                       /* xmm0=sin xmm1=cos */
        movss   dword ptr [rsp], xmm0          /* sn */
        movss   dword ptr [rsp + 4], xmm1      /* cs */
        /* a' = a*cs + b*sn ; b' = -a*sn + b*cs */
        movss   xmm0, dword ptr [r12]          /* a */
        movss   xmm1, dword ptr [r12 + 4]      /* b */
        movss   xmm2, dword ptr [rsp + 4]      /* cs */
        movss   xmm3, dword ptr [rsp]          /* sn */
        movss   xmm4, xmm0
        mulss   xmm4, xmm2
        movss   xmm5, xmm1
        mulss   xmm5, xmm3
        addss   xmm4, xmm5
        movss   xmm5, xmm0
        mulss   xmm5, xmm3
        xorps   xmm6, xmm6
        subss   xmm6, xmm5
        movss   xmm5, xmm1
        mulss   xmm5, xmm2
        addss   xmm6, xmm5
        movss   dword ptr [r12], xmm4
        movss   dword ptr [r12 + 4], xmm6
        movss   xmm0, dword ptr [r12 + 12]
        movss   xmm1, dword ptr [r12 + 16]
        movss   xmm2, dword ptr [rsp + 4]
        movss   xmm3, dword ptr [rsp]
        movss   xmm4, xmm0
        mulss   xmm4, xmm2
        movss   xmm5, xmm1
        mulss   xmm5, xmm3
        addss   xmm4, xmm5
        movss   xmm5, xmm0
        mulss   xmm5, xmm3
        xorps   xmm6, xmm6
        subss   xmm6, xmm5
        movss   xmm5, xmm1
        mulss   xmm5, xmm2
        addss   xmm6, xmm5
        movss   dword ptr [r12 + 12], xmm4
        movss   dword ptr [r12 + 16], xmm6
.Lxr_done:
        add     rsp, 24
        pop     r12
        ret
        .size   mfd_xform_rotate, .-mfd_xform_rotate

        .globl  mfd_xform_apply
        .type   mfd_xform_apply, @function
mfd_xform_apply:
        /* rdi=m xmm0=x xmm1=y rsi=ox rdx=oy */
        test    rdi, rdi
        jz      1f
        test    rsi, rsi
        jz      1f
        test    rdx, rdx
        jz      1f
        movss   xmm2, dword ptr [rdi]
        mulss   xmm2, xmm0
        movss   xmm3, dword ptr [rdi + 4]
        mulss   xmm3, xmm1
        addss   xmm2, xmm3
        addss   xmm2, dword ptr [rdi + 8]
        movss   dword ptr [rsi], xmm2
        movss   xmm2, dword ptr [rdi + 12]
        mulss   xmm2, xmm0
        movss   xmm3, dword ptr [rdi + 16]
        mulss   xmm3, xmm1
        addss   xmm2, xmm3
        addss   xmm2, dword ptr [rdi + 20]
        movss   dword ptr [rdx], xmm2
1:      ret
        .size   mfd_xform_apply, .-mfd_xform_apply

        .globl  mfd_line_xf
        .type   mfd_line_xf, @function
mfd_line_xf:
        /* rdi=s rsi=m  xmm0..3 = x0 y0 x1 y1  r8d? color is 7th arg
         * SysV: color after 6 float? Actually floats in xmm, color is next GP: rdx?
         * Args: s, m, x0, y0, x1, y1, color
         * rdi=s rsi=m xmm0=x0 xmm1=y0 xmm2=x1 xmm3=y1  edx=color (first free GP after rsi)
         */
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        sub     rsp, 40
        mov     r12, rdi
        mov     r13, rsi
        mov     r14d, edx                      /* color */
        test    r13, r13
        jnz     .Lxf_m
        /* no matrix: round floats to int and line */
        cvtss2si esi, xmm0
        cvtss2si edx, xmm1
        cvtss2si ecx, xmm2
        cvtss2si r8d, xmm3
        mov     rdi, r12
        mov     r9d, r14d
        call    mfd_line_aa
        jmp     .Lxf_done
.Lxf_m:
        /* apply m to (x0,y0) and (x1,y1) */
        movss   dword ptr [rsp], xmm0
        movss   dword ptr [rsp + 4], xmm1
        movss   dword ptr [rsp + 8], xmm2
        movss   dword ptr [rsp + 12], xmm3
        lea     rsi, [rsp + 16]
        lea     rdx, [rsp + 20]
        mov     rdi, r13
        movss   xmm0, dword ptr [rsp]
        movss   xmm1, dword ptr [rsp + 4]
        call    mfd_xform_apply
        lea     rsi, [rsp + 24]
        lea     rdx, [rsp + 28]
        mov     rdi, r13
        movss   xmm0, dword ptr [rsp + 8]
        movss   xmm1, dword ptr [rsp + 12]
        call    mfd_xform_apply
        movss   xmm0, dword ptr [rsp + 16]
        cvtss2si esi, xmm0
        movss   xmm0, dword ptr [rsp + 20]
        cvtss2si edx, xmm0
        movss   xmm0, dword ptr [rsp + 24]
        cvtss2si ecx, xmm0
        movss   xmm0, dword ptr [rsp + 28]
        cvtss2si r8d, xmm0
        mov     rdi, r12
        mov     r9d, r14d
        call    mfd_line_aa
.Lxf_done:
        add     rsp, 40
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   mfd_line_xf, .-mfd_line_xf

/*--------------------------------------------------------------------
 * mfd_version → pointer to rodata string
 *------------------------------------------------------------------*/
        .section .rodata
        .align  8
.Lver:
        .asciz  "0.1.0-dev.1-asm"
        .text
        .globl  mfd_version
        .type   mfd_version, @function
mfd_version:
        lea     rax, [rip + .Lver]
        ret
        .size   mfd_version, .-mfd_version

        .section .note.GNU-stack, "", @progbits
