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

; this sets up identity paging
set_up_page_tables:
    ; map first P4 entry to P3 table
    mov eax, p3_table
    or eax, 0b11 ; present + writable
    mov [p4_table], eax

    ; map first P3 entry to P2 table
    mov eax, p2_table
    or eax, 0b11 ; present + writable
    mov [p3_table], eax

    ; map each P2 entry to a P1 table
    mov ecx, 0 ; counter variable

    .map_p2_table:
        ; map each ecx-th P2 entry to a P1 table
        lea eax, [p1_tables + ecx * 8]
        shl eax, 9   ; eax => p1_tables + ecx * 4096
        or eax, 0b11 ; present + writable
        mov [p2_table + ecx * 8], eax

        ; map each edx-th P1 entry to a standard 4KB frame
        mov edx, 0 ; counter variable

        .map_p1_table:
            ; get the addr of the ecx-th P1 table
            lea eax, [p1_tables + ecx * 8]
            shl eax, 9   ; eax => p1_tables + ecx * 4096
            mov ebx, eax ; ebx => p1_tables + ecx * 4096

            ; get the addr of the frame to be mapped (ecx * 512 * 4096 + edx * 4096)
            mov     eax, ecx ; eax => ecx
            shl     eax, 9   ; eax => ecx * 512
            add     eax, edx ; eax => ecx * 512 + edx
            shl     eax, 12  ; eax => (ecx * 512 + edx) * 4096
            or eax, 0b11     ; present + writable
            mov [ebx + edx * 8], eax

            inc edx           ; increase counter
            cmp edx, 512      ; if counter == 512, the whole P1 table is mapped
            jne .map_p1_table ; else map the next entry

        inc ecx           ; increase counter
        cmp ecx, 512      ; if counter == 512, the whole P2 table is mapped
        jne .map_p2_table ; else map the next entry

    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the EFER MSR (model specific register)
    ; after enabling paging in the cr0 register, this will make the cpu go to the 32-bit compatibility submode
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8  ; long mode
    or eax, 1 << 11 ; NXE (enable the no execute bit)
    wrmsr

    ; set up the cr0 register
    mov eax, cr0
    or eax, 1 << 31 ; enable paging
    or eax, 1 << 16 ; enable the write protect bit
    mov cr0, eax

    ret

_start:
    ; at this point, the cpu is in 32 bit protected mode with paging disabled
    mov esp, stack_top ; setup the stack pointer
    mov edi, ebx ; pass the multiboot boot info pointer as argument to Rust

    call check_multiboot
    call check_cpuid
    call check_long_mode

    ; after this checks, we know that the bootloader used was multiboot compliant (as we expect)
    ; and the cpu supports 64 bit long mode. No changes to the cpu state were made yet, we are still
    ; in 32 bit protected mode with paging disabled

    call set_up_page_tables

    ; setup recursive mapping (useful to setup more advanced paging in Rust)
    mov eax, p4_table
    or eax, 0b11 ; present + writable
    mov [p4_table + 511 * 8], eax ; point the last P4 entry to the P4 table itself

    call enable_paging

    ; at this point, the cpu is in 32-bit compatibility submode (a submode of long mode) and
    ; we have paging enabled (using identity paging to simplify the assembly)
    ; Identity Paging --> maps virtual addrs to physical addrs of same value so,
    ; virtual addr = physical addr
    ; at this point, only the first 1GiB is mapped with 'huge' P2 pages

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    ; use a `far jump` to load the code selector into the cs (code selector) register
    jmp gdt64.code:_start_long_mode

section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
p1_tables:
    resb 4096 * 512
; The multiboot standard does not define the value of the stack pointer register
; (esp) and it is up to the kernel to provide a stack. This allocates 16K bytes
; for it, and creates a symbol at the top. The stack grows downwards on x86.
; The stack on x86 must be 16-byte aligned according to the System V ABI standard
; and de-facto extensions. The compiler will assume the stack is properly aligned
; and failure to align the stack will result in undefined behaviour.
stack_bottom:
    resb 4096 * 4 ; 16 Kb -> 4 memory pages
stack_top:

section .rodata
gdt64:
    dq 0 ; zero entry
    .code: equ $ - gdt64
        dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment

    .pointer:
        dw $ - gdt64 - 1
        dq gdt64

; +-------------------------------------------------------------------------+
; |  All the code below is 64 bit with the cpu running in 64 bit long mode  |
; +-------------------------------------------------------------------------+

section .text
bits 64

; rust entry point
extern main

_start_long_mode:
    ; load 0 into all data segment registers (to avoid future problems)
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; call rust
    call main
