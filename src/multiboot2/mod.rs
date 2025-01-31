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

use tag_iter::MbTagIter;
use tag_trait::MbTag;

#[repr(C)]
#[derive(Clone)]
struct MbBootInformationHeader {
    total_size: u32,
    reserved: u32,
    // followed by the tags
}

#[repr(C)]
pub(crate) struct MbTagHeader {
    tag_type: TagType,
    size: u32,
}

#[repr(u32)]
#[derive(PartialEq)]
pub(crate) enum TagType {
    End = 0,
    CmdLine = 1,
    BootLoaderName = 2,
    Modules = 3,
    BasicMemoryInfo = 4,
    BiosBootDevice = 5,
    MemoryMap = 6,
    VbeInfo = 7,
    FrameBufferInfo = 8,
    ElfSymbols = 9,
    ApmTable = 10,
    Efi32BitSystemTablePtr = 11,
    Efi64BitSystemTablePtr = 12,
    SmBiosTables = 13,
    AcpiOldRsdp = 14,
    AcpiNewRsdp = 15,
    NetworkingInfo = 16,
    EfiMemoryMap = 17,
    EfiBootServicesNotTerminated = 18,
    Efi32BitImageHandlePtr = 19,
    Efi64BitImageHandlePtr = 20,
    ImageLoadBasePhysicalAdress = 21,
}

impl MbTagHeader {
    fn cast_to<T: MbTag + ?Sized>(&self) -> &T {
        // Safety: At this point, we take the data as being valid as it was already checked.
        unsafe { MbTag::from_base_tag(self) }
    }
}

#[repr(C)]
pub(crate) struct MbBootInfo {
    header: MbBootInformationHeader,

    /*
     * Not using NonNull<MbTagHeader> as it expects mut ptrs.
     */
    tags_ptr: *const MbTagHeader,
}

// TODO: make an enum with all the possible errors and return that on the Results instead of the &strs
impl MbBootInfo {
    pub unsafe fn new(mb_boot_info: *const u8) -> Result<Self, &'static str> {
        // make sure that the ptr is not null
        if mb_boot_info.is_null() {
            return Err("`mb_boot_info` is null!");
        }

        // make sure that the pointer is aligned to 64 bits
        if mb_boot_info.align_offset(size_of::<u64>()) != 0 {
            return Err("`mb_boot_info` is not aligned to 64bits!");
        }

        let mb_header: &MbBootInformationHeader = &*mb_boot_info.cast();    
        let tags_ptr: *const MbTagHeader = mb_boot_info.offset(size_of::<MbBootInformationHeader>() as isize).cast();

        Ok(Self {
            header: mb_header.clone(),
            tags_ptr
        })
    }

    fn tags (&self) -> MbTagIter {
        MbTagIter::new(self.tags_ptr)
    }

    pub fn get_tag<T: MbTag + ?Sized>(&self) -> Option<&T> {
        self.tags()
            .find(|tag| tag.tag_type == T::TAG_TYPE)
            .map(|tag| tag.cast_to::<T>())
    }
}

// // TODO: mark this as unsafe
// // TODO: remove the unwrap()s from the str creations
// // TODO: add checks to make sure that the header tag has the correct size (where possible)
// pub fn mb_test(mb_boot_info_addr: *const u8) {
//     let mb_header = unsafe { &*(mb_boot_info_addr as *const MbBootInformationHeader) };
//     let size = mb_header.total_size;
//     // println!("Boot info total size: {}", size);
//     // println!("Reserved: {}", mb_header.reserved);
//     let mut tag_addr = unsafe { mb_boot_info_addr.offset(size_of::<u64>() as isize) };
//     let mut tag = unsafe { &*(tag_addr as *const MbTagHeader) };
//     loop {
//         match tag.tag_type {
//             TagType::End => break,
//             // TagType::CmdLine => {
//             //     // construct the cmd line tag from raw bytes
//             //     let ptr = tag as *const MbTagHeader as *const u8;
//             //     let bytes: *const [u8] = slice_from_raw_parts(ptr, tag.size as usize);
//             //     let cmd_line = unsafe { &*(bytes as *const CmdLine) };
//             //     // calculate the real str size, convert [u8] to &str and print it
//             //     let str_len = tag.size as usize - size_of::<MbTagHeader>() - 1;
//             //     let str = from_utf8(&cmd_line.string[..str_len]).unwrap();
//             //     // println!("Got CmdLine tag:\n    cmdline: '{}'", str);
//             // }
//             // TagType::BootLoaderName => {
//             //     // construct the bootloader name tag from raw bytes
//             //     let ptr = tag as *const MbTagHeader as *const u8;
//             //     let bytes: *const [u8] = slice_from_raw_parts(ptr, tag.size as usize);
//             //     let bootloader_name = unsafe { &*(bytes as *const BootLoaderName) };
//             //     // calculate the real str size, convert [u8] to &str and print it
//             //     let str_len = tag.size as usize - size_of::<MbTagHeader>() - 1;
//             //     let str = from_utf8(&bootloader_name.string[..str_len]).unwrap();
//             //     // println!("Got BootLoaderName tag:\n    bootloader name: '{}'", str);
//             // }
//             // TagType::Modules => {
//             //     // construct the modules tag from raw bytes
//             //     let ptr = tag as *const MbTagHeader as *const u8;
//             //     let bytes = slice_from_raw_parts(ptr, tag.size as usize);
//             //     let modules = unsafe { &*(bytes as *const Modules) };
//             //     let str_len = tag.size as usize - size_of::<MbTagHeader>() - size_of::<u64>() - 1;
//             //     let str = from_utf8(&modules.string[..str_len]).unwrap();
//             //     // println!("Got Modules tag:\n    mod_start: {}\n    mod_end: {}\n    string: '{}'", modules.mod_start, modules.mod_end, str);
//             // }
//             // TagType::BasicMemoryInfo => {
//             //     // construct the basic mem info tag from the headet tag
//             //     let basic_mem_info = unsafe { &*(tag as *const MbTagHeader as *const BasicMemoryInfo) };
//             //     // println!("Got BasicMemoryInfo tag:\n    mem_lower: {}\n    mem_upper: {}", basic_mem_info.mem_lower, basic_mem_info.mem_upper);
//             // }
//             // TagType::BiosBootDevice => {
//             //     // construct the bios boot device tag from the header tag
//             //     let bios_boot_device = unsafe { &*(tag as *const MbTagHeader as *const BiosBootDevice) };
//             //     // println!("Got BiosBootDevice tag:\n    biosdev: {}\n    partition: {}\n    sub_partition: {}",
//             //     //     bios_boot_device.biosdev, bios_boot_device.partition, bios_boot_device.sub_partition
//             //     // );
//             // }
//             // TagType::MemoryMap => {
//             //     // construct the memory map tag from raw bytes
//             //     let ptr = tag as *const MbTagHeader as *const u8;
//             //     let bytes = slice_from_raw_parts(ptr, tag.size as usize);
//             //     let memory_map = unsafe { &*(bytes as *const MemoryMap) };
//             //     // TODO: make sure that:
//             //     // size_of::<MemoryMapEntry> == memory_map.entry_size
//             //     // memory_map.entry_size % 8 == 0 (is multiple of 8)
//             //     // enum for the entry types
//             //     let entry_count = (tag.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 2) / size_of::<MemoryMapEntry>();
//             //     // println!("Got MemoryMap tag:");
//             //     // for entry_idx in 0..entry_count {
//             //     //     let entry = &memory_map.entries[entry_idx];
//             //     //     println!("    base_addr: {}, length: {}, type: {}, reserved: {}", entry.base_addr, entry.length, entry.entry_type, entry.reserved);
//             //     // }
//             // }
//             // /*
//             //  * Not exactly sure how to print vbe_control_info and vbe_mode_info
//             //  * Also, EFI systems (including this one) should not have this tag AFAIK
//             //  */
//             // TagType::VbeInfo => {
//             //     // construct the vbe info tag from the header tag
//             //     let vbe_info = unsafe { &*(tag as * const MbTagHeader as *const VbeInfo) };
//             //     // println!("Got VbeInfo tag!");
//             // }
//             // TagType::ElfSymbols => {
//             //     // construct the elf symbols tag from raw bytes
//             //     let ptr = tag as *const MbTagHeader as *const u8;
//             //     let bytes = slice_from_raw_parts(ptr, tag.size as usize);
//             //     let elf_symbols = unsafe { &*(bytes as *const ElfSymbols) };
//             //     // construct the elf sections from raw bytes
//             //     let section_headers_ptr: *const ElfSectionHeader = &elf_symbols.section_headers as *const [u8] as *const u8 as *const _;
//             //     let elf_sections = slice_from_raw_parts(section_headers_ptr, elf_symbols.num as usize);
//             //     let elf_sections = unsafe { &*(elf_sections as *const [ElfSectionHeader]) };
//             //     // println!("Got ElfSymbols tag:\n    num: {}", elf_symbols.num);
//             //     // for section in elf_sections {
//             //     //     println!("is unused = {}", section.section_type == ElfSectionType::Unused);
//             //     // }
//             // }
//             // TagType::ApmTable => {
//             //     // construct the apm table tag from the header tag
//             //     let apm_table = unsafe { &*(tag as *const MbTagHeader as *const ApmTable) };
//             //     // println!("Got ApmTable tag!");
//             // }
//             /*
//              * I don't think this tag will even exist in this context as the kernel is 64bit.
//              * Maybe it could be removed?
//              */
//             // TagType::Efi32BitSystemTablePtr => {
//             //     // construct the efi 32 bit system table ptr tag from the header tag
//             //     let efi32_system_table_ptr = unsafe { &*(tag as *const MbTagHeader as *const Efi32BitSystemTablePtr) };
//             //     println!("Got Efi32BitSystemTablePtr tag!\n    pointer: {}", efi32_system_table_ptr.pointer);
//             // }
//             // TagType::Efi64BitSystemTablePtr => {
//             //     // construct the efi 64 bit system table ptr tag from the header tag
//             //     let efi64_system_table_ptr = unsafe { &*(tag as *const MbTagHeader as *const Efi64BitSystemTablePtr) };
//             //     println!("Got Efi64BitSystemTablePtr tag!\n    pointer: {}", efi64_system_table_ptr.pointer);
//             // }
//             // TagType::EfiBootServicesNotTerminated => {
//             //     // construct the efi boot services not terminated tag from the header tag
//             //     let _efi_boot_services_not_terminated: &EfiBootServicesNotTerminated = unsafe { &*(tag as *const MbTagHeader as *const _) };
//             //     println!("Got EfiBootServicesNotTerminated tag!\n    This means that ExitBootServices wasn't called.");
//             // }
//             /*
//              * I don't think this tag will even exist in this context as the kernel is 64bit.
//              * Maybe it could be removed?
//              */
//             // TagType::Efi32BitImageHandlePtr => {
//             //     // construct the efi 32 bit image handle ptr tag from the header tag
//             //     let efi_32_image_handle_ptr = unsafe { &*(tag as *const MbTagHeader as *const Efi32BitImageHandlePtr) };
//             //     println!("Got Efi32BitImageHandlePtr tag!\n    pointer: {}", efi_32_image_handle_ptr.pointer);
//             // }
//             // TagType::Efi64BitImageHandlePtr => {
//             //     // construct the efi 64 bit image handle ptr tag from the header tag
//             //     let efi_64_image_handle_ptr = unsafe { &*(tag as *const MbTagHeader as *const Efi64BitImageHandlePtr) };
//             //     println!("Got Efi64BitImageHandlePtr tag!\n    pointer: {}", efi_64_image_handle_ptr.pointer);
//             // }
//             // TagType::ImageLoadBasePhysicalAdress => {
//             //     // construct the image load base physical adress tag from the header tag
//             //     let image_load_base_physical_adress: &ImageLoadBasePhysicalAdress = unsafe { &*(tag as *const MbTagHeader as *const _) };
//             //     // println!("Got ImageLoadBasePhysicalAdress tag!\n    load base addr: {}", image_load_base_physical_adress.load_base_addr);
//             // }
//             _ => {}
//         }
//         // go to the next tag
//         tag_addr = unsafe { tag_addr.offset(((tag.size as usize + 7) & !7) as isize) };
//         tag = unsafe { &*(tag_addr as *const MbTagHeader) };
//     }
// }
