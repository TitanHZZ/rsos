// https://wiki.osdev.org/Global_Descriptor_Table
// https://wiki.osdev.org/GDT_Tutorial
use crate::memory::VirtualAddress;
use core::marker::PhantomData;
use bitflags::bitflags;

bitflags! {
    #[repr(C)]
    struct NormalSegmentAccessByte: u8 {
        const ACCESSED        = 1 << 0;
        const RW              = 1 << 1;
        const DC              = 1 << 2; // Direction bit/Conforming bit
        const EXECUTABLE      = 1 << 3;
        const DESCRIPTOR_TYPE = 1 << 4; // Descriptor type (code/data or system descriptor)
        const DPL_LO          = 1 << 5;
        const DPL_HI          = 1 << 6;
        const PRESENT         = 1 << 7;
    }
}

bitflags! {
    #[repr(C)]
    struct SystemSegmentAccessByte: u8 {
        // const TYPE         = 1 << 0 | 1 << 1 | 1 << 2 | 1 << 3;
        const DESCRIPTOR_TYPE = 1 << 4; // Descriptor type (code/data or system descriptor)
        const DPL_LO          = 1 << 5;
        const DPL_HI          = 1 << 6;
        const PRESENT         = 1 << 7;
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
enum SystemSegmentAccessByteType {
    LDT               = 0x2,
    TssAvailable64bit = 0x9,
    TssBusy64bit      = 0xb,
}

pub trait SegmentDescriptorType {
    type SegmentDescriptorArgs;

    fn set_args<T: SegmentDescriptorType>(desc: &mut SegmentDescriptor<T>, args: Self::SegmentDescriptorArgs);
}

pub struct NormalDescriptor;
pub struct SystemDescriptor;

#[repr(C)]
pub struct NormalDescriptorAccessByteArgs {
    flags: NormalSegmentAccessByte,
}

#[repr(C)]
pub struct SystemDescriptorAccessByteArgs {
    flags: SystemSegmentAccessByte,
    seg_type: SystemSegmentAccessByteType,
}

impl SegmentDescriptorType for NormalDescriptor {
    type SegmentDescriptorArgs = NormalDescriptorAccessByteArgs;

    fn set_args<T: SegmentDescriptorType>(desc: &mut SegmentDescriptor<T>, args: Self::SegmentDescriptorArgs) {
        desc.access_byte = args.flags.bits();
    }
}

impl SegmentDescriptorType for SystemDescriptor {
    type SegmentDescriptorArgs = SystemDescriptorAccessByteArgs;

    fn set_args<T: SegmentDescriptorType>(desc: &mut SegmentDescriptor<T>, args: Self::SegmentDescriptorArgs) {
        desc.access_byte = args.flags.bits();
        desc.access_byte |= args.seg_type as u8;
    }
}

#[repr(C)]
pub struct SegmentDescriptor<T: SegmentDescriptorType> {
    limit_0: u16,
    base_0: u16,
    base_1: u8,
    access_byte: u8, // NormalAccessByte or SystemSegmentAccessByte
    limit_1_and_flags: u8,
    base_2: u8,
    base_3: u32,
    reserved: u32,

    _type: PhantomData<T>,
}

impl<T: SegmentDescriptorType> SegmentDescriptor<T> {
    /// Creates a completly zeroed out SegmentDescriptor.
    pub const fn new() -> Self {
        SegmentDescriptor {
            limit_0: 0,
            base_0: 0,
            base_1: 0,
            access_byte: 0,
            limit_1_and_flags: 0,
            base_2: 0,
            base_3: 0,
            reserved: 0,

            _type: PhantomData,
        }
    }

    /// Sets the `limit`. Please keep in mind that the limit is just 20bits so the top 12bits of this u32 will be ignored.
    pub fn set_limit(&mut self, limit: u32) {
        self.limit_0 = (limit & 0xFFFF) as u16;
        self.limit_1_and_flags = (self.limit_1_and_flags & 0xF0) | ((limit >> 16) & 0x0F) as u8;
    }

    /// Sets the full 64bit base `VirtualAddress`.
    pub fn set_base(&mut self, base: VirtualAddress) {
        self.base_0 = ((base >> 00) & 0x0000_FFFF) as u16;
        self.base_1 = ((base >> 16) & 0x0000_00FF) as u8;
        self.base_2 = ((base >> 24) & 0x0000_00FF) as u8;
        self.base_3 = ((base >> 32) & 0xFFFF_FFFF) as u32;
    }

    pub fn set_access_byte(&mut self, args: T::SegmentDescriptorArgs) {
        T::set_args(self, args);
    }

    pub fn set_flags(&mut self, flags: SegmentFlags) {
        self.limit_1_and_flags = (self.limit_1_and_flags & 0x0F) | (flags.bits() << 4);
    }
}

#[repr(C)]
pub struct GDT {
    null_descriptor: SegmentDescriptor<NormalDescriptor>,
    pub code_segment: SegmentDescriptor<NormalDescriptor>,
    pub tss_segment: SegmentDescriptor<SystemDescriptor>,
}

impl GDT {
    pub const fn new() -> Self {
        GDT {
            null_descriptor: SegmentDescriptor::new(),
            code_segment: SegmentDescriptor::new(),
            tss_segment: SegmentDescriptor::new(),
        }
    }
}
