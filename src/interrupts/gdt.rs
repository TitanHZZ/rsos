// https://wiki.osdev.org/Global_Descriptor_Table
// https://wiki.osdev.org/GDT_Tutorial
use crate::memory::VirtualAddress;
use bitflags::bitflags;
use core::{arch::asm};
use super::tss::TSS;

/// Reloads all segment registers: cs, ss, ds, es, fs and gs.
/// 
/// The cs register will have the value of `code_sel` and the rest of the registers will be set to 0.
/// 
/// # Safety
/// 
/// The caller must ensure that `code_sel` is a valid segment selector and that the GDT is valid and correctly loaded.
// https://wiki.osdev.org/GDT_Tutorial#Long_Mode_2
pub unsafe fn reload_seg_regs(code_sel: SegmentSelector) {
    unsafe {
        asm!(
            "push {sel}",             // Push code segment to stack, 0x08 is a stand-in for your code segment
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
            sel = in(reg) code_sel.as_u16() as u64,
            tmp = lateout(reg) _,
        );
    }
}

bitflags! {
    #[repr(C)]
    pub struct NormalDescAccessByte: u8 {
        const ACCESSED        = 1 << 0;
        const RW              = 1 << 1;
        const DC              = 1 << 2; // Direction bit/Conforming bit
        const EXECUTABLE      = 1 << 3;
        const IS_CODE_OR_DATA = 1 << 4; // Descriptor type (code/data or system descriptor)
        const DPL_LO          = 1 << 5;
        const DPL_HI          = 1 << 6;
        const PRESENT         = 1 << 7;
    }
}

bitflags! {
    #[repr(C)]
    pub struct SystemDescAccessByte: u8 {
        // const TYPE         = 1 << 0 | 1 << 1 | 1 << 2 | 1 << 3;
        const IS_CODE_OR_DATA = 1 << 4; // Descriptor type (code/data or system descriptor)
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

impl Default for NormalSegmentDescriptor {
    fn default() -> Self {
        Self::new()
    }
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

impl Default for SystemSegmentDescriptor {
    fn default() -> Self {
        Self::new()
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

    #[allow(clippy::identity_op)]
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
// TODO: it might make sense to add support for TI's and RPL's != 0
#[repr(C)]
pub struct SegmentSelector {
    selector: u16,
}

impl SegmentSelector {
    pub fn as_u16(&self) -> u16 {
        self.selector
    }
}

#[repr(C, packed)]
#[allow(clippy::upper_case_acronyms)]
struct GDTR {
    size: u16,
    offset: u64,
}

#[derive(Debug)]
pub enum GdtError {
    NotEnoughGdtSpace,
}

impl Default for GDT {
    fn default() -> Self {
        Self::new()
    }
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

    /// Adds `desc` to the GDT and returs a SegmentSelector that points to the newly added descriptor.
    pub fn new_descriptor(&mut self, desc: Descriptor) -> Result<SegmentSelector, GdtError> {
        match desc {
            Descriptor::NormalDescriptor(n_desc) => {
                // make sure that the max limit is not violated
                if self.normal_desc_count >= 5 {
                    return Err(GdtError::NotEnoughGdtSpace);
                }

                let gdt_entry: u64 = n_desc.limit_0 as u64
                    | (n_desc.base_0 as u64) << 16
                    | (n_desc.base_1 as u64) << 32
                    | (n_desc.access_byte as u64) << 40
                    | (n_desc.limit_1_and_flags as u64) << 48
                    | (n_desc.base_2 as u64) << 56;

                let gdt_offset: usize = self.normal_desc_count as usize + self.system_desc_count as usize * 2;
                debug_assert!(gdt_offset <= 6);

                self.normal_desc_count += 1;
                self.descriptors[gdt_offset] = gdt_entry;
                Ok(SegmentSelector {
                    selector: (gdt_offset * 8) as u16,
                })
            },
            Descriptor::SystemDescriptor(s_desc) => {
                // make sure that the max limit id not violated
                if self.system_desc_count >= 1 {
                    return Err(GdtError::NotEnoughGdtSpace);
                }

                let gdt_entry_lo = s_desc.normal_desc.limit_0 as u64
                    | (s_desc.normal_desc.base_0 as u64) << 16
                    | (s_desc.normal_desc.base_1 as u64) << 32
                    | (s_desc.normal_desc.access_byte as u64) << 40
                    | (s_desc.normal_desc.limit_1_and_flags as u64) << 48
                    | (s_desc.normal_desc.base_2 as u64) << 56;

                let gdt_entry_hi = s_desc.base_3 as u64 | (s_desc.reserved as u64) << 32;

                let gdt_offset: usize = self.normal_desc_count as usize + self.system_desc_count as usize * 2;
                debug_assert!(gdt_offset <= 5);

                self.system_desc_count += 1;
                self.descriptors[gdt_offset] = gdt_entry_lo;
                self.descriptors[gdt_offset + 1] = gdt_entry_hi;
                Ok(SegmentSelector {
                    selector: (gdt_offset * 8) as u16,
                })
            }
        }
    }

    /// Loads `slf` as the current GDT.
    /// 
    /// This does not reload segment registers altough it is necessary to do so with `reload_seg_regs()`.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that `slf` is valid.
    pub unsafe fn load(slf: &'static Self) {
        let gdtr = GDTR {
            size: ((slf.normal_desc_count + slf.system_desc_count * 2) * 8 - 1) as u16,
            offset: slf as *const GDT as u64,
        };

        unsafe {
            asm!("lgdt [{}]", in(reg) &gdtr, options(nostack, preserves_flags));
        }
    }
}
