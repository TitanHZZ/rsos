// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
// https://wiki.osdev.org/Multiboot
pub mod cmd_line;
pub mod tag_iter;
pub mod tag_trait;
pub mod boot_loader_name;
pub mod modules;
pub mod basic_memory_info;
pub mod bios_boot_device;
pub mod memory_map;
pub mod vbe_info;
pub mod elf_symbols;
pub mod apm_table;
pub mod efi_system_table;
pub mod efi_boot_services_not_terminated;
pub mod efi_image_handle;
pub mod image_load_base_phy_addr;
pub mod acpi_new_rsdp;
pub mod framebuffer_info;

use tag_iter::MbTagIter;
use tag_trait::MbTag;

use crate::{memory::{PhysicalAddress, VirtualAddress}};

// TODO: remove BIOS only mb2 tags

#[repr(C)]
#[derive(Clone)]
struct MbBootInformationHeader {
    total_size: u32,
    reserved: u32,
    // followed by the tags
}

#[repr(C)]
pub struct MbTagHeader {
    tag_type: TagType,
    size: u32,
}

#[repr(u32)]
#[derive(PartialEq)]
pub enum TagType {
    End                          = 0,
    CmdLine                      = 1,
    BootLoaderName               = 2,
    Modules                      = 3,
    BasicMemoryInfo              = 4,
    BiosBootDevice               = 5,
    MemoryMap                    = 6,
    VbeInfo                      = 7,
    FrameBufferInfo              = 8,
    ElfSymbols                   = 9,
    ApmTable                     = 10,
    Efi32BitSystemTablePtr       = 11,
    Efi64BitSystemTablePtr       = 12,
    SmBiosTables                 = 13,
    AcpiOldRsdp                  = 14,
    AcpiNewRsdp                  = 15,
    NetworkingInfo               = 16,
    EfiMemoryMap                 = 17,
    EfiBootServicesNotTerminated = 18,
    Efi32BitImageHandlePtr       = 19,
    Efi64BitImageHandlePtr       = 20,
    ImageLoadBasePhysicalAdress  = 21,
}

impl MbTagHeader {
    fn cast_to<T: MbTag + ?Sized>(&self) -> &T {
        // Safety: At this point, we take the data as being valid as it was already checked.
        unsafe { MbTag::from_base_tag(self) }
    }
}

#[repr(C)]
pub struct MbBootInfo {
    header: MbBootInformationHeader,

    /*
     * Not using NonNull<MbTagHeader> as it expects mut ptrs.
     */
    tags_ptr: *const MbTagHeader,
}

#[derive(Debug)]
pub enum MbBootInfoError {
    Not64BitAligned,
    NullPtr,
}

impl MbBootInfo {
    /// # Safety
    /// 
    /// The caller must ensure that `mb_boot_info` is non null and points to a valid Mb2 struct.  
    /// The ptr will be checked for nulls and alignment but no parsing will be performed.
    pub unsafe fn new(mb_boot_info: *const u8) -> Result<Self, MbBootInfoError> {
        // make sure that the ptr is not null
        if mb_boot_info.is_null() {
            return Err(MbBootInfoError::NullPtr);
        }

        // make sure that the pointer is aligned to 64 bits
        if mb_boot_info.align_offset(size_of::<u64>()) != 0 {
            return Err(MbBootInfoError::Not64BitAligned);
        }

        // the `unwrap()` is fine as we know that the value fits
        let mb_header: &MbBootInformationHeader = unsafe { &*mb_boot_info.cast() };
        let tags_ptr: *const MbTagHeader = unsafe { mb_boot_info.offset(size_of::<MbBootInformationHeader>().try_into().unwrap()).cast() };

        Ok(Self {
            header: mb_header.clone(),
            tags_ptr
        })
    }

    pub fn size(&self) -> u32 {
        self.header.total_size
    }

    fn tags (&self) -> MbTagIter {
        MbTagIter::new(self.tags_ptr, self.tags_ptr as VirtualAddress + self.header.total_size as VirtualAddress)
    }

    pub fn get_tag<T: MbTag + ?Sized>(&self) -> Option<&T> {
        self.tags()
            .find(|tag| tag.tag_type == T::TAG_TYPE)
            .map(|tag| tag.cast_to::<T>())
    }

    pub fn addr(&self) -> PhysicalAddress {
        self.tags_ptr as usize - size_of::<MbBootInformationHeader>()
    }
}
