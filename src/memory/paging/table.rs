use super::{
    entry::{Entry, EntryFlags},
    ENTRY_COUNT,
};

/*
 * This is the base addr used to modify the Page Tables themselves using recursive mapping.
 * 0o177777_777_777_777_777_0000 = 0xfffffffffffff000
 */
pub const P4: *const Table = 0o177777_777_777_777_777_0000 as *const Table;

/*
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
 * For more information: https://os.phil-opp.com/page-tables/#mapping-page-tables
 * and: https://wiki.osdev.org/User:Neon/Recursive_Paging
 */

pub struct Table {
    entries: [Entry; ENTRY_COUNT],
}

impl Table {
    fn next_table_addr(&self, table_index: usize) -> Option<usize> {
        // get the flags for the entry in the `table_index` pos
        let entry_flags = self.entries[table_index].flags();

        if entry_flags.contains(EntryFlags::PRESENT) && !entry_flags.contains(EntryFlags::HUGE_PAGE)
        {
            let res = self as *const Table as usize;
            return Some((res << 9) | (table_index << 12));
        }

        None
    }
}
