// https://wiki.osdev.org/Global_Descriptor_Table
// https://wiki.osdev.org/GDT_Tutorial
use crate::memory::VirtualAddress;
use bitflags::bitflags;
use super::tss::TSS;
use core::arch::asm;

bitflags! {
    #[repr(C)]
    pub struct NormalSegmentAccessByte: u8 {
        const ACCESSED           = 1 << 0;
        const RW                 = 1 << 1;
        const DC                 = 1 << 2; // Direction bit/Conforming bit
        const EXECUTABLE         = 1 << 3;
        // const DESCRIPTOR_TYPE = 1 << 4; // Descriptor type (code/data or system descriptor)
        const DPL_LO             = 1 << 5;
        const DPL_HI             = 1 << 6;
        const PRESENT            = 1 << 7;
    }
}

bitflags! {
    #[repr(C)]
    pub struct SystemSegmentAccessByte: u8 {
        // const TYPE            = 1 << 0 | 1 << 1 | 1 << 2 | 1 << 3;
        // const DESCRIPTOR_TYPE = 1 << 4; // Descriptor type (code/data or system descriptor)
        const DPL_LO             = 1 << 5;
        const DPL_HI             = 1 << 6;
        const PRESENT            = 1 << 7;
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
pub enum SystemSegmentAccessByteType {
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
    const fn new() -> Self {
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
    const fn new() -> Self {
        SystemSegmentDescriptor {
            normal_desc: NormalSegmentDescriptor::new(),
            base_3: 0,
            reserved: 0,
        }
    }
}

pub struct NormalDescriptorAccessByteArgs {
    pub flags: NormalSegmentAccessByte,
}

pub struct SystemDescriptorAccessByteArgs {
    pub flags: SystemSegmentAccessByte,
    pub seg_type: SystemSegmentAccessByteType,
}

pub trait SegmentDescriptorTrait {
    type SegmentDescriptorArgs;

    fn set_limit(&mut self, limit: u32);
    fn set_base(&mut self, tss: &'static TSS);
    fn set_access_byte(&mut self, args: Self::SegmentDescriptorArgs);
    fn set_flags(&mut self, flags: SegmentFlags);
}

impl SegmentDescriptorTrait for NormalSegmentDescriptor {
    type SegmentDescriptorArgs = NormalDescriptorAccessByteArgs;

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

impl SegmentDescriptorTrait for SystemSegmentDescriptor {
    type SegmentDescriptorArgs = SystemDescriptorAccessByteArgs;

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

// TODO: this might have to be packed
#[repr(C)]
pub struct GDT {
    null_descriptor: NormalSegmentDescriptor,
    pub code_descriptor: NormalSegmentDescriptor,
    pub tss_descriptor: SystemSegmentDescriptor,
}

#[repr(C, packed)]
struct GDTR {
    size: u16,
    offset: u64,
}

impl GDT {
    /// Creates a new `GDT` with 3 predefined segment descriptors.
    pub const fn new() -> Self {
        let mut gdt = GDT {
            null_descriptor: NormalSegmentDescriptor::new(),
            code_descriptor: NormalSegmentDescriptor::new(),
            tss_descriptor: SystemSegmentDescriptor::new(),
        };

        // set the type of the code descriptor to 'code/data segment descriptor'
        gdt.code_descriptor.access_byte |= 1 << 4;
        gdt
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
}
