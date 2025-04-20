// https://github.com/fabiansperber/multiboot2-elf64/blob/master/README.md
// https://refspecs.linuxfoundation.org/elf/elf.pdf
use crate::memory::PhysicalAddress;

use super::{tag_trait::MbTag, MbTagHeader, TagType};
use core::ptr::slice_from_raw_parts;
use bitflags::bitflags;
use core::ffi::CStr;

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub struct ElfSymbols {
    header: MbTagHeader,
    pub num: u32, // number of section headers
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

pub struct ElfSection {
    header: &'static ElfSectionHeader,
    string_table: &'static ElfSectionHeader,
}

#[repr(u32)]
#[derive(Debug)]
pub enum ElfSectionType {
    Unused,
    ProgramSection,
    LinkerSymbolTable,
    StringTable,
    RelaRelocation,
    SymbolHashTable,
    DynamicLinkingTable,
    Note,
    Uninitialized,
    RelRelocation,
    Reserved,
    DynamicLoaderSymbolTable,
    Unknown,                  // this enum does not cover the entire u32 range
    EnvironmentSpecific(u32), // from 0x60000000 to 0x6FFFFFFF
    ProcessorSpecific(u32),   // from 0x70000000 to 0x7FFFFFFF
}

bitflags! {
    #[derive(Debug)]
    pub struct ElfSectionFlags: u64 {
        const ELF_SECTION_WRITABLE   = 0x00000001; // section contains data that is writable
        const ELF_SECTION_ALLOCATED  = 0x00000002; // section is in memory during execution
        const ELF_SECTION_EXECUTABLE = 0x00000004; // section contains executable code
        const ENVIRONMENT_SPECIFIC   = 0x0F000000;
        const PROCESSOR_SPECIFIC     = 0xF0000000;
    }
}

#[derive(Debug)]
pub enum ElfSectionError {
    Invalid32BitSectionHeaders,
    StringSectionNotLoaded,
    StringMissingNull,
    StringNotUtf8,
}

impl ElfSymbols {
    // Safety: This assumes that the memory is valid as it *should* only be created by the bootloader and thus,
    // it assumes correct bootloader behavior.
    pub fn sections(&self) -> Result<ElfSymbolsIter, ElfSectionError> {
        if self.entry_size as usize != size_of::<ElfSectionHeader>() { // must be 64bytes
            return Err(ElfSectionError::Invalid32BitSectionHeaders);
        }

        // construct the elf sections from raw bytes
        let section_headers_ptr: *const ElfSectionHeader = &self.section_headers as *const [u8] as *const u8 as *const _;
        let sections = slice_from_raw_parts(section_headers_ptr, self.num as usize);
        let sections = unsafe { &*(sections as *const [ElfSectionHeader]) };

        Ok(ElfSymbolsIter {
            sections,
            curr_section_idx: 0,
            string_table: &sections[self.string_table as usize],
        })
    }
}

impl MbTag for ElfSymbols {
    const TAG_TYPE: TagType = TagType::ElfSymbols;

    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata {
        base_tag.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 3
    }
}

impl ElfSection {
    // Safety: The caller must ensure that the data is valid as this assumes so. This *should* not be a problem
    // as this should only be called by the iter and we assume correct bootloader behavior.
    // The string *should* never leave memory, so it's lifetime is static as it lasts for the entire duration of the program.
    pub fn name(&self) -> Result<&str, ElfSectionError> {
        let strings_ptr = self.string_table.addr as *const u8;
        if strings_ptr.is_null() {
            return Err(ElfSectionError::StringSectionNotLoaded);
        }

        // get a reference to the byte slice containing the string
        let max_string_len = self.string_table.size - self.header.name_index as u64;
        let name_ptr = unsafe { strings_ptr.offset(self.header.name_index as isize) };
        let name_bytes = unsafe { &*slice_from_raw_parts(name_ptr, max_string_len as usize) };

        // convert the cstr to a string slice and return it
        let name_cstr = CStr::from_bytes_until_nul(name_bytes).map_err(|_| ElfSectionError::StringMissingNull)?;
        name_cstr.to_str().map_err(|_| ElfSectionError::StringNotUtf8)
    }

    pub fn section_type(&self) -> ElfSectionType {
        match self.header.section_type {
            0  => ElfSectionType::Unused,
            1  => ElfSectionType::ProgramSection,
            2  => ElfSectionType::LinkerSymbolTable,
            3  => ElfSectionType::StringTable,
            4  => ElfSectionType::RelaRelocation,
            5  => ElfSectionType::SymbolHashTable,
            6  => ElfSectionType::DynamicLinkingTable,
            7  => ElfSectionType::Note,
            8  => ElfSectionType::Uninitialized,
            9  => ElfSectionType::RelRelocation,
            10 => ElfSectionType::Reserved,
            11 => ElfSectionType::DynamicLoaderSymbolTable,
            0x60000000..=0x6FFFFFFF => ElfSectionType::EnvironmentSpecific(self.header.section_type),
            0x70000000..=0x7FFFFFFF => ElfSectionType::ProcessorSpecific(self.header.section_type),
            _  => ElfSectionType::Unknown,
        }
    }

    pub fn flags(&self) -> ElfSectionFlags {
        ElfSectionFlags::from_bits_truncate(self.header.flags)
    }

    pub fn addr(&self) -> PhysicalAddress {
        self.header.addr as _
    }

    pub fn size(&self) -> u64 {
        self.header.size
    }

    pub fn entry_size(&self) -> u64 {
        self.header.entry_size
    }
}

#[derive(Clone, Copy)]
pub struct ElfSymbolsIter {
    sections: &'static [ElfSectionHeader],
    curr_section_idx: usize,
    string_table: &'static ElfSectionHeader,
}

impl Iterator for ElfSymbolsIter {
    type Item = ElfSection;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_section_idx >= self.sections.len() {
            return None;
        }

        // go to the next section and return the current one
        self.curr_section_idx += 1;
        Some(ElfSection {
            header: &self.sections[self.curr_section_idx - 1],
            string_table: &self.string_table,
        })
    }
}
