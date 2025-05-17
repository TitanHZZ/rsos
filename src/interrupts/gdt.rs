// https://wiki.osdev.org/Global_Descriptor_Table
// https://wiki.osdev.org/GDT_Tutorial
use crate::memory::VirtualAddress;
use bitflags::bitflags;
use core::{arch::asm};
use super::tss::TSS;

// https://wiki.osdev.org/GDT_Tutorial#Long_Mode_2
pub unsafe fn reload_seg_regs() {
    unsafe {
        asm!(
            "push 0x08",              // Push code segment to stack, 0x08 is a stand-in for your code segment
            "lea {tmp}, [13f + rip]", // Load address of the label `13` into `reg`
            "push {tmp}",             // Push this value to the stack
            "retfq",                  // Perform a far return, RETFQ or LRETQ depending on syntax
            "13:",
            // Reload data segment registers
            "mov rax, 0", // 0x10 is a stand-in for your data segment
            "mov ss, rax",
            "mov ds, rax",
            "mov es, rax",
            "mov fs, rax",
            "mov gs, rax",
            tmp = lateout(reg) _,
        );
    }
}

bitflags! {
    #[repr(C)]
    pub struct NormalDescAccessByte: u8 {
        const ACCESSED            = 1 << 0;
        const RW                  = 1 << 1;
        const DC                  = 1 << 2; // Direction bit/Conforming bit
        const EXECUTABLE          = 1 << 3;
        const IS_CODE_OR_DATA_SEG = 1 << 4; // Descriptor type (code/data or system descriptor)
        const DPL_LO              = 1 << 5;
        const DPL_HI              = 1 << 6;
        const PRESENT             = 1 << 7;
    }
}

bitflags! {
    #[repr(C)]
    pub struct SystemDescAccessByte: u8 {
        // const TYPE             = 1 << 0 | 1 << 1 | 1 << 2 | 1 << 3;
        const IS_CODE_OR_DATA_SEG = 1 << 4; // Descriptor type (code/data or system descriptor)
        const DPL_LO              = 1 << 5;
        const DPL_HI              = 1 << 6;
        const PRESENT             = 1 << 7;
    }
}

bitflags! {
    #[repr(C)]
    pub struct SegmentFlags: u8 {
        // const RESERVED    = 1 << 0;
        const LONG_MODE_CODE = 1 << 1;
        const DB             = 1 << 2;
        const GRANULARITY    = 1 << 3;
    }
}

#[repr(u8)]
pub enum SystemDescAccessByteType {
    LDT               = 0x2,
    TssAvailable64bit = 0x9,
    TssBusy64bit      = 0xb,
}

#[repr(C)]
pub struct NormalSegmentDescriptor {
    limit_0: u16,
    base_0: u16,
    base_1: u8,
    access_byte: u8, // NormalAccessByte or SystemSegmentAccessByte
    limit_1_and_flags: u8,
    base_2: u8,
}

#[repr(C)]
pub struct SystemSegmentDescriptor {
    normal_desc: NormalSegmentDescriptor,
    base_3: u32,
    reserved: u32,
}

impl NormalSegmentDescriptor {
    /// Creates a completly zeroed out `NormalSegmentDescriptor`.
    pub const fn new() -> Self {
        NormalSegmentDescriptor {
            limit_0: 0,
            base_0: 0,
            base_1: 0,
            access_byte: 0,
            limit_1_and_flags: 0,
            base_2: 0,
        }
    }
}

impl SystemSegmentDescriptor {
    /// Creates a completly zeroed out `SystemSegmentDescriptor`.
    pub const fn new() -> Self {
        SystemSegmentDescriptor {
            normal_desc: NormalSegmentDescriptor::new(),
            base_3: 0,
            reserved: 0,
        }
    }
}

pub struct NormalDescAccessByteArgs {
    flags: NormalDescAccessByte,
}

impl NormalDescAccessByteArgs {
    pub fn new(flags: NormalDescAccessByte) -> Self {
        NormalDescAccessByteArgs { flags }
    }
}

pub struct SystemDescAccessByteArgs {
    flags: SystemDescAccessByte,
    seg_type: SystemDescAccessByteType,
}

impl SystemDescAccessByteArgs {
    pub fn new(flags: SystemDescAccessByte, seg_type: SystemDescAccessByteType) -> Self {
        SystemDescAccessByteArgs { flags, seg_type }
    }
}

pub trait SegmentDescriptor {
    type SegmentDescriptorArgs;

    fn set_limit(&mut self, limit: u32);
    fn set_base(&mut self, tss: &'static TSS);
    fn set_access_byte(&mut self, args: Self::SegmentDescriptorArgs);
    fn set_flags(&mut self, flags: SegmentFlags);
}

impl SegmentDescriptor for NormalSegmentDescriptor {
    type SegmentDescriptorArgs = NormalDescAccessByteArgs;

    fn set_limit(&mut self, limit: u32) {
        self.limit_0 = (limit & 0xFFFF) as u16;
        self.limit_1_and_flags = (self.limit_1_and_flags & 0xF0) | ((limit >> 16) & 0x0F) as u8;
    }

    fn set_base(&mut self, tss: &'static TSS) {
        let base = tss as *const TSS as VirtualAddress;
        self.base_0 = ((base >> 00) & 0x0000_FFFF) as u16;
        self.base_1 = ((base >> 16) & 0x0000_00FF) as u8;
        self.base_2 = ((base >> 24) & 0x0000_00FF) as u8;
    }

    fn set_access_byte(&mut self, args: Self::SegmentDescriptorArgs) {
        self.access_byte = args.flags.bits();
    }

    fn set_flags(&mut self, flags: SegmentFlags) {
        self.limit_1_and_flags = (self.limit_1_and_flags & 0x0F) | (flags.bits() << 4);
    }
}

impl SegmentDescriptor for SystemSegmentDescriptor {
    type SegmentDescriptorArgs = SystemDescAccessByteArgs;

    fn set_limit(&mut self, limit: u32) {
        self.normal_desc.set_limit(limit);
    }

    fn set_base(&mut self, tss: &'static TSS) {
        let base = tss as *const TSS as VirtualAddress;
        self.normal_desc.set_base(tss);
        self.base_3 = ((base >> 32) & 0xFFFF_FFFF) as u32;
    }

    fn set_access_byte(&mut self, args: Self::SegmentDescriptorArgs) {
        self.normal_desc.access_byte = args.flags.bits();
        self.normal_desc.access_byte |= args.seg_type as u8;
    }

    fn set_flags(&mut self, flags: SegmentFlags) {
        self.normal_desc.set_flags(flags);
    }
}

pub enum Descriptor<'a> {
    NormalDescriptor(&'a NormalSegmentDescriptor),
    SystemDescriptor(&'a SystemSegmentDescriptor),
}

#[repr(C)]
/// This represents a 64bit long-mode GDT that holds at most 5 normal descriptors (including the null descriptor) and 1 system segment.
pub struct GDT {
    descriptors: [u64; 7],

    // metadata to keep track of the GDT state
    normal_desc_count: u8,
    system_desc_count: u8,
}

// https://wiki.osdev.org/Segment_Selector
#[repr(C)]
pub struct SegmentSelector {
    selector: u16,
}

#[repr(C, packed)]
struct GDTR {
    size: u16,
    offset: u64,
}

impl GDT {
    /// Creates a new GDT with just a null descriptor,
    pub fn new() -> Self {
        GDT {
            descriptors: [0; 7],
            normal_desc_count: 1,
            system_desc_count: 0,
        }
    }

    // TODO: remove the panic!() and add a proper Result<> as the return
    pub fn new_descriptor(&mut self, desc: Descriptor) -> SegmentSelector {
        match desc {
            Descriptor::NormalDescriptor(n_desc) => {
                // make sure that the max limit is not violated
                if self.normal_desc_count >= 5 {
                    panic!("not enough gdt space");
                }

                let gdt_entry: u64 = n_desc.limit_0 as u64
                    | (n_desc.base_0 as u64) << 16
                    | (n_desc.base_1 as u64) << 32
                    | (n_desc.access_byte as u64) << 40
                    | (n_desc.limit_1_and_flags as u64) << 48
                    | (n_desc.base_2 as u64) << 56;

                let gdt_offset: usize = self.normal_desc_count as usize + self.system_desc_count as usize * 2;
                debug_assert!(gdt_offset <= 6);

                self.descriptors[gdt_offset] = gdt_entry;
                // TODO: it might make sense to add support for TI's and RPL's != 0
                SegmentSelector {
                    selector: self.normal_desc_count as u16 + self.system_desc_count as u16 * 2,
                }
            },
            Descriptor::SystemDescriptor(system_segment_descriptor) => {
                todo!()
            },
        }
    }

    // TODO: write the description and safety sections
    pub unsafe fn load(slf: &'static Self) {
        let gdtr = GDTR {
            size: (size_of::<GDT>() - 1) as u16,
            offset: slf as *const GDT as u64,
        };

        unsafe {
            asm!("lgdt [{}]", in(reg) &gdtr, options(nostack, preserves_flags));
        }
    }

    // Creates a new `GDT` with 3 predefined segment descriptors.
    // pub const fn new() -> Self {
    //     let mut gdt = GDT {
    //         null_descriptor: NormalSegmentDescriptor::new(),
    //         code_descriptor: NormalSegmentDescriptor::new(),
    //         tss_descriptor: SystemSegmentDescriptor::new(),
    //     };
    //     // set the type of the code descriptor to 'code/data segment descriptor'
    //     gdt.code_descriptor.access_byte |= 1 << 4;
    //     gdt
    // }
    // pub fn set_code_descriptor(&mut self, f: impl FnOnce(&mut NormalSegmentDescriptor)) {
    //     // use the read, modify, write pattern
    //     let mut tmp = unsafe { addr_of!(self.code_descriptor).read_unaligned() };
    //     f(&mut tmp);
    //     unsafe { addr_of_mut!(self.code_descriptor).write_unaligned(tmp) };
    // }
    // pub fn set_tss_descriptor(&mut self, f: impl FnOnce(&mut SystemSegmentDescriptor)) {
    //     // use the read, modify, write pattern
    //     let mut tmp = unsafe { addr_of!(self.tss_descriptor).read_unaligned() };
    //     f(&mut tmp);
    //     unsafe { addr_of_mut!(self.tss_descriptor).write_unaligned(tmp) };
    // }
    // // TODO: write the description and safety sections
    // pub unsafe fn load(slf: &'static Self) {
    //     let gdtr = GDTR {
    //         size: (size_of::<GDT>() - 1) as u16,
    //         offset: slf as *const GDT as u64,
    //     };
    //     unsafe {
    //         asm!("lgdt [{}]", in(reg) &gdtr, options(nostack, preserves_flags));
    //     }
    // }
}
