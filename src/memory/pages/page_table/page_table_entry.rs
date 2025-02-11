use crate::{memory::{frames::Frame, PhysicalAddress}, multiboot2::elf_symbols::ElfSectionFlags};
use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy)]
    pub struct EntryFlags: u64 {
        const PRESENT         = 1 << 0;  // the page is currently in memory
        const WRITABLE        = 1 << 1;  // it’s allowed to write to this page
        const USER_ACCESSIBLE = 1 << 2;  // if not set, only kernel mode code can access this page
        const WRITE_THROUGH   = 1 << 3;  // writes go directly to memory
        const NO_CACHE        = 1 << 4;  // no cache is used for this page
        const ACCESSED        = 1 << 5;  // the CPU sets this bit when this page is used
        const DIRTY           = 1 << 6;  // the CPU sets this bit when a write to this page occurs
        const HUGE_PAGE       = 1 << 7;  // must be 0 in P1 and P4, creates a 1GiB page in P3, creates a 2MiB page in P2
        const GLOBAL          = 1 << 8;  // page isn’t flushed from caches on address space switch (PGE bit of CR4 register must be set)
        const NO_EXECUTE      = 1 << 63; // forbid executing code on this page (the NXE bit in the EFER register must be set)
    }
}

/*
 * An entry in a page table is an addr with some flags.
 * That´s why this is not an addr and instead, a u64.
 * Also, an entry is exactly 64 bits (u64) and not usize.
 */
#[derive(Clone, Copy)]
pub struct Entry(u64);

impl Entry {
    pub fn is_used(&self) -> bool {
        // an entry equal to 0 is unused otherwise, it´s used
        self.0 != 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn phy_addr(&self) -> Option<PhysicalAddress> {
        if self.flags().contains(EntryFlags::PRESENT) {
            return Some((self.0 & 0x000fffff_fffff000) as PhysicalAddress);
        }

        None
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        Some(Frame::from_phy_addr(self.phy_addr()?))
    }

    pub fn set_flags(&mut self, flags: EntryFlags) {
        self.0 = (self.0 & 0x000fffff_fffff000) | flags.bits();
    }

    pub fn set_phy_addr(&mut self, frame: Frame) {
        self.0 = (self.0 & !0x000fffff_fffff000) | frame.addr() as u64;
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        self.set_phy_addr(frame);
        self.set_flags(flags);
    }
}

impl EntryFlags {
    pub fn from_elf_section_flags(section_flags: ElfSectionFlags) -> Self {
        let mut flags = EntryFlags::empty();

        if section_flags.contains(ElfSectionFlags::ELF_SECTION_WRITABLE) {
            flags |= EntryFlags::WRITABLE;
        }

        if section_flags.contains(ElfSectionFlags::ELF_SECTION_ALLOCATED) {
            flags |= EntryFlags::PRESENT;
        }

        if !section_flags.contains(ElfSectionFlags::ELF_SECTION_EXECUTABLE) {
            flags |= EntryFlags::NO_EXECUTE;
        }

        flags
    }
}
