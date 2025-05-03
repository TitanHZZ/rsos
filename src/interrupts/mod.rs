// https://wiki.osdev.org/Interrupt_Descriptor_Table
use crate::memory::VirtualAddress;
use core::marker::PhantomData;
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

// https://en.wikipedia.org/wiki/FLAGS_register
// https://wiki.osdev.org/CPU_Registers_x86-64#RFLAGS_Register
bitflags! {
    #[repr(C)]
    struct RFLAGS: u64 {
        const CARRY_FLAG                     = 1 << 0;
        const PARITY_FLAG                    = 1 << 2;
        const AUXILIARY_CARRY_FLAG           = 1 << 4;
        const ZERO_FLAG                      = 1 << 6;
        const SIGN_FLAG                      = 1 << 7;
        const TRAP_FLAG                      = 1 << 8;
        const INTERRUPT_ENABLE_FLAG          = 1 << 9;
        const DIRECTION_FLAG                 = 1 << 10;
        const OVERFLOW_FLAG                  = 1 << 11;
        const IO_PRIVILEGE_LEVEL             = (1 << 12) | (1 << 13);
        const NESTED_TASK                    = 1 << 14;
        const RESUME_FLAG                    = 1 << 16;
        const VIRTUAL_8086_MODE              = 1 << 17;
        const ALIGNMENT_CHECK_ACCESS_CONTROL = 1 << 18;
        const VIRTUAL_INTERRUPT_FLAG         = 1 << 19;
        const VIRTUAL_INTERRUPT_PENDING      = 1 << 20;
        const ID_FLAG                        = 1 << 21;
    }
}

#[repr(u8)]
enum GateType {
    InterruptGate = 0xE, // 0b1110
    TrapGate      = 0xF, // 0b1111
}

#[repr(C)]
struct InterruptArgs {
    instruction_pointer: VirtualAddress,
    code_segment: u16,
    rflags: RFLAGS,
    stack_pointer: VirtualAddress,
    stack_segment: u16,
}

trait InterruptFunc {}

// x86-interrupt calling convention
// https://github.com/rust-lang/rust/issues/40180
type IntFunc = extern "x86-interrupt" fn(args: InterruptArgs);
type IntFuncWithErr = extern "x86-interrupt" fn(args: InterruptArgs, error_code: u64);

impl InterruptFunc for IntFunc {}
impl InterruptFunc for IntFuncWithErr {}

// this represents en entry on the IDT
// https://wiki.osdev.org/Interrupt_Descriptor_Table#Gate_Descriptor_2
#[repr(C)]
#[derive(Clone, Copy)]
struct InterruptDescriptor<F: InterruptFunc> {
    offset_1: u16,              // offset bits 0..15
    selector: u16,              // a code segment selector in GDT or LDT
    ist: Ist,                   // bits 0..2 holds Interrupt Stack Table offset, rest of bits zero.
    type_attrs: TypeAttributes, // gate type, dpl, and p fields
    offset_2: u16,              // offset bits 16..31
    offset_3: u32,              // offset bits 32..63
    zero: u32,                  // reserved

    _func: PhantomData<F>,
}

#[repr(C)]
struct InterruptDescriptorTable {
    divide_error                : InterruptDescriptor<IntFunc>,
    debug_exception             : InterruptDescriptor<IntFunc>,
    non_maskable_interrupt      : InterruptDescriptor<IntFunc>,
    breakpoint                  : InterruptDescriptor<IntFunc>,
    overflow                    : InterruptDescriptor<IntFunc>,
    bound_range_exceeded        : InterruptDescriptor<IntFunc>,
    invalid_opcode              : InterruptDescriptor<IntFunc>,
    device_not_available        : InterruptDescriptor<IntFunc>,
    double_fault                : InterruptDescriptor<IntFuncWithErr>,
    coprocessor_segment_overrun : InterruptDescriptor<IntFunc>,          // reserved
    invalid_tss                 : InterruptDescriptor<IntFuncWithErr>,
    segment_not_present         : InterruptDescriptor<IntFuncWithErr>,
    stack_segment_fault         : InterruptDescriptor<IntFuncWithErr>,
    general_protection          : InterruptDescriptor<IntFuncWithErr>,
    page_fault                  : InterruptDescriptor<IntFuncWithErr>,
    intel_reserved              : InterruptDescriptor<IntFunc>,          // reserved
    x87_fp_error                : InterruptDescriptor<IntFunc>,
    alignment_check             : InterruptDescriptor<IntFuncWithErr>,
    machine_check               : InterruptDescriptor<IntFunc>,
    simd_fp_exception           : InterruptDescriptor<IntFunc>,
    virtualization_exception    : InterruptDescriptor<IntFunc>,
    control_protection_exception: InterruptDescriptor<IntFuncWithErr>,
    reserved_for_future_use     : [InterruptDescriptor<IntFunc>; 10],    // reserved
    interrupt                   : [InterruptDescriptor<IntFunc>; 224],   // reserved
}

// TODO: are these the values it should have ?
impl<F: InterruptFunc> InterruptDescriptor<F> {
    /// Returns a completly zeroed out `InterruptDescriptor`.
    fn new() -> Self {
        InterruptDescriptor {
            offset_1: 0x0000,
            selector: 0, // ??
            ist: Ist::empty(), // ??
            type_attrs: TypeAttributes::empty(), // ??
            offset_2: 0x0000,
            offset_3: 0x0000,
            zero: 0x00000000,

            _func: PhantomData,
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
