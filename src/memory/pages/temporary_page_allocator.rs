use crate::memory::{pages::{paging::ActivePagingContext, Page, PageAllocator}, AddrOps, MemoryError, VirtualAddress, FRAME_PAGE_SIZE};
use crate::data_structures::bitmap::Bitmap;
use spin::mutex::Mutex;

// TODO: make all temporary page allocations use this

struct TemporaryPageAllocatorInner {
    bitmap: Bitmap<1>,
    start_addr: VirtualAddress,
}

pub struct TemporaryPageAllocator(Mutex<TemporaryPageAllocatorInner>);

impl TemporaryPageAllocatorInner {
    /// Creates a new **TemporaryPageAllocatorInner** that will allocate pages from `start_addr` onwards.
    const fn new(start_addr: VirtualAddress) -> Self {
        TemporaryPageAllocatorInner {
            bitmap: Bitmap::new(None),
            start_addr,
        }
    }
}

impl TemporaryPageAllocator {
    /// Create a new **TemporaryPageAllocator** that holds 8 pages for allocation starting at `start_addr` aligned down to `FRAME_PAGE_SIZE`.
    pub fn new(start_addr: VirtualAddress) -> Self {
        TemporaryPageAllocator(Mutex::new(TemporaryPageAllocatorInner::new(start_addr.align_down(FRAME_PAGE_SIZE))))
    }
}

unsafe impl PageAllocator for TemporaryPageAllocator {
    unsafe fn init(&self, active_paging: &ActivePagingContext) -> Result<(), MemoryError> {
        let allocator = &mut *self.0.lock();

        // make sure that the pages are not being used
        for i in 0..allocator.bitmap.len() {
            let addr = allocator.start_addr + i * FRAME_PAGE_SIZE;
            active_paging.translate(addr).map_err(|_| MemoryError::BadTemporaryPageAllocator)?;
        }

        Ok(())
    }

    fn allocate_page(&self) -> Result<Page, MemoryError> {
        let allocator = &mut *self.0.lock();

        // look for the first free page and return it
        let idx = allocator.bitmap.iter().enumerate().find(|(_, bit)| !bit).ok_or(MemoryError::NotEnoughVirMemory)?.0;
        allocator.bitmap.set(idx, true);

        return Ok(Page::from_virt_addr(allocator.start_addr + idx * FRAME_PAGE_SIZE)?);
    }

    fn deallocate_page(&self, page: Page) {
        let allocator = &mut *self.0.lock();

        // make sure that the address is valid and within range
        assert!(page.addr() >= allocator.start_addr && page.addr() < (allocator.start_addr + allocator.bitmap.len() * FRAME_PAGE_SIZE));

        // make sure that the page was previously allocated
        let bit_idx = (page.addr() - allocator.start_addr) / FRAME_PAGE_SIZE;
        assert!(allocator.bitmap.get(bit_idx).is_some());

        // deallocate
        allocator.bitmap.set(bit_idx, false);
    }
}
