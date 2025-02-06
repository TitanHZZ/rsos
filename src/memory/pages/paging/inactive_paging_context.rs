use crate::memory::{frames::{Frame, FrameAllocator}, pages::page_table::page_table_entry::EntryFlags, MemoryError};
use super::ActivePagingContext;

pub struct InactivePagingContext {
    // this is just a frame because because it's not in use so it's not really a page table
    p4_frame: Frame,
}

impl InactivePagingContext {
    // TODO: - we are missing a page allocator. so for now we just use a big address to map the frame
    pub fn new<A: FrameAllocator>(active_paging: &mut ActivePagingContext, frame_allocator: &mut A) -> Result<Self, MemoryError> {
        let p4_frame = frame_allocator.allocate_frame()?;

        // map the p4 frame and add the recursive entry
        active_paging.map(0xdeadbeef, frame_allocator, EntryFlags::PRESENT | EntryFlags::WRITABLE)?;

        Err(MemoryError::NotEnoughPhyMemory)
    }
}
