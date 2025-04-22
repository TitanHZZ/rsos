// https://wiki.osdev.org/Interrupt_Descriptor_Table#Gate_Descriptor_2

use bitflags::bitflags;

bitflags! {
    struct Ist: u8 {
        const IST_OFFSET = 0b111;
    }
}

bitflags! {
    struct TypeAttributes: u8 {
        const GATE_TYPE = 0b00001111;
        const DPL       = 0b01100000; // Descriptor Privilege Level
        const PRESENT   = 0b10000000;
    }
}

#[repr(u8)]
enum GateType {
    InterruptGate = 0xE, // 0b1110
    TrapGate = 0xF,      // 0b1111
}

#[repr(C)]
struct InterruptDescriptor {
    offset_1: u16,       // offset bits 0..15
    selector: u16,       // a code segment selector in GDT or LDT
    ist: u8,             // bits 0..2 holds Interrupt Stack Table offset, rest of bits zero.
    type_attributes: u8, // gate type, dpl, and p fields
    offset_2: u16,       // offset bits 16..31
    offset_3: u32,       // offset bits 32..63
    zero: u32,           // reserved
}
