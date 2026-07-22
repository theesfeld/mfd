/* SPDX-License-Identifier: MIT
 * Xiaolin Wu antialiased lines — pure System V AMD64 asm.
 * Crisp hairlines with coverage blending (0xAARRGGBB).
 *
 * vge_plot_blend(s, x, y, color, cov)  cov = 0..255
 * vge_line_aa(s, x0, y0, x1, y1, color)
 */
        .text
        .intel_syntax noprefix
        .file   "vge_aa.s"

/*--------------------------------------------------------------------
 * vge_plot_blend(VgeSurface*, x, y, color, cov)
 * rdi=s esi=x edx=y ecx=color r8d=cov(0..255)
 * Writes color with alpha scaled by cov; keeps higher coverage if present.
 *------------------------------------------------------------------*/
        .globl  vge_plot_blend
        .type   vge_plot_blend, @function
        .align  16
vge_plot_blend:
        test    rdi, rdi
        jz      .Lpb_ret
        test    r8d, r8d
        jz      .Lpb_ret
        test    esi, esi
        js      .Lpb_ret
        test    edx, edx
        js      .Lpb_ret
        cmp     esi, dword ptr [rdi]
        jae     .Lpb_ret
        cmp     edx, dword ptr [rdi + 4]
        jae     .Lpb_ret
        mov     r9, qword ptr [rdi + 16]
        test    r9, r9
        jz      .Lpb_ret
        mov     r10d, dword ptr [rdi + 8]
        mov     eax, edx
        imul    eax, r10d
        lea     eax, [rax + rsi*4]
        lea     r9, [r9 + rax]                 /* pixel ptr */

        /* new_a = (src_a * cov) >> 8 ; if src_a==0 treat as 255 */
        mov     eax, ecx
        shr     eax, 24
        test    eax, eax
        jnz     1f
        mov     eax, 255
1:      imul    eax, r8d
        shr     eax, 8                         /* new_a */
        test    eax, eax
        jz      .Lpb_ret

        mov     r10d, dword ptr [r9]           /* old */
        mov     r11d, r10d
        shr     r11d, 24                       /* old_a */
        cmp     eax, r11d
        jb      .Lpb_ret                       /* keep stronger coverage */
        /* pack: (new_a<<24) | (color & 0x00FFFFFF) */
        and     ecx, 0x00FFFFFF
        shl     eax, 24
        or      ecx, eax
        mov     dword ptr [r9], ecx
.Lpb_ret:
        ret
        .size   vge_plot_blend, .-vge_plot_blend

/*--------------------------------------------------------------------
 * vge_line_aa — Xiaolin Wu (integer gradient, coverage blend)
 * rdi=s esi=x0 edx=y0 ecx=x1 r8d=y1 r9d=color
 *------------------------------------------------------------------*/
        .globl  vge_line_aa
        .type   vge_line_aa, @function
        .align  16
vge_line_aa:
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        sub     rsp, 72

        test    rdi, rdi
        jz      .Laa_done
        mov     r12, rdi                       /* s */
        mov     r13d, r9d                      /* color */

        /* steep = abs(y1-y0) > abs(x1-x0) */
        mov     eax, r8d
        sub     eax, edx
        mov     r10d, eax
        neg     r10d
        cmovs   r10d, eax                      /* abs(dy) */
        mov     eax, ecx
        sub     eax, esi
        mov     r11d, eax
        neg     r11d
        cmovs   r11d, eax                      /* abs(dx) */
        xor     ebx, ebx                       /* steep=0 */
        cmp     r10d, r11d
        jle     .Laa_swap0
        mov     ebx, 1
        /* swap x0,y0 and x1,y1 components */
        xchg    esi, edx
        xchg    ecx, r8d
.Laa_swap0:
        /* ensure x0 <= x1 */
        cmp     esi, ecx
        jle     .Laa_ord
        xchg    esi, ecx
        xchg    edx, r8d
.Laa_ord:
        mov     dword ptr [rbp - 48], esi      /* x0 */
        mov     dword ptr [rbp - 52], edx      /* y0 */
        mov     dword ptr [rbp - 56], ecx      /* x1 */
        mov     dword ptr [rbp - 60], r8d      /* y1 */
        mov     dword ptr [rbp - 64], ebx      /* steep */

        mov     eax, ecx
        sub     eax, esi
        mov     dword ptr [rbp - 68], eax      /* dx */
        mov     eax, r8d
        sub     eax, edx
        /* gradient = dy/dx as 16.16 fixed; if dx==0 vertical handled below */
        mov     r14d, eax                      /* dy signed */
        mov     eax, dword ptr [rbp - 68]
        test    eax, eax
        jnz     .Laa_grad
        /* dx==0: single column after steep transform */
        mov     edi, dword ptr [rbp - 48]
        mov     esi, dword ptr [rbp - 52]
        mov     edx, dword ptr [rbp - 60]
        cmp     esi, edx
        jle     2f
        xchg    esi, edx
2:      mov     r15d, esi
.Laa_vloop:
        cmp     r15d, edx
        jg      .Laa_done
        mov     eax, dword ptr [rbp - 64]
        test    eax, eax
        jz      3f
        /* steep: plot(y,x) */
        mov     rdi, r12
        mov     esi, r15d
        mov     edx, dword ptr [rbp - 48]
        mov     ecx, r13d
        mov     r8d, 255
        call    vge_plot_blend
        jmp     4f
3:      mov     rdi, r12
        mov     esi, dword ptr [rbp - 48]
        mov     edx, r15d
        mov     ecx, r13d
        mov     r8d, 255
        call    vge_plot_blend
4:      inc     r15d
        jmp     .Laa_vloop

.Laa_grad:
        /* gradient 16.16 = (dy << 16) / dx */
        movsxd  rax, r14d
        shl     rax, 16
        movsxd  rcx, dword ptr [rbp - 68]
        cqo
        idiv    rcx
        mov     r14, rax                       /* grad 16.16 in r14 */

        /* --- first endpoint --- */
        /* xend = round(x0) ; yend = y0 + grad*(xend-x0) */
        /* Use integer x0,y0 as already int endpoints: full cover ends */
        mov     edi, dword ptr [rbp - 48]      /* x */
        mov     esi, dword ptr [rbp - 52]      /* y */
        /* plot endpoint pair with full cover */
        mov     eax, dword ptr [rbp - 64]
        test    eax, eax
        jz      .Laa_e0
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 52]
        mov     edx, dword ptr [rbp - 48]
        mov     ecx, r13d
        mov     r8d, 255
        call    vge_plot_blend
        jmp     .Laa_e0b
.Laa_e0:
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 48]
        mov     edx, dword ptr [rbp - 52]
        mov     ecx, r13d
        mov     r8d, 255
        call    vge_plot_blend
.Laa_e0b:
        /* intery = y0 + grad  (start after first pixel) as 16.16 */
        movsxd  rax, dword ptr [rbp - 52]
        shl     rax, 16
        add     rax, r14
        mov     r15, rax                       /* intery 16.16 */

        /* x from x0+1 to x1-1 */
        mov     ebx, dword ptr [rbp - 48]
        inc     ebx
        mov     r10d, dword ptr [rbp - 56]
        dec     r10d
        cmp     ebx, r10d
        jg      .Laa_endpt
        mov     dword ptr [rbp - 72], r10d     /* x_last */

.Laa_loop:
        cmp     ebx, dword ptr [rbp - 72]
        jg      .Laa_endpt

        /* y = intery >> 16 ; fpart = (intery >> 8) & 0xFF  (approx 8-bit frac) */
        mov     eax, r15d
        sar     eax, 16                        /* ipart y */
        mov     edx, r15d
        shr     edx, 8
        movzx   edx, dl                        /* fpart 0..255 */
        mov     r8d, 255
        sub     r8d, edx                       /* rfpart */
        mov     r9d, eax                       /* y */

        mov     eax, dword ptr [rbp - 64]
        test    eax, eax
        jnz     .Laa_steep

        /* plot(x, y, rfpart); plot(x, y+1, fpart) */
        mov     rdi, r12
        mov     esi, ebx
        mov     edx, r9d
        mov     ecx, r13d
        /* r8 already rfpart */
        call    vge_plot_blend
        mov     rdi, r12
        mov     esi, ebx
        mov     edx, r9d
        inc     edx
        mov     ecx, r13d
        mov     eax, r15d
        shr     eax, 8
        movzx   r8d, al
        call    vge_plot_blend
        jmp     .Laa_next

.Laa_steep:
        /* plot(y, x, rfpart); plot(y+1, x, fpart) */
        mov     eax, 255
        mov     edx, r15d
        shr     edx, 8
        movzx   edx, dl
        sub     eax, edx
        mov     rdi, r12
        mov     esi, r9d
        mov     edx, ebx
        mov     ecx, r13d
        mov     r8d, eax
        call    vge_plot_blend
        mov     rdi, r12
        mov     esi, r9d
        inc     esi
        mov     edx, ebx
        mov     ecx, r13d
        mov     eax, r15d
        shr     eax, 8
        movzx   r8d, al
        call    vge_plot_blend

.Laa_next:
        add     r15, r14                       /* intery += grad */
        inc     ebx
        jmp     .Laa_loop

.Laa_endpt:
        /* second endpoint full cover */
        mov     eax, dword ptr [rbp - 64]
        test    eax, eax
        jz      .Laa_e1
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 60]
        mov     edx, dword ptr [rbp - 56]
        mov     ecx, r13d
        mov     r8d, 255
        call    vge_plot_blend
        jmp     .Laa_done
.Laa_e1:
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 56]
        mov     edx, dword ptr [rbp - 60]
        mov     ecx, r13d
        mov     r8d, 255
        call    vge_plot_blend

.Laa_done:
        add     rsp, 72
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   vge_line_aa, .-vge_line_aa

        .section .note.GNU-stack, "", @progbits
