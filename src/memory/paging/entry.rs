use bitflags::bitflags;

pub struct Entry(u64);

impl Entry {
    fn is_used(&self) -> bool {
        /*
         * An entry equal to 0 is unused otherwise, it´s used.
         */
        self.0 != 0
    }

    fn set_unused(&mut self) {
        self.0 = 0;
    }
}

bitflags! {
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
