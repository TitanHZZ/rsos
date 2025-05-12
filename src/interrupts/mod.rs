// https://wiki.osdev.org/Interrupt_Descriptor_Table
// https://wiki.osdev.org/Interrupts_Tutorial
pub mod tss;
pub mod gdt;

use core::{marker::PhantomData, arch::asm};
use crate::{io_port::IoPort, memory::VirtualAddress};
use bitflags::bitflags;

/// # Safety
/// 
/// The caller must ensure that both the **GDT** and **IDT** are correct, valid and loaded.
pub unsafe fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

pub fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

const PIC1: u16         = 0x20; /* IO base address for master PIC */
const PIC2: u16         = 0xA0; /* IO base address for slave PIC */
const PIC1_COMMAND: u16 = PIC1;
const PIC1_DATA: u16    = PIC1 + 1;
const PIC2_COMMAND: u16 = PIC2;
const PIC2_DATA: u16    = PIC2 + 1;

// https://wiki.osdev.org/8259_PIC#Disabling
/// Disables the PICs (master and slave) by masking all of their interrupts.
pub fn disable_pics() {
    IoPort::write_u8(PIC1_DATA, 0xFF);
    IoPort::write_u8(PIC2_DATA, 0xFF);
}

const GATE_TYPE_MASK: u8 = 0b0000_1111;
const DPL_LEVEL_MASK: u8 = 0b0110_0000;

#[repr(u8)]
pub enum GateType {
    InterruptGate = 0x0E, // 0b0000_1110
    TrapGate      = 0x0F, // 0b0000_1111
}

#[repr(u8)]
pub enum DplLevel {
    Ring0 = 0x00 << 5, // 0b0000_0000
    Ring1 = 0x01 << 5, // 0b0010_0000
    Ring2 = 0x02 << 5, // 0b0100_0000
    Ring3 = 0x03 << 5, // 0b0110_0000
}

// https://en.wikipedia.org/wiki/FLAGS_register
// https://wiki.osdev.org/CPU_Registers_x86-64#RFLAGS_Register
bitflags! {
    #[repr(C)]
    #[derive(Debug)]
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
        const IO_PRIVILEGE_LEVEL_LO          = 1 << 12;
        const IO_PRIVILEGE_LEVEL_HI          = 1 << 13;
        const NESTED_TASK                    = 1 << 14;
        const RESUME_FLAG                    = 1 << 16;
        const VIRTUAL_8086_MODE              = 1 << 17;
        const ALIGNMENT_CHECK_ACCESS_CONTROL = 1 << 18;
        const VIRTUAL_INTERRUPT_FLAG         = 1 << 19;
        const VIRTUAL_INTERRUPT_PENDING      = 1 << 20;
        const ID_FLAG                        = 1 << 21;
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct InterruptArgs {
    instruction_pointer: VirtualAddress,
    code_segment: u16,
    rflags: RFLAGS,
    stack_pointer: VirtualAddress,
    stack_segment: u16,
}

#[repr(C, packed)]
struct IdtR {
    size: u16, // size of the IDT (minus 1)
    addr: VirtualAddress, // virtual addr of the IDT
}

pub trait InterruptFunc {
    fn to_virt_addr(self) -> VirtualAddress;
}

// x86-interrupt calling convention
// https://github.com/rust-lang/rust/issues/40180
pub type IntFunc = extern "x86-interrupt" fn(args: InterruptArgs);
pub type IntFuncWithErr = extern "x86-interrupt" fn(args: InterruptArgs, error_code: u64);

impl InterruptFunc for IntFunc {
    fn to_virt_addr(self) -> VirtualAddress {
        self as VirtualAddress
    }
}

impl InterruptFunc for IntFuncWithErr {
    fn to_virt_addr(self) -> VirtualAddress {
        self as VirtualAddress
    }
}

// this represents en entry on the IDT
// https://wiki.osdev.org/Interrupt_Descriptor_Table#Gate_Descriptor_2
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InterruptDescriptor<F: InterruptFunc> {
    offset_1: u16,  // offset bits 0..15
    selector: u16,  // a code segment selector in GDT or LDT
    ist: u8,        // bits 0..2 holds Interrupt Stack Table offset, rest of bits zero.
    type_attrs: u8, // gate type, dpl, and p fields
    offset_2: u16,  // offset bits 16..31
    offset_3: u32,  // offset bits 32..63
    zero: u32,      // reserved

    _func: PhantomData<F>,
}

// TODO: critical exceptions should probably use different (dedicated) stacks
impl<F: InterruptFunc> InterruptDescriptor<F> {
    /// Creates a new `InterruptDescriptor` with the following defaults:
    ///   - The fn offset is 0
    ///   - The code segment selector is 0x8. This is the first entry in the GDT after the null descriptor
    ///   - IST is set to 0 so, the invocation uses the current stack
    ///   - The gate type is interrupt so, interrups are disabled during handler invocation
    ///   - DPL is 0 so, only the kernel (ring 0) can invoque the fn
    ///   - PRESENT is 0
    const fn new() -> Self {
        InterruptDescriptor {
            offset_1: 0x0000,
            selector: 0x8, // just use the basic code segment in the GDT
            ist: 0x00, // always use the current stack
            type_attrs: GateType::InterruptGate as _,
            offset_2: 0x0000,
            offset_3: 0x0000,
            zero: 0x00000000,

            _func: PhantomData,
        }
    }

    /// Sets the fn addr and the 'present' bit.
    pub fn set_fn(&mut self, func: F) {
        let addr = func.to_virt_addr();
        self.offset_1 = (addr >> 00) as u16 & 0xFFFF;
        self.offset_2 = (addr >> 16) as u16 & 0xFFFF;
        self.offset_3 = (addr >> 32) as u32 & 0xFFFFFFFF;

        // set the present flag
        self.type_attrs |= 0b1000_0000;
    }

    /// Sets the gate type.
    pub fn set_gate_type(&mut self, gate_type: GateType) {
        self.type_attrs = (self.type_attrs & !GATE_TYPE_MASK) | gate_type as u8;
    }

    /// Sets the DPL level.
    pub fn set_dpl_level(&mut self, dpl_level: DplLevel) {
        self.type_attrs = (self.type_attrs & !DPL_LEVEL_MASK) | dpl_level as u8;
    }
}

#[repr(C)]
pub struct InterruptDescriptorTable {
    pub divide_error                : InterruptDescriptor<IntFunc>,
    pub debug_exception             : InterruptDescriptor<IntFunc>,
    pub non_maskable_interrupt      : InterruptDescriptor<IntFunc>,
    pub breakpoint                  : InterruptDescriptor<IntFunc>,
    pub overflow                    : InterruptDescriptor<IntFunc>,
    pub bound_range_exceeded        : InterruptDescriptor<IntFunc>,
    pub invalid_opcode              : InterruptDescriptor<IntFunc>,
    pub device_not_available        : InterruptDescriptor<IntFunc>,
    pub double_fault                : InterruptDescriptor<IntFuncWithErr>,
    coprocessor_segment_overrun     : InterruptDescriptor<IntFunc>,          // reserved
    pub invalid_tss                 : InterruptDescriptor<IntFuncWithErr>,
    pub segment_not_present         : InterruptDescriptor<IntFuncWithErr>,
    pub stack_segment_fault         : InterruptDescriptor<IntFuncWithErr>,
    pub general_protection          : InterruptDescriptor<IntFuncWithErr>,
    pub page_fault                  : InterruptDescriptor<IntFuncWithErr>,
    intel_reserved                  : InterruptDescriptor<IntFunc>,          // reserved
    pub x87_fp_error                : InterruptDescriptor<IntFunc>,
    pub alignment_check             : InterruptDescriptor<IntFuncWithErr>,
    pub machine_check               : InterruptDescriptor<IntFunc>,
    pub simd_fp_exception           : InterruptDescriptor<IntFunc>,
    pub virtualization_exception    : InterruptDescriptor<IntFunc>,
    pub control_protection_exception: InterruptDescriptor<IntFuncWithErr>,
    reserved_for_future_use         : [InterruptDescriptor<IntFunc>; 10],    // reserved
    interrupt                       : [InterruptDescriptor<IntFunc>; 224],   // external interrupts (PIC/APIC)
}

impl InterruptDescriptorTable {
    /// Creates a new `InterruptDescriptorTable` where every entry is comes from [`InterruptDescriptor::new`].
    pub const fn new() -> Self {
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
            interrupt: [InterruptDescriptor::new(); 224], // external interrupts (PIC/APIC)
        }
    }

    /// Loads `idt` as the current IDT.  
    /// This does not enable/disable interrupts.
    /// 
    /// # Safety: 
    /// 
    /// The caller must ensure that `idt` is a valid IDT and interrupts **should** be disabled before
    /// loading the IDT and enabled again afterwards.  
    /// The IDT also **needs** to live for the duration of it's use where preferably, it's lifetime would be `'static`.
    pub unsafe fn load(idt: &'static Self) {
        let idtr = IdtR {
            size: size_of::<InterruptDescriptorTable>() as u16 - 1,
            addr: idt as *const InterruptDescriptorTable as VirtualAddress,
        };

        unsafe {
            asm!("lidt [{}]", in(reg) &idtr, options(nostack, preserves_flags));
        }
    }
}
