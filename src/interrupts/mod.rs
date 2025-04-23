// https://wiki.osdev.org/Interrupt_Descriptor_Table

use bitflags::bitflags;

// TODO: make sure there is no problem in having the InterruptDescriptor type Copyable

bitflags! {
    #[derive(Clone, Copy)]
    struct Ist: u8 {
        const IST_OFFSET = 0b111;
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    struct TypeAttributes: u8 {
        const GATE_TYPE = 0b00001111;
        const DPL       = 0b01100000; // Descriptor Privilege Level
        const PRESENT   = 0b10000000;
    }
}

#[repr(u8)]
enum GateType {
    InterruptGate = 0xE, // 0b1110
    TrapGate      = 0xF, // 0b1111
}

// this represents en entry on the IDT
// https://wiki.osdev.org/Interrupt_Descriptor_Table#Gate_Descriptor_2
#[repr(C)]
#[derive(Clone, Copy)]
struct InterruptDescriptor {
    offset_1: u16,              // offset bits 0..15
    selector: u16,              // a code segment selector in GDT or LDT
    ist: Ist,                   // bits 0..2 holds Interrupt Stack Table offset, rest of bits zero.
    type_attrs: TypeAttributes, // gate type, dpl, and p fields
    offset_2: u16,              // offset bits 16..31
    offset_3: u32,              // offset bits 32..63
    zero: u32,                  // reserved
}

#[repr(C)]
struct InterruptDescriptorTable {
    divide_error: InterruptDescriptor,
    debug_exception: InterruptDescriptor,
    non_maskable_interrupt: InterruptDescriptor,
    breakpoint: InterruptDescriptor,
    overflow: InterruptDescriptor,
    bound_range_exceeded: InterruptDescriptor,
    invalid_opcode: InterruptDescriptor,
    device_not_available: InterruptDescriptor,
    double_fault: InterruptDescriptor,
    coprocessor_segment_overrun: InterruptDescriptor, // reserved
    invalid_tss: InterruptDescriptor,
    segment_not_present: InterruptDescriptor,
    stack_segment_fault: InterruptDescriptor,
    general_protection: InterruptDescriptor,
    page_fault: InterruptDescriptor,
    intel_reserved: InterruptDescriptor, // reserved
    x87_fp_error: InterruptDescriptor,
    alignment_check: InterruptDescriptor,
    machine_check: InterruptDescriptor,
    simd_fp_exception: InterruptDescriptor,
    virtualization_exception: InterruptDescriptor,
    control_protection_exception: InterruptDescriptor,
    reserved_for_future_use: [InterruptDescriptor; 10], // reserved
    interrupt: [InterruptDescriptor; 224], // reserved
}

// TODO: are these the values it should have ?
impl InterruptDescriptor {
    /// Returns a completly zeroed out `InterruptDescriptor`.
    fn new() -> Self {
        InterruptDescriptor {
            offset_1: 0x0000,
            selector: 0, // ??
            ist: Ist::empty(), // ??
            type_attrs: TypeAttributes::empty(), // ??
            offset_2: 0x0000,
            offset_3: 0x0000,
            zero: 0x00000000
        }
    }
}

impl InterruptDescriptorTable {
    fn new() -> Self {
        InterruptDescriptorTable {
            divide_error: InterruptDescriptor::new(),
            debug_exception: InterruptDescriptor::new(),
            non_maskable_interrupt: InterruptDescriptor::new(),
            breakpoint: InterruptDescriptor::new(),
            overflow: InterruptDescriptor::new(),
            bound_range_exceeded: InterruptDescriptor::new(),
            invalid_opcode: InterruptDescriptor::new(),
            device_not_available: InterruptDescriptor::new(),
            double_fault: InterruptDescriptor::new(),
            coprocessor_segment_overrun: InterruptDescriptor::new(), // reserved
            invalid_tss: InterruptDescriptor::new(),
            segment_not_present: InterruptDescriptor::new(),
            stack_segment_fault: InterruptDescriptor::new(),
            general_protection: InterruptDescriptor::new(),
            page_fault: InterruptDescriptor::new(),
            intel_reserved: InterruptDescriptor::new(), // reserved
            x87_fp_error: InterruptDescriptor::new(),
            alignment_check: InterruptDescriptor::new(),
            machine_check: InterruptDescriptor::new(),
            simd_fp_exception: InterruptDescriptor::new(),
            virtualization_exception: InterruptDescriptor::new(),
            control_protection_exception: InterruptDescriptor::new(),
            reserved_for_future_use: [InterruptDescriptor::new(); 10], // reserved
            interrupt: [InterruptDescriptor::new(); 224], // reserved
        }
    }
}
