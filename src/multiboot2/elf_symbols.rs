use super::MbTagHeader;
use bitflags::bitflags;

// https://github.com/fabiansperber/multiboot2-elf64/blob/master/README.md
// https://refspecs.linuxfoundation.org/elf/elf.pdf
#[repr(C)]
pub(crate) struct ElfSymbols {
    header: MbTagHeader,
    pub(crate) num: u32, // number of section headers
    entry_size: u32, // size of each section header (needs to be 64 as that is the size of every entry for ELF64)
    string_table: u32,

    /*
     * If this was `section_headers: [ElfSectionHeader]`, it would unalign the sections by 4 bytes
     * as thus, make the reading completly wrong.
     * This means that the sections will all be unaligned by 4 bytes (but this is not a problem).
     * 
     * Perhaps this could be done with `#[repr(C, packed)]`?
     */
    pub(crate) section_headers: [u8],
}

#[repr(C)]
pub(crate) struct ElfSectionHeader {
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
