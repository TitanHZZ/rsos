use crate::{memory::{cr3::CR3, frames::{Frame, FrameAllocator}, pages::{page_table::{page_table_entry::EntryFlags, Level4, Table, ENTRY_COUNT}, PageAllocator}, MemoryError}};
use super::ActivePagingContext;

pub struct InactivePagingContext {
    // this is just a frame because because it's not in use so it's not really a page table
    p4_frame: Frame,
}

impl InactivePagingContext {
    /// This creates a new recursively mapped (inactive) paging context.
    pub fn new<F: FrameAllocator, P: PageAllocator>(active_paging: &ActivePagingContext, fa: &F, pa: &P) -> Result<Self, MemoryError> {
        let p4_frame = fa.allocate_frame()?;
        let p4_page = pa.allocate_page()?;

        // map the p4 frame
        active_paging.map_page_to_frame(p4_page, p4_frame, fa, EntryFlags::PRESENT | EntryFlags::WRITABLE)?;

        // recursively map the table
        // the unsafe block *is* safe as we know that the page is valid
        let table = unsafe { &mut *(p4_page.addr() as *mut Table<Level4>) };
        table.set_unused();
        table.entries[ENTRY_COUNT - 1].set(p4_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);

        // deallocate the page
        pa.deallocate_page(p4_page);

        // don't deallocate the frame because we need it to remain valid
        active_paging.unmap_page(p4_page, fa, false);
        Ok(InactivePagingContext { p4_frame })
    }

    pub(super) fn switch_with_cr3(&mut self) {
        // swap the values in CR3 and InactivePagingContext
        let p4_frame_backup = self.p4_frame;
        self.p4_frame = Frame::from_phy_addr(CR3::get());
        CR3::set(p4_frame_backup.addr());
    }

    pub fn p4_frame(&self) -> Frame {
        self.p4_frame
    }
}
