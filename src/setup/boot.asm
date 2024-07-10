
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

bits 32

global start
section .text
start:
    ; print `OK` to screen
    mov dword [0xb8000], 0x2f4b2f4f
    hlt
