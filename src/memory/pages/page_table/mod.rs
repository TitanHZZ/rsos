pub mod page_table_entry;

use crate::memory::{frames::FrameAllocator, MemoryError, FRAME_PAGE_SIZE};
use page_table_entry::{Entry, EntryFlags};
use core::marker::PhantomData;

pub const ENTRY_COUNT: usize = 512; // 512 = 2^9 = log2(PAGE_SIZE), PAGE_SIZE = 4096

/*
 * This is the base addr used to modify the Page Tables themselves using recursive mapping:
 * 0o177777_777_777_777_777_0000 = 0xfffffffffffff000
 * 0o177777 is just the extension to 64 bits
 *
 * This are the addresses that must be used to access the page tables themselves.
 * +-------+-------------------------------+-------------------------------------+
 * | Table | Address                       | Indexes                             |
 * | P4    | 0o177777_777_777_777_777_0000 | –                                   |
 * | P3    | 0o177777_777_777_777_XXX_0000 | XXX is the P4 index                 |
 * | P2    | 0o177777_777_777_XXX_YYY_0000 | like above, and YYY is the P3 index |
 * | P1    | 0o177777_777_XXX_YYY_ZZZ_0000 | like above, and ZZZ is the P2 index |
 * +-------+-------------------------------+-------------------------------------+
 * As it can be seen, the addresses may be calculated with the following formula:
 * next_table_address = (table_address << 9) | (index << 12)
 * The `_0000` at the end of the addrs means that they are page table aligned and
 * may be used as indexes to read/write from/to a page table.
 * For more information:
 *  - https://os.phil-opp.com/page-tables/#mapping-page-tables
 *  - https://wiki.osdev.org/User:Neon/Recursive_Paging
 */
pub const P4: *mut Table<Level4> = 0o177777_777_777_777_777_0000 as *mut _;

pub trait TableLevel {}
pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

pub trait HierarchicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

impl HierarchicalLevel for Level4 {
    type NextLevel = Level3;
}

impl HierarchicalLevel for Level3 {
    type NextLevel = Level2;
}

impl HierarchicalLevel for Level2 {
    type NextLevel = Level1;
}

pub struct Table<L: TableLevel> {
    pub entries: [Entry; ENTRY_COUNT],
    _level: PhantomData<L>,
}

impl<L: TableLevel> Table<L> {
    pub fn set_unused(&mut self) {
        for entry in &mut self.entries {
            entry.set_unused();
        }
    }
}

impl<L: HierarchicalLevel> Table<L> {
    fn next_table_addr(&self, table_index: usize) -> Option<usize> {
        // index must be between 0 and ENTRY_COUNT
        if table_index >= ENTRY_COUNT {
            return None;
        }

        let entry_flags = self.entries[table_index].flags();
        if entry_flags.contains(EntryFlags::PRESENT) && !entry_flags.contains(EntryFlags::HUGE_PAGE) {
            let addr = self as *const _ as usize;
            return Some((addr << 9) | (table_index << 12)); // see comment at the top
        }

        None
    }

    pub fn next_table(&self, table_index: usize) -> Option<&Table<L::NextLevel>> {
        Some(unsafe { &*(self.next_table_addr(table_index)? as *const _) })
    }

    pub fn next_table_mut(&self, table_index: usize) -> Option<&mut Table<L::NextLevel>> {
        Some(unsafe { &mut *(self.next_table_addr(table_index)? as *mut _) })
    }

    /*
     * This function will always create a standard 4KB page as huge pages are not supported.
     */
    pub fn create_next_table<A: FrameAllocator>(&mut self, table_index: usize, frame_allocator: &mut A) -> Result<&mut Table<L::NextLevel>, MemoryError> {
        // check if page table is already allocated
        if self.next_table(table_index).is_none() {
            // page table is not yet created so allocate a new frame to hold the new page table
            let frame = frame_allocator.allocate_frame()?;

            // set the new entry
            self.entries[table_index].set(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);

            // this unwrap() *should* never fail as we just set the entry above
            self.next_table_mut(table_index).unwrap().set_unused();
        }

        // at this point, we *should* have a valid entry at `table_index` so this unwrap() *should* be fine
        Ok(self.next_table_mut(table_index).unwrap())
    }
}
