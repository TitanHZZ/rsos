// https://wiki.osdev.org/Global_Descriptor_Table
// https://wiki.osdev.org/GDT_Tutorial
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

#[repr(u8)]
enum SystemSegmentAccessByteType {
    LDT               = 0x2,
    TssAvailable64bit = 0x9,
    TssBusy64bit      = 0xb,
}

trait SegmentDescriptorType {
    type SegmentDescriptorArgs;
}

struct NormalDescriptor;
struct SystemDescriptor;

#[repr(C)]
struct NormalDescriptorAccessByteArgs {
    flags: NormalSegmentAccessByte,
}

#[repr(C)]
struct SystemDescriptorAccessByteArgs {
    flags: SystemSegmentAccessByte,
    seg_type: SystemSegmentAccessByteType,
}

impl SegmentDescriptorType for NormalDescriptor {
    type SegmentDescriptorArgs = NormalDescriptorAccessByteArgs;
}

impl SegmentDescriptorType for SystemDescriptor {
    type SegmentDescriptorArgs = SystemDescriptorAccessByteArgs;
}

#[repr(C)]
struct SegmentDescriptor<T: SegmentDescriptorType> {
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
    const fn new() -> Self {
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

    fn set_access_byte(&mut self, args: T::SegmentDescriptorArgs) {
        todo!();
    }
}

#[repr(C)]
struct GDT {
    null_descriptor: SegmentDescriptor<NormalDescriptor>,
    code_segment: SegmentDescriptor<NormalDescriptor>,
    tss_segment: SegmentDescriptor<SystemDescriptor>,
}

impl GDT {
    fn new() -> Self {
        GDT {
            null_descriptor: SegmentDescriptor::new(),
            code_segment: SegmentDescriptor::new(),
            tss_segment: SegmentDescriptor::new(),
        }
    }
}
