mod entry;
mod table;

use super::{Frame, FrameAllocator, PhysicalAddress, VirtualAddress, PAGE_SIZE};
use core::{marker::PhantomData, ptr::NonNull};
use entry::EntryFlags;
use table::{Level4, Table, P4};

const ENTRY_COUNT: usize = 512; // 512 = 2^9 = log2(PAGE_SIZE), PAGE_SIZE = 4096
pub struct Page(usize); // this usize is the page index in the virtual memory

/* ----------------- SOME NOTES ON PAGE TABLE INDEX CALCULATION -----------------
 * Let´s assume this address: 0xdeadbeef, with 4KiB pages (12 bits)
 * The calculated page index is: 912091 (0xdeadbeef / PAGE_SIZE)
 *
 * Page table indexes to translate the addr:
 * 0xdeadbeef:
 *  p4_idx   -> 0    (0xdeadbeef >> 39 & 0o777)
 *  p3_idx   -> 3    (0xdeadbeef >> 30 & 0o777)
 *  p2_idx   -> 245  (0xdeadbeef >> 21 & 0o777)
 *  p1_idx   -> 219  (0xdeadbeef >> 12 & 0o777)
 *  page_idx -> 239  (0xdeadbeef & 0o777)
 *
 * To calculate this same page table indexes but with the page index instead:
 * idx:
 *  p4_idx   -> 0    (912091 >> (39 - 12) & 0o777)
 *  p3_idx   -> 3    (912091 >> (30 - 12) & 0o777)
 *  p2_idx   -> 245  (912091 >> (21 - 12) & 0o777)
 *  p1_idx   -> 219  (912091 >> (12 - 12) & 0o777)
 *
 * We need to subtract 12 because the page index is 4096 (4KiB) times smaller than the original addr.
 */
impl Page {
    fn corresponding_page(addr: VirtualAddress) -> Page {
        // in x86_64, the top 16 bits of a virtual addr must be sign extension bits
        // if they are not, its an invalid addr
        assert!(
            addr < 0x0000_8000_0000_0000 || addr >= 0xffff_8000_0000_0000,
            "Invalid virtual address: 0x{:x}",
            addr
        );
        Page(addr / PAGE_SIZE)
    }

    fn p4_index(&self) -> usize {
        (self.0 >> 27) & 0o777
    }

    fn p3_index(&self) -> usize {
        (self.0 >> 18) & 0o777
    }

    fn p2_index(&self) -> usize {
        (self.0 >> 9) & 0o777
    }

    fn p1_index(&self) -> usize {
        (self.0 >> 0) & 0o777
    }
}

pub struct Paging {
    p4: NonNull<Table<Level4>>,

    // makes this struct `own` a `Table<Level4>`
    _marker: PhantomData<Table<Level4>>,
}

impl Paging {
    /*
     * Safety: This should be unsafe because the p4 addr will always be the same (at least for now),
     * and that means that creating multiple `Paging` objects could result in undefined behaviour
     * as all the `Paging` objects woulb be pointing to the same memory.
     */
    pub unsafe fn new() -> Self {
        Paging {
            // this can be unchecked as we know that the ptr is non null
            p4: NonNull::new_unchecked(P4),
            _marker: PhantomData,
        }
    }

    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn map_page_to_frame<A: FrameAllocator>(
        &mut self,
        page: Page,
        frame: Frame,
        frame_allocator: &mut A,
        flags: EntryFlags,
    ) {
        let p4 = self.p4_mut();
        let p3 = p4.create_next_table(page.p4_index(), frame_allocator);
        let p2 = p3.create_next_table(page.p3_index(), frame_allocator);
        let p1 = p2.create_next_table(page.p2_index(), frame_allocator);

        // the entry must be unused
        assert!(!p1.entries[page.p1_index()].is_used());

        p1.entries[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
    }

    pub fn map_page<A: FrameAllocator>(
        &mut self,
        page: Page,
        frame_allocator: &mut A,
        flags: EntryFlags,
    ) {
        // get a random (free) frame
        let frame = frame_allocator
            .allocate_frame()
            .expect("Out of memory. Could not allocate new frame.");

        self.map_page_to_frame(page, frame, frame_allocator, flags);
    }

    pub fn unmap_page(&self) {
        unimplemented!("Page unmapping is not yet implemented!");
    }

    /*
     * This takes a Page and returns the respective Frame if
     * the address is mapped.
     */
    fn translate_page(&self, page: Page) -> Option<Frame> {
        // p3 might be needed if huge pages are involed
        let p3 = self.p4().next_table(page.p4_index());

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1.entries[page.p1_index()].pointed_frame())
            /*
             * This might happen if the addr is not mapped (page does not exist) or
             * there is a huge page involved (next_table() does not support huge pages)
             */
            .or_else(|| {
                let p3_entry = p3?.entries[page.p3_index()];
                if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                    // every p3 entry points to 1GiB pages, so the addr must be 1GiB aligned
                    assert!(p3_entry.phy_addr()? % (ENTRY_COUNT * ENTRY_COUNT * PAGE_SIZE) == 0);

                    return Some(Frame::corresponding_frame(
                        p3_entry.phy_addr()?
                            + page.p2_index() * ENTRY_COUNT * PAGE_SIZE
                            + page.p1_index() * PAGE_SIZE,
                    ));
                }

                let p2_entry = p3?.next_table(page.p3_index())?.entries[page.p2_index()];
                if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                    // every p2 entry points to a 2MiB page, so the addr must be 2MiB aligned
                    assert!(p2_entry.phy_addr()? % (ENTRY_COUNT * PAGE_SIZE) == 0);

                    return Some(Frame::corresponding_frame(
                        p2_entry.phy_addr()? + page.p1_index() * PAGE_SIZE,
                    ));
                }

                None
            })
    }

    /*
     * Takes a virtual address and returns the respective physical address
     * if it exists (if it is mapped).
     */
    pub fn translate(self, virtual_addr: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_addr % PAGE_SIZE;
        let page = Page::corresponding_page(virtual_addr);
        let frame = self.translate_page(page)?;

        Some(frame.0 * PAGE_SIZE + offset)
    }
}
