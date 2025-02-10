use crate::memory::{frames::{Frame, FrameAllocator}, pages::{page_table::{page_table_entry::EntryFlags, Level4, Table, ENTRY_COUNT}, Page}, MemoryError};
use super::ActivePagingContext;

pub struct InactivePagingContext {
    // this is just a frame because because it's not in use so it's not really a page table
    p4_frame: Frame,
}

// TODO: - we are missing a page allocator. so for now we just use a big address to map the frame
impl InactivePagingContext {
    /*
     * This creates a new recursively mapped (inactive) paging context.
     */
    pub fn new<A: FrameAllocator>(active_paging: &mut ActivePagingContext, frame_allocator: &mut A) -> Result<Self, MemoryError> {
        let p4_frame = frame_allocator.allocate_frame()?;
        let p4_page = Page::from_virt_addr(0xdeadbeef)?;

        // map the p4 frame
        active_paging.map_page_to_frame(p4_page, p4_frame, frame_allocator, EntryFlags::PRESENT | EntryFlags::WRITABLE)?;

        // recursive map the table
        // the unsafe block is safe as we know that the page is valid
        let table = unsafe { &mut *(p4_page.addr() as usize as *mut Table<Level4>) };
        table.set_unused();
        table.entries[ENTRY_COUNT - 1].set(p4_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);

        active_paging.unmap_page(p4_page, frame_allocator);
        Ok(InactivePagingContext { p4_frame })
    }

    pub fn p4_frame(&self) -> Frame {
        self.p4_frame
    }
}
