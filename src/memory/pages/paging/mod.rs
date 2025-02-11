pub mod inactive_paging_context;

use crate::memory::{cr3::CR3, frames::{Frame, FrameAllocator}, MemoryError, PhysicalAddress, VirtualAddress, FRAME_PAGE_SIZE};
use super::{page_table::{page_table_entry::EntryFlags, Level4, Table, ENTRY_COUNT, P4}, Page};
use inactive_paging_context::InactivePagingContext;
use core::{marker::PhantomData, ptr::NonNull};

/*
 * Safety: Raw pointers are not Send/Sync so `Paging` cannot be used between threads as it would cause data races.
 */
pub struct ActivePagingContext {
    p4: NonNull<Table<Level4>>,

    // makes this struct `own` a `Table<Level4>`
    _marker: PhantomData<Table<Level4>>,
}

impl ActivePagingContext {
    /*
     * Safety: This should be unsafe because the p4 addr will always be the same (at least for now),
     * and that means that creating multiple `Paging` objects could result in undefined behaviour
     * as all the `Paging` objects woulb be pointing to the same memory (and own it).
     */
    pub unsafe fn new() -> Self {
        ActivePagingContext {
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

    /*
     * Maps a specific Page to a specific Frame.
     */
    pub fn map_page_to_frame<A: FrameAllocator>(&mut self, page: Page, frame: Frame, frame_allocator: &mut A, flags: EntryFlags) -> Result<(), MemoryError> {
        let p4 = self.p4_mut();
        let p3 = p4.create_next_table(page.p4_index(), frame_allocator)?;
        let p2 = p3.create_next_table(page.p3_index(), frame_allocator)?;
        let p1 = p2.create_next_table(page.p2_index(), frame_allocator)?;

        // the entry must be unused
        debug_assert!(!p1.entries[page.p1_index()].is_used());

        p1.entries[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
        Ok(())
    }

    /*
     * Maps a specific Page to a (random) Frame.
     */
    pub fn map_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &mut A, flags: EntryFlags) -> Result<(), MemoryError> {
        // get a random (free) frame
        let frame = frame_allocator.allocate_frame()?;
        return self.map_page_to_frame(page, frame, frame_allocator, flags);
    }

    /*
     * Maps the Page containing the `virtual_addr` to a (random) Frame.
     */
    pub fn map<A: FrameAllocator>(&mut self, virtual_addr: VirtualAddress, frame_allocator: &mut A, flags: EntryFlags) -> Result<(), MemoryError> {
        let page = Page::from_virt_addr(virtual_addr)?;
        return self.map_page(page, frame_allocator, flags);
    }

    /*
     * Maps a Frame to a Page with same addr (identity mapping).
     */
    pub fn identity_map<A: FrameAllocator>(&mut self, frame: Frame, frame_allocator: &mut A, flags: EntryFlags) -> Result<(), MemoryError> {
        self.map_page_to_frame(Page::from_virt_addr(frame.addr())?, frame, frame_allocator, flags)
    }

    /*
     * This will unmap a page and the respective frame.
     * If an invalidd page is given, it will simply be ignored as there is nothing to unmap.
     */
    // TODO: - free P1, P2 and P3 if they get empty
    //       - deallocate the frame
    pub fn unmap_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &mut A) {
        // set the entry in p1 as unused and free the respective frame
        self.p4_mut().next_table(page.p4_index())
            .and_then(|p3: _| p3.next_table_mut(page.p3_index()))
            .and_then(|p2: _| p2.next_table_mut(page.p2_index()))
            .and_then(|p1: _| {
                let entry = &mut p1.entries[page.p1_index()];
                let frame = entry.pointed_frame();
                entry.set_unused();

                frame
            }).and_then(|frame| {
                // deallocate the frame
                // frame_allocator.deallocate_frame(frame);

                Some(()) // `and_then()` requires an Option to be returned
            });

        // invalidate the TLB entry
        CR3::invalidate_entry(page.addr());
    }

    /*
     * This takes a Page and returns the respective Frame if the address is mapped.
     */
    fn translate_page(&self, page: Page) -> Option<Frame> {
        self.p4().next_table(page.p4_index())
            .and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1.entries[page.p1_index()].pointed_frame())
    }

    /*
     * Takes a virtual address and returns the respective physical address if it exists (if it is mapped).
     */
    pub fn translate(&self, virtual_addr: VirtualAddress) -> Result<Option<PhysicalAddress>, MemoryError> {
        let offset = virtual_addr % FRAME_PAGE_SIZE;
        let page = Page::from_virt_addr(virtual_addr)?;

        match self.translate_page(page) {
            Some(frame) => return Ok(Some(frame.addr() + offset)),
            None => return Ok(None),
        }
    }

    /*
     * This allows to call ActivePagingContext funcs on an InactivePagingContext object to
     * map, unmap and translate addrs manually as if it were the active paging context.
     * 
     * This does not affect hardware translations and thus, is totally safe to use as long as the
     * caller makes sure that the inactive paging context is in a valid state before being switched to.
     */
    // TODO: disallow a recursive update_inactive_context() call as it does not work
    pub fn update_inactive_context<F, A>(&mut self, inactive_context: &InactivePagingContext, frame_allocator: &mut A, f: F) -> Result<(), MemoryError>
    where
        F: FnOnce(&mut ActivePagingContext, &mut A) -> Result<(), MemoryError>,
        A: FrameAllocator
    {
        // backup the current active paging p4 frame addr and map the current p4 table so we can change it later
        let p4_frame = Frame::from_phy_addr(CR3::get());
        let p4_page = Page::from_virt_addr(0xdeadbeef)?;
        self.map_page_to_frame(p4_page, p4_frame, frame_allocator, EntryFlags::PRESENT | EntryFlags::WRITABLE)?;

        // set the recusive entry on the current paging context to the inactive p4 frame
        self.p4_mut().entries[ENTRY_COUNT - 1].set_phy_addr(inactive_context.p4_frame());

        // flush the all the tlb entries
        // needed because the recursive addrs may be mapped to the active paging context and
        // we need them pointing to the inactive context (hardware translations would still work)
        CR3::invalidate_all();

        f(self, frame_allocator)?;

        // restore the active paging context recusive mapping
        let table = unsafe { &mut *(p4_page.addr() as usize as *mut Table<Level4>) };
        table.entries[ENTRY_COUNT - 1].set_phy_addr(p4_frame);

        // invalidate the entries so that the recursive mapping works again (we don't use cached addrs)
        CR3::invalidate_all();
        self.unmap_page(p4_page, frame_allocator);

        Ok(())
    }

    /*
     * The, current, active paging context will become inactive
     * and the inactive one, will become active.
     */
    pub fn switch(&mut self, inactive_context: &mut InactivePagingContext) {
        // the ActivePagingContext does not need to be modified as it only uses a recursive addr,
        // so it will work with whatever addr is in CR3

        // swap the values in CR3 and InactivePagingContext (also clears the TLB)
        inactive_context.switch_with_cr3();
    }
}
