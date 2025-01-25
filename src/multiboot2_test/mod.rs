// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
// https://wiki.osdev.org/Multiboot
use crate::{print, println};
use bitflags::bitflags;
use core::{ptr::slice_from_raw_parts, str::from_utf8};

#[repr(C)]
struct MbBootInformationHeader {
    total_size: u32,
    reserved: u32,
    // followed by the tags
}

#[repr(C)]
struct MbTagHeader {
    tag_type: TagType,
    size: u32,
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(PartialEq)]
enum TagType {
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
    Efi32BitImagehandlePtr = 19,
    Efi64BitImagehandlePtr = 20,
    ImageLoadBasePhysicalAdress = 21,
}

#[repr(C)]
struct CmdLine {
    header: MbTagHeader,
    string: [u8],
}

#[repr(C)]
struct BootLoaderName {
    header: MbTagHeader,
    string: [u8],
}

#[repr(C)]
struct Modules {
    header: MbTagHeader,
    mod_start: u32,
    mod_end: u32,
    string: [u8],
}

#[repr(C)]
struct BasicMemoryInfo {
    header: MbTagHeader,
    mem_lower: u32,
    mem_upper: u32,
}

#[repr(C)]
struct BiosBootDevice {
    header: MbTagHeader,
    biosdev: u32,
    partition: u32,
    sub_partition: u32,
}

#[repr(C)]
struct MemoryMap {
    header: MbTagHeader,
    entry_size: u32,
    entry_version: u32,
    entries: [MemoryMapEntry],
}

#[repr(C)]
struct MemoryMapEntry {
    base_addr: u64,
    length: u64,
    entry_type: u32,
    reserved: u32,
}

#[repr(C)]
struct VbeInfo {
    header: MbTagHeader,
    vbe_mode: u16,
    vbe_interface_seg: u16,
    vbe_interface_off: u16,
    vbe_interface_len: u16,
    vbe_control_info: [u8; 512],
    vbe_mode_info: [u8; 256],
}

// https://github.com/fabiansperber/multiboot2-elf64/blob/master/README.md
// https://refspecs.linuxfoundation.org/elf/elf.pdf
#[repr(C)]
struct ElfSymbols {
    header: MbTagHeader,
    num: u32, // number of section headers
    entry_size: u32, // size of each section header (needs to be 64 as that is the size of every entry for ELF64)
    string_table: u32,

    /*
     * If this was `section_headers: [ElfSectionHeader]`, it would unalign the sections by 4 bytes
     * as thus, make the reading completly wrong.
     * This means that the sections will all be unaligned by 4 bytes (but this is not a problem).
     * 
     * Perhaps this could be done with `#[repr(C, packed)]`?
     */
    section_headers: [u8],
}

#[repr(C)]
struct ElfSectionHeader {
    name_index: u32,
    section_type: ElfSectionType,
    flags: ElfSectionFlags,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entry_size: u64,
}

/*
 * Environment-specific use from 0x60000000 to 0x6FFFFFFF
 * Processor-specific use from 0x70000000 to 0x7FFFFFFF
 */
#[repr(u32)]
#[allow(dead_code)]
#[derive(PartialEq)]
enum ElfSectionType {
    Unused = 0,
    ProgramSection = 1,
    LinkerSymbolTable = 2,
    StringTable = 3,
    RelaRelocation = 4,
    SymbolHashTable = 5,
    DynamicLinkingTable = 6,
    Note = 7,
    Uninitialized = 8,
    RelRelocation = 9,
    Reserved = 10,
    DynamicLoaderSymbolTable = 11,
}

bitflags! {
    /*
     * Environment-specific use at 0x0F000000
     * Processor-specific use at 0xF0000000
     */
    struct ElfSectionFlags: u64 {
        const ELF_SECTION_WRITABLE = 0x1;
        const ELF_SECTION_ALLOCATED = 0x2;
        const ELF_SECTION_EXECUTABLE = 0x4;
    }
}

// TODO: mark this as unsafe
// TODO: remove the unwrap()s from the str creations
pub fn mb_test(mb_boot_info_addr: usize) {
    let mb_header = unsafe { &*(mb_boot_info_addr as *const MbBootInformationHeader) };
    let size = mb_header.total_size;

    // println!("Boot info total size: {}", size);
    // println!("Reserved: {}", mb_header.reserved);

    let mut tag_addr = mb_boot_info_addr + size_of::<u64>();
    let mut tag = unsafe { &*(tag_addr as *const MbTagHeader) };

    loop {
        match tag.tag_type {
            TagType::End => break,
            TagType::CmdLine => {
                // construct the cmd line tag from raw bytes
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes: *const [u8] = slice_from_raw_parts(ptr, tag.size as usize);
                let cmd_line = unsafe { &*(bytes as *const CmdLine) };

                // calculate the real str size, convert [u8] to &str and print it
                let str_len = tag.size as usize - size_of::<MbTagHeader>() - 1;
                let str = from_utf8(&cmd_line.string[..str_len]).unwrap();

                // println!("Got CmdLine tag:\n    cmdline: '{}'", str);
            }
            TagType::BootLoaderName => {
                // construct the bootloader name tag from raw bytes
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes: *const [u8] = slice_from_raw_parts(ptr, tag.size as usize);
                let bootloader_name = unsafe { &*(bytes as *const BootLoaderName) };

                // calculate the real str size, convert [u8] to &str and print it
                let str_len = tag.size as usize - size_of::<MbTagHeader>() - 1;
                let str = from_utf8(&bootloader_name.string[..str_len]).unwrap();

                // println!("Got BootLoaderName tag:\n    bootloader name: '{}'", str);
            }
            TagType::Modules => {
                // construct the modules tag from raw bytes
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes = slice_from_raw_parts(ptr, tag.size as usize);
                let modules = unsafe { &*(bytes as *const Modules) };

                let str_len = tag.size as usize - size_of::<MbTagHeader>() - size_of::<u64>() - 1;
                let str = from_utf8(&modules.string[..str_len]).unwrap();

                // println!("Got Modules tag:\n    mod_start: {}\n    mod_end: {}\n    string: '{}'", modules.mod_start, modules.mod_end, str);
            }
            TagType::BasicMemoryInfo => {
                // construct the basic mem info tag from the headet tag
                let basic_mem_info = unsafe { &*(tag as *const MbTagHeader as *const BasicMemoryInfo) };

                // println!("Got BasicMemoryInfo tag:\n    mem_lower: {}\n    mem_upper: {}", basic_mem_info.mem_lower, basic_mem_info.mem_upper);
            }
            TagType::BiosBootDevice => {
                // construct the bios boot device tag from the header tag
                let bios_boot_device = unsafe { &*(tag as *const MbTagHeader as *const BiosBootDevice) };

                // println!("Got BiosBootDevice tag:\n    biosdev: {}\n    partition: {}\n    sub_partition: {}",
                //     bios_boot_device.biosdev, bios_boot_device.partition, bios_boot_device.sub_partition
                // );
            }
            TagType::MemoryMap => {
                // construct the memory map tag from raw bytes
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes = slice_from_raw_parts(ptr, tag.size as usize);
                let memory_map = unsafe { &*(bytes as *const MemoryMap) };

                // TODO: make sure that:
                // size_of::<MemoryMapEntry> == memory_map.entry_size
                // memory_map.entry_size % 8 == 0 (is multiple of 8)
                // enum for the entry types
                let entry_count = (tag.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 2) / size_of::<MemoryMapEntry>();
 
                // println!("Got MemoryMap tag:");
                // for entry_idx in 0..entry_count {
                //     let entry = &memory_map.entries[entry_idx];
                //     println!("    base_addr: {}, length: {}, type: {}, reserved: {}", entry.base_addr, entry.length, entry.entry_type, entry.reserved);
                // }
            }
            /*
             * Not exactly sure how to print vbe_control_info and vbe_mode_info
             * Also, EFI systems (including this one) should not have this tag AFAIK
             */
            TagType::VbeInfo => {
                // construct the vbe info tag from the header tag
                let _vbe_info = unsafe { &*(tag as * const MbTagHeader as *const VbeInfo) };

                // println!("Got VbeInfo tag!");
            }
            TagType::ElfSymbols => {
                // construct the elf symbols tag from raw bytes
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes = slice_from_raw_parts(ptr, tag.size as usize);
                let elf_symbols = unsafe { &*(bytes as *const ElfSymbols) };

                // construct the elf sections from raw bytes
                let section_headers_ptr: *const ElfSectionHeader = &elf_symbols.section_headers as *const [u8] as *const u8 as *const _;
                let elf_sections = slice_from_raw_parts(section_headers_ptr, elf_symbols.num as usize);
                let elf_sections = unsafe { &*(elf_sections as *const [ElfSectionHeader]) };

                // TODO: --- FINISH THIS TODO ---

                // println!("Got ElfSymbols tag:\n    num: {}", elf_symbols.num);
                // for section in elf_sections {
                //     println!("is unused = {}", section.section_type == ElfSectionType::Unused);
                // }
            }
            _ => {}
        }

        // go to the next tag
        tag_addr = tag_addr + ((tag.size as usize + 7) & !7);
        tag = unsafe { &*(tag_addr as *const MbTagHeader) };
    }
}
