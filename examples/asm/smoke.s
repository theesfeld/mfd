/* Pure-asm smoke: no C, no Rust, no libc. Links only libvge objects.
 * exit 0 = pass, exit 2 = fail
 */
        .intel_syntax noprefix
        .text
        .globl  _start
        .type   _start, @function
_start:
        /* stack: surface + 64*64*4 pixel buffer */
        /* align rsp */
        and     rsp, -16
        sub     rsp, 16448                     /* surface 32 + pixels 16384 + pad */

        /* VgeSurface at [rsp]: w,h,stride,pad,pixels* */
        lea     rax, [rsp + 32]                /* pixel buffer */
        mov     dword ptr [rsp], 64            /* width */
        mov     dword ptr [rsp + 4], 64        /* height */
        mov     dword ptr [rsp + 8], 256       /* stride = 64*4 */
        mov     dword ptr [rsp + 12], 0
        mov     qword ptr [rsp + 16], rax

        /* clear black opaque */
        mov     rdi, rsp
        mov     esi, 0xFF000000
        call    vge_clear

        /* green line along top */
        mov     rdi, rsp
        xor     esi, esi
        xor     edx, edx
        mov     ecx, 63
        xor     r8d, r8d
        mov     r9d, 0xFF00FF46
        call    vge_line

        /* check pixel (0,0) */
        mov     eax, dword ptr [rsp + 32]
        and     eax, 0x00FFFFFF
        cmp     eax, 0x00FF46
        jne     .Lfail

        /* success: exit(0) */
        mov     rax, 60                        /* sys_exit */
        xor     rdi, rdi
        syscall

.Lfail:
        mov     rax, 60
        mov     rdi, 2
        syscall
        .size   _start, .-_start
        .section .note.GNU-stack, "", @progbits
