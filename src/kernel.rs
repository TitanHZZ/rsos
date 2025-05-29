use crate::{memory::{AddrOps, VirtualAddress, FRAME_PAGE_SIZE}, multiboot2::{elf_symbols::{ElfSectionFlags, ElfSymbols, ElfSymbolsIter}, MbBootInfo}};

// TODO: this should probably be a static and hold a mutex
pub struct Kernel {
    // kernel dimensions
    k_start: usize,
    k_end: usize,

    // multiboot2
    mb_info: MbBootInfo,
    mb_start: usize,
    mb_end: usize,
}

impl Kernel {
    // TODO: this could return a Result<>
    pub fn new(mb_info: MbBootInfo) -> Self {
        // get the necessary mb2 tags and data
        // let mem_map: &MemoryMap          = mb_info.get_tag::<MemoryMap>().expect("Memory map tag is not present");
        let elf_symbols: &ElfSymbols     = mb_info.get_tag::<ElfSymbols>().expect("Elf symbols tag is not present");
        let elf_sections: ElfSymbolsIter = elf_symbols.sections().expect("Elf sections are invalid");

        // get the kernel start and end addrs
        let k_start = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
            .map(|s: _| s.addr()).min().expect("Elf sections is empty").align_down(FRAME_PAGE_SIZE);

        let k_end   = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
            .map(|s: _| s.addr() + s.size() as usize).max().expect("Elf sections is empty").align_up(FRAME_PAGE_SIZE) - 1;

        // get the mb2 info start and end addrs
        let mb_start = mb_info.addr().align_down(FRAME_PAGE_SIZE);
        let mb_end   = mb_info.addr() + mb_info.size() as usize - 1;
        let mb_end   = mb_end.align_up(FRAME_PAGE_SIZE) - 1;

        Kernel {
            k_start,
            k_end,

            mb_info,
            mb_start,
            mb_end,
        }
    }

    pub fn k_start(&self) -> VirtualAddress {
        self.k_start
    }

    pub fn k_end(&self) -> VirtualAddress {
        self.k_end
    }

    pub fn mb_info(&self) -> &MbBootInfo {
        &self.mb_info
    }

    pub fn mb_start(&self) -> VirtualAddress {
        self.mb_start
    }

    pub fn mb_end(&self) -> VirtualAddress {
        self.mb_end
    }
}
