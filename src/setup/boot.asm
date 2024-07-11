bits 32

; Multiboot 2 header
; +---------------+-----------------+-----------------------------------------+
; |     Field     |      Type       |                  Value                  |
; +---------------+-----------------+-----------------------------------------+
; | magic number  |       u32       |               0xE85250D6                |
; +---------------+-----------------+-----------------------------------------+
; | architecture  |       u32       |         0 for i386, 4 for MIPS          |
; +---------------+-----------------+-----------------------------------------+
; | header length |       u32       |    total header size, including tags    |
; +---------------+-----------------+-----------------------------------------+
; | checksum      |       u32       | -(magic + architecture + header_length) |
; +---------------+-----------------+-----------------------------------------+
; | tags          |     variable    |                                         |
; +---------------+-----------------+-----------------------------------------+
; | end tag       | (u16, u16, u32) |                (0, 0, 8)                |
; +---------------+-----------------+-----------------------------------------+

; The bootloader will search for this signature in the first 8 KiB of the
; kernel file, aligned at a 32-bit boundary. The signature is in its own
; section so the header can be forced to be within the first 8 KiB of the kernel file.
section .multiboot_header
header_start:
    dd 0xE85250D6                                                   ; magic number
    dd 0                                                            ; architecture
    dd header_end - header_start                                    ; header length
    dd 0x100000000 - (0xE85250D6 + 0 + (header_end - header_start)) ; checksum
    ; no optional multiboot 2 tags

    ; end tag
    dw 0    ; type
    dw 0    ; flags
    dd 8    ; size
header_end:

global _start
section .text

; prints `ERR: ` and the given error code to screen and hangs.
; al -> error code (in ascii)
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

check_multiboot:
    ; magic value that multiboot compliant bootloaders put in eax before loading the kernel
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret

    .no_multiboot:
        mov al, "0"
        jmp error

; check if CPUID is supported by attempting to flip the ID bit (bit 21)
; in the FLAGS register. If we can flip it, CPUID is available.
; taken from https://wiki.osdev.org/Setting_Up_Long_Mode#Detection_of_CPUID
check_cpuid:
    ; Copy FLAGS in to EAX via stack
    pushfd
    pop eax

    ; Copy to ECX as well for comparing later on
    mov ecx, eax

    ; Flip the ID bit
    xor eax, 1 << 21

    ; Copy EAX to FLAGS via the stack
    push eax
    popfd

    ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax

    ; Restore FLAGS from the old version stored in ECX (i.e. flipping the
    ; ID bit back if it was ever flipped).
    push ecx
    popfd

    ; Compare EAX and ECX. If they are equal then that means the bit
    ; wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je .no_cpuid
    ret

    .no_cpuid:
        mov al, "1"
        jmp error

; test if extended processor info in available
; taken from https://wiki.osdev.org/Setting_Up_Long_Mode#x86_or_x86-64
check_long_mode:
    mov eax, 0x80000000    ; implicit argument for cpuid
    cpuid                  ; get highest supported argument
    cmp eax, 0x80000001    ; it needs to be at least 0x80000001
    jb .no_long_mode       ; if it's less, the CPU is too old for long mode

    ; use extended info to test if long mode is available
    mov eax, 0x80000001    ; argument for extended processor info
    cpuid                  ; returns various feature bits in ecx and edx
    test edx, 1 << 29      ; test if the LM-bit is set in the D-register
    jz .no_long_mode       ; If it's not set, there is no long mode
    ret

    .no_long_mode:
        mov al, "2"
        jmp error

_start:
    mov esp, stack_top ; setup the stack pointer

    call check_multiboot
    call check_cpuid
    call check_long_mode

    ; print `OK` to screen
    mov dword [0xb8000], 0x2f4b2f4f
    hlt

; The multiboot standard does not define the value of the stack pointer register
; (esp) and it is up to the kernel to provide a stack. This allocates 64 bytes
; for it, and creates a symbol at the top. The stack grows downwards on x86.
; The stack on x86 must be 16-byte aligned according to the System V ABI standard
; and de-facto extensions. The compiler will assume the stack is properly aligned
; and failure to align the stack will result in undefined behaviour.
section .bss
align 16
stack_bottom:
    resb 64 ; 64 B
stack_top:
