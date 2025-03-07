.code32

/*
 * Multiboot 2 header
 * +---------------+-----------------+-----------------------------------------+
 * |     Field     |      Type       |                  Value                  |
 * +---------------+-----------------+-----------------------------------------+
 * | magic number  |       u32       |               0xE85250D6                |
 * +---------------+-----------------+-----------------------------------------+
 * | architecture  |       u32       |         0 for i386, 4 for MIPS          |
 * +---------------+-----------------+-----------------------------------------+
 * | header length |       u32       |    total header size, including tags    |
 * +---------------+-----------------+-----------------------------------------+
 * | checksum      |       u32       | -(magic + architecture + header_length) |
 * +---------------+-----------------+-----------------------------------------+
 * | tags          |     variable    |                                         |
 * +---------------+-----------------+-----------------------------------------+
 * | end tag       | (u16, u16, u32) |                (0, 0, 8)                |
 * +---------------+-----------------+-----------------------------------------+
 */

/*
 * The bootloader will search for this signature in the first 8 KiB of the
 * kernel file, aligned at a 32-bit boundary. The signature is in its own
 * section so the header can be forced to be within the first 8 KiB of the kernel file.
 */
.section .multiboot_header
header_start:
    .long 0xE85250D6                                                   # magic number
    .long 0                                                            # architecture
    .long header_end - header_start                                    # header length
    .long 0x100000000 - (0xE85250D6 + 0 + (header_end - header_start)) # checksum
    # no optional multiboot 2 tags

    # end tag
    .word 0 # type
    .word 0 # flags
    .long 8 # size
header_end:

.global _start
.section .text

# prints `ERR: ` and the given error code to screen and hangs.
# al -> error code (in ascii)
error:
    movl $0x4f524f45, 0xb8000
    movl $0x4f3a4f52, 0xb8004
    movl $0x4f204f20, 0xb8008
    movb %al, 0xb800a
    hlt

check_multiboot:
    # magic value that multiboot compliant bootloaders put in eax before loading the kernel
    cmpl $0x36d76289, %eax
    jne .no_multiboot
    ret

    .no_multiboot:
        movb $'0', %al
        jmp error

# check if CPUID is supported by attempting to flip the ID bit (bit 21)
# in the FLAGS register. If we can flip it, CPUID is available.
# taken from https://wiki.osdev.org/Setting_Up_Long_Mode#Detection_of_CPUID
check_cpuid:
    # copy FLAGS into eax via the stack
    pushf
    pop %eax

    # copy to ecx as well for comparing later on
    movl %eax, %ecx

    # flip the id bit
    xorl $(1 << 21), %eax

    # copy eax to FLAGS via the stack
    push %eax
    popf

    # copy FLAGS back to eax (with the flipped bit if CPUID is supported)
    pushf
    pop %eax

    # Restore FLAGS from the old version stored in ECX (i.e., flipping the
    # ID bit back if it was ever flipped).
    push %ecx
    popf

    # compare eax and ecx. If they are equal, then that means the bit
    # wasn't flipped, and CPUID isn't supported.
    cmpl %ecx, %eax
    je .no_cpuid
    ret

    .no_cpuid:
        movb $'1', %al
        jmp error

# test if extended processor info in available
# taken from https://wiki.osdev.org/Setting_Up_Long_Mode#x86_or_x86-64
check_long_mode:
    movl $0x80000000, %eax # implicit argument for cpuid
    cpuid                  # get highest supported argument
    cmpl $0x80000001, %eax # needs to be at least 0x80000001
    jb no_long_mode        # if it's less, the cpu is too old for long mode

    # test if long mode is avalable using the extended info
    movl $0x80000001, %eax # argument for extended processor info
    cpuid                  # returns various feature bits in ecx and edx
    testl $(1 << 29), %edx # test if the lm-bit is set in the edx register
    jz no_long_mode        # if it's not set, there is no long mode
    ret

    no_long_mode:
        movb $'2', %al # move ascii "2" into al
        jmp error

# test if extended processor info in available
# taken from https://wiki.osdev.org/Setting_Up_Long_Mode#x86_or_x86-64
set_up_page_tables:
    # map first P4 entry to P3 table
    movl $p3_table, %eax
    orl $0b11, %eax # present + writable
    movl %eax, p4_table

    # map first P3 entry to P2 table
    movl $p2_table, %eax
    orl $0b11, %eax # present + writable
    movl %eax, p3_table

    # map each P2 entry to a P1 table
    movl $0, %ecx # counter variable

    .map_p2_table:
        # map each ecx-th P2 entry to a P1 table
        leal p1_tables(, %ecx, 8), %eax
        shll $9, %eax   # eax => p1_tables + ecx * 4096
        orl $0b11, %eax # present + writable
        movl %eax, p2_table(, %ecx, 8)

        # map each edx-th P1 entry to a standard 4KB frame
        movl $0, %edx # counter variable

        .map_p1_table:
            # get the addr of the ecx-th P1 table
            leal p1_tables(, %ecx, 8), %eax
            shll $9, %eax   # eax => p1_tables + ecx * 4096
            movl %eax, %ebx # ebx => p1_tables + ecx * 4096

            # get the addr of the frame to be mapped (ecx * 512 * 4096 + edx * 4096)
            movl %ecx, %eax # eax => ecx
            shll $9, %eax   # eax => ecx * 512
            addl %edx, %eax # eax => ecx * 512 + edx
            shll $12, %eax  # eax => (ecx * 512 + edx) * 4096
            orl $0b11, %eax # present + writable
            movl %eax, (%ebx, %edx, 8)

            incl %edx         # increase counter
            cmpl $512, %edx   # if counter == 512, the whole P1 table is mapped
            jne .map_p1_table # else map the next entry

        incl %ecx         # increase counter
        cmpl $512, %ecx   # if counter == 512, the whole P2 table is mapped
        jne .map_p2_table # else map the next entry

    ret

enable_paging:
    # load P4 to cr3 register (CPU uses this to access the P4 table)
    movl $p4_table, %eax
    movl %eax, %cr3

    # enable PAE-flag in cr4 (Physical Address Extension)
    movl %cr4, %eax
    orl $(1 << 5), %eax
    movl %eax, %cr4

    # set the EFER MSR (Model-Specific Register)
    # after enabling paging in the cr0 register, this will make the CPU go to the 32-bit compatibility submode
    movl $0xC0000080, %ecx
    rdmsr
    orl $(1 << 8), %eax  # enable long mode
    orl $(1 << 11), %eax # enable NXE (No-Execute bit)
    wrmsr

    # set up the cr0 register
    movl %cr0, %eax
    orl $(1 << 31), %eax # enable paging
    orl $(1 << 16), %eax # enable the write protect bit
    movl %eax, %cr0

    ret

.section .text
.global _start
_start:
    # at this point, the CPU is in 32-bit protected mode with paging disabled
    movl $stack_top, %esp # setup the stack pointer
    movl %ebx, %edi       # pass the multiboot boot info pointer as argument to Rust

    call check_multiboot
    call check_cpuid
    call check_long_mode

    # after these checks, we know that the bootloader used was multiboot compliant
    # and the CPU supports 64-bit long mode. No changes to the CPU state were made yet.

    call set_up_page_tables

    # setup recursive mapping (useful for setting up more advanced paging in Rust)
    movl $p4_table, %eax
    orl $0b11, %eax               # Present + Writable
    movl %eax, p4_table + 511 * 8 # point the last P4 entry to itself

    call enable_paging

    # at this point, we are in 32-bit compatibility submode with paging enabled
    # Identity Paging --> Virtual addresses map to physical addresses of the same value

    # load the 64-bit GDT
    lgdt gdt64.pointer

    # far jump to load the code selector into the CS register
    ljmp $0x08, $_start_long_mode

.section .bss
.align 4096
p4_table:
    .skip 4096
p3_table:
    .skip 4096
p2_table:
    .skip 4096
p1_tables:
    .skip 4096 * 512

# The multiboot standard does not define the value of the stack pointer register
# (esp), and it is up to the kernel to provide a stack. This allocates 16K bytes
# for it, and creates a symbol at the top. The stack grows downwards on x86.
# The stack on x86 must be 16-byte aligned according to the System V ABI standard
# and de-facto extensions. Failure to align the stack will result in undefined behavior.
stack_bottom:
    .skip 4096 * 4  # 16 KB -> 4 memory pages
stack_top:

.section .rodata
gdt64:
    .quad 0 # Zero entry

gdt64_code:
    .quad (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53) # code segment

gdt64.pointer:
    .word gdt64.pointer - gdt64 - 1
    .quad gdt64

/*
 * +-------------------------------------------------------------------------+
 * |  All the code below is 64 bit with the cpu running in 64 bit long mode  |
 * +-------------------------------------------------------------------------+
 */

.code64
.section .text

_start_long_mode:
    # load 0 into all data segment registers (to avoid future problems)
    movq $0, %rax
    movq %rax, %ss
    movq %rax, %ds
    movq %rax, %es
    movq %rax, %fs
    movq %rax, %gs

    # call Rust main function
    call main
