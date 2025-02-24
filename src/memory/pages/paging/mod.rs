pub mod inactive_paging_context;

use crate::memory::{cr3::CR3, frames::{Frame, FrameAllocator}, MemoryError, PhysicalAddress, VirtualAddress, FRAME_PAGE_SIZE};
use super::{page_table::{page_table_entry::EntryFlags, Level4, Table, ENTRY_COUNT, P4}, Page};
use inactive_paging_context::InactivePagingContext;
use core::{marker::PhantomData, ptr::NonNull};
use spin::Mutex;

/*
 * Safety: Raw pointers are not Send/Sync so `Paging` cannot be used between threads as it would cause data races.
 */
pub(in crate::memory) struct ActivePagingContextInner {
    p4: NonNull<Table<Level4>>,

    // makes this struct `own` a `Table<Level4>`
    _marker: PhantomData<Table<Level4>>,
}

unsafe impl Send for ActivePagingContextInner {}

pub struct ActivePagingContext(Mutex<ActivePagingContextInner>);

pub static ACTIVE_PAGING_CTX: ActivePagingContext = ActivePagingContext(Mutex::new(ActivePagingContextInner {
    // this can be unchecked as we know that the ptr is non null
    p4: unsafe { NonNull::new_unchecked(P4) },
    _marker: PhantomData,
}));

impl ActivePagingContextInner {
    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    /*
     * Maps a specific Page to a specific Frame.
     */
    pub(in crate::memory) fn map_page_to_frame<A: FrameAllocator>(&mut self, page: Page, frame: Frame, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        let p4 = self.p4_mut();
        let p3 = p4.create_next_table(page.p4_index(), frame_allocator)?;
        let p2 = p3.create_next_table(page.p3_index(), frame_allocator)?;
        let p1 = p2.create_next_table(page.p2_index(), frame_allocator)?;

        // the entry must be unused
        if p1.entries[page.p1_index()].is_used() {
            return Err(MemoryError::MappingUsedTableEntry);
        }

        p1.entries[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
        Ok(())
    }

    /*
     * Maps a specific Page to a (random) Frame.
     */
    pub(in crate::memory) fn map_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        // get a random (free) frame
        let frame = frame_allocator.allocate_frame()?;
        return self.map_page_to_frame(page, frame, frame_allocator, flags);
    }

    /*
     * Maps the Page containing the `virtual_addr` to a (random) Frame.
     */
    pub(in crate::memory) fn map<A: FrameAllocator>(&mut self, virtual_addr: VirtualAddress, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        let page = Page::from_virt_addr(virtual_addr)?;
        return self.map_page(page, frame_allocator, flags);
    }

    /*
     * Maps a Frame to a Page with same addr (identity mapping).
     */
    pub(in crate::memory) fn identity_map<A: FrameAllocator>(&mut self, frame: Frame, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        self.map_page_to_frame(Page::from_virt_addr(frame.addr())?, frame, frame_allocator, flags)
    }

    /*
     * This will unmap a page and the respective frame.
     * If an invalidd page is given, it will simply be ignored as there is nothing to unmap.
     */
    // TODO: - free P1, P2 and P3 if they get empty
    //       - deallocate the frame
    pub(in crate::memory) fn unmap_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &A) {
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
    pub(in crate::memory) fn translate_page(&self, page: Page) -> Option<Frame> {
        self.p4().next_table(page.p4_index())
            .and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1.entries[page.p1_index()].pointed_frame())
    }

    /*
     * Takes a virtual address and returns the respective physical address if it exists (if it is mapped).
     */
    // Safety: does not need locking as we are calling translate_page() that will lock()
    pub(in crate::memory) fn translate(&self, virtual_addr: VirtualAddress) -> Result<Option<PhysicalAddress>, MemoryError> {
        let offset = virtual_addr % FRAME_PAGE_SIZE;
        let page = Page::from_virt_addr(virtual_addr)?;

        match self.translate_page(page) {
            Some(frame) => return Ok(Some(frame.addr() + offset)),
            None => return Ok(None),
        }
    }

    /*
     * The, current, active paging context will become inactive
     * and the inactive one, will become active.
     */
    pub(in crate::memory) fn switch(&self, inactive_context: &mut InactivePagingContext) {
        // the ActivePagingContext does not need to be modified as it only uses a recursive addr,
        // so it will work with whatever addr is in CR3

        // swap the values in CR3 and InactivePagingContext (also clears the TLB)
        inactive_context.switch_with_cr3();
    }
}

impl ActivePagingContext {
    /*
     * Maps a specific Page to a specific Frame.
     */
    pub fn map_page_to_frame<A: FrameAllocator>(&self, page: Page, frame: Frame, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.map_page_to_frame(page, frame, frame_allocator, flags)
    }

    /*
     * Maps a specific Page to a (random) Frame.
     */
    pub fn map_page<A: FrameAllocator>(&self, page: Page, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.map_page(page, frame_allocator, flags)
    }

    /*
     * Maps the Page containing the `virtual_addr` to a (random) Frame.
     */
    pub fn map<A: FrameAllocator>(&self, virtual_addr: VirtualAddress, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.map(virtual_addr, frame_allocator, flags)
    }

    /*
     * Maps a Frame to a Page with same addr (identity mapping).
     */
    pub fn identity_map<A: FrameAllocator>(&self, frame: Frame, frame_allocator: &A, flags: EntryFlags) -> Result<(), MemoryError> {
        let apc = &mut *self.0.lock();
        apc.identity_map(frame, frame_allocator, flags)
    }

    /*
     * This will unmap a page and the respective frame.
     * If an invalidd page is given, it will simply be ignored as there is nothing to unmap.
     */
    pub fn unmap_page<A: FrameAllocator>(&self, page: Page, frame_allocator: &A) {
        let apc = &mut *self.0.lock();
        apc.unmap_page(page, frame_allocator);
    }

    /*
     * This takes a Page and returns the respective Frame if the address is mapped.
     */
    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let apc = &*self.0.lock();
        apc.translate_page(page)
    }

    /*
     * Takes a virtual address and returns the respective physical address if it exists (if it is mapped).
     */
    pub fn translate(&self, virtual_addr: VirtualAddress) -> Result<Option<PhysicalAddress>, MemoryError> {
        let apc = &mut *self.0.lock();
        apc.translate(virtual_addr)
    }

    /*
     * The, current, active paging context will become inactive
     * and the inactive one, will become active.
     */
    pub fn switch(&self, inactive_context: &mut InactivePagingContext) {
        let apc = &*self.0.lock();
        apc.switch(inactive_context);
    }

    /*
     * This is a special method that should only be used inside `memory` as it should not
     * really be part of the public interface.
     * 
     * This allows to call ActivePagingContext funcs on an InactivePagingContext object to
     * map, unmap and translate addrs manually as if it were the active paging context.
     * 
     * This does not affect hardware translations and thus, is totally safe to use as long as the
     * caller makes sure that the inactive paging context is in a valid state before being switched to.
     */
    // Safety: If `&mut ActivePagingContextInner` is used incorrectly, it will lead to UB so, please be careful and
    //   do not share or send the reference to anywhere else. This is why this function cannot be used outside of crate::memory.
    pub(in crate::memory) fn update_inactive_context<F, A>(&self, inactive_context: &InactivePagingContext, frame_allocator: &A, f: F)
        -> Result<(), MemoryError>
    where
        F: FnOnce(&mut ActivePagingContextInner, &A) -> Result<(), MemoryError>,
        A: FrameAllocator
    {
        let apc = &mut *self.0.lock();

        // backup the current active paging p4 frame addr and map the current p4 table so we can change it later
        let p4_frame = Frame::from_phy_addr(CR3::get());
        let p4_page = Page::from_virt_addr(0xdeadbeef)?;
        apc.map_page_to_frame(p4_page, p4_frame, frame_allocator, EntryFlags::PRESENT | EntryFlags::WRITABLE)?;

        // set the recusive entry on the current paging context to the inactive p4 frame
        apc.p4_mut().entries[ENTRY_COUNT - 1].set_phy_addr(inactive_context.p4_frame());

        // flush the all the tlb entries
        // needed because the recursive addrs may be mapped to the active paging context and
        // we need them pointing to the inactive context (hardware translations would still work)
        CR3::invalidate_all();

        f(apc, frame_allocator)?;

        // restore the active paging context recusive mapping
        let table = unsafe { &mut *(p4_page.addr() as usize as *mut Table<Level4>) };
        table.entries[ENTRY_COUNT - 1].set_phy_addr(p4_frame);

        // invalidate the entries so that the recursive mapping works again (we don't use cached addrs)
        CR3::invalidate_all();
        apc.unmap_page(p4_page, frame_allocator);

        Ok(())
    }
}
