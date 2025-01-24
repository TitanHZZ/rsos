// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
// https://wiki.osdev.org/Multiboot
use crate::{print, println};
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

#[repr(C)]
struct ElfSymbols {
    header: MbTagHeader,
    num: u16,
    entsize: u16,
    shndx: u16, // string table
    reserved: u16,
    section_headers: [u8],
}

#[repr(C)]
pub struct ElfSectionHeader {
    inner: *const u8,
    string_section: *const u8,
    entry_size: u32,
}

#[repr(C)]
struct ElfSectionInner32 {
    name_index: u32,
    section_type: u32,
    flags: u32,
    addr: u32,
    offset: u32,
    size: u32,
    link: u32,
    info: u32,
    addralign: u32,
    entry_size: u32,
}

#[repr(C)]
struct ElfSectionInner64 {
    name_index: u32,
    section_type: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entry_size: u64,
}

// TODO: mark this as unsafe
// TODO: remove the unwrap()s from the str creations
pub fn mb_test(mb_boot_info_addr: usize) {
    let mb_header = unsafe { &*(mb_boot_info_addr as *const MbBootInformationHeader) };
    let size = mb_header.total_size;

    println!("Boot info total size: {}", size);
    println!("Reserved: {}", mb_header.reserved);

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

                println!("Got CmdLine tag:\n    cmdline: '{}'", str);
            }
            TagType::BootLoaderName => {
                // construct the bootloader name tag from raw bytes
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes: *const [u8] = slice_from_raw_parts(ptr, tag.size as usize);
                let bootloader_name = unsafe { &*(bytes as *const BootLoaderName) };

                // calculate the real str size, convert [u8] to &str and print it
                let str_len = tag.size as usize - size_of::<MbTagHeader>() - 1;
                let str = from_utf8(&bootloader_name.string[..str_len]).unwrap();

                println!("Got BootLoaderName tag:\n    bootloader name: '{}'", str);
            }
            TagType::Modules => {
                // construct the modules tag from raw bytes
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes = slice_from_raw_parts(ptr, tag.size as usize);
                let modules = unsafe { &*(bytes as *const Modules) };

                let str_len = tag.size as usize - size_of::<MbTagHeader>() - size_of::<u64>() - 1;
                let str = from_utf8(&modules.string[..str_len]).unwrap();

                println!("Got Modules tag:\n    mod_start: {}\n    mod_end: {}\n    string: '{}'", modules.mod_start, modules.mod_end, str);
            }
            TagType::BasicMemoryInfo => {
                // construct the basic mem info tag from the headet tag
                let basic_mem_info = unsafe { &*(tag as *const MbTagHeader as *const BasicMemoryInfo) };

                println!("Got BasicMemoryInfo tag:\n    mem_lower: {}\n    mem_upper: {}", basic_mem_info.mem_lower, basic_mem_info.mem_upper);
            }
            TagType::BiosBootDevice => {
                // construct the bios boot device tag from the header tag
                let bios_boot_device = unsafe { &*(tag as *const MbTagHeader as *const BiosBootDevice) };

                println!("Got BiosBootDevice tag:\n    biosdev: {}\n    partition: {}\n    sub_partition: {}",
                    bios_boot_device.biosdev, bios_boot_device.partition, bios_boot_device.sub_partition
                );
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
                let entry_count = (tag.size as usize - size_of::<MbTagHeader>() - size_of::<u64>()) / size_of::<MemoryMapEntry>();
 
                println!("Got MemoryMap tag:");
                for entry_idx in 0..entry_count {
                    let entry = &memory_map.entries[entry_idx];
                    println!("    base_addr: {}, length: {}, type: {}, reserved: {}", entry.base_addr, entry.length, entry.entry_type, entry.reserved);
                }
            }
            /*
             * Not exactly sure how to print vbe_control_info and vbe_mode_info
             * Also, EFI systems (including this one) should not have this tag AFAIK
             */
            TagType::VbeInfo => {
                // construct the vbe info tag from the header tag
                let _vbe_info = unsafe { &*(tag as * const MbTagHeader as *const VbeInfo) };
                println!("Got VbeInfo tag!");
            }
            TagType::ElfSymbols => {
                let ptr = tag as *const MbTagHeader as *const u8;
                let bytes = slice_from_raw_parts(ptr, tag.size as usize);
                let elf_symbols = unsafe { &*(bytes as *const ElfSymbols) };

                println!("Got ElfSymbols tag:\n    section count: {} {}", elf_symbols.num, elf_symbols.entsize);
            }
            _ => {}
        }

        // go to the next tag
        tag_addr = tag_addr + ((tag.size as usize + 7) & !7);
        tag = unsafe { &*(tag_addr as *const MbTagHeader) };
    }

    // while tag.tag_type != TagType::End {
    //     if tag.tag_type == TagType::CmdLine {
    //         // construct the cmd line tag from raw bytes
    //         let ptr = tag as *const MbTagHeader as *const u8;
    //         let bytes: *const [u8] = slice_from_raw_parts(ptr, tag.size as usize);
    //         let cmd_line = unsafe { &*(bytes as *const CmdLine) };
    //         // calculate the real str size, convert [u8] to &str and print it
    //         let str_len = tag.size as usize - size_of::<MbTagHeader>() - 1;
    //         let str = from_utf8(&cmd_line.string[..str_len]).unwrap();
    //         println!("cmdline: {}", str);
    //     }
    //     // go to the next tag
    //     tag_addr = tag_addr + ((tag.size as usize + 7) & !7);
    //     tag = unsafe { &*(tag_addr as *const MbTagHeader) };
    // }
}
