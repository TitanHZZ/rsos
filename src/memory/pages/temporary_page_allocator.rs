use crate::{globals::ACTIVE_PAGING_CTX, memory::{pages::{Page, PageAllocator}, MemoryError, VirtualAddress, FRAME_PAGE_SIZE}, serial_println};
use crate::data_structures::bitmap::Bitmap;
use spin::mutex::Mutex;

/// A page allocator meant to be used until a permanent page allocator is initialized.
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
    /// Create a new **TemporaryPageAllocator** that holds 8 pages for allocation starting at `start_addr`.
    /// 
    /// # Panics
    /// If `start_addr` is not a multiple of **FRAME_PAGE_SIZE**.
    pub const fn new(start_addr: VirtualAddress) -> Self {
        assert!(start_addr.is_multiple_of(FRAME_PAGE_SIZE));
        TemporaryPageAllocator(Mutex::new(TemporaryPageAllocatorInner::new(start_addr)))
    }
}

unsafe impl PageAllocator for TemporaryPageAllocator {
    unsafe fn init(&self) -> Result<(), MemoryError> {
        let allocator = &mut *self.0.lock();

        // make sure that the pages are not being used
        for i in 0..allocator.bitmap.len() {
            let addr = allocator.start_addr + i * FRAME_PAGE_SIZE;
            ACTIVE_PAGING_CTX.translate(addr).map_err(|_| MemoryError::BadTemporaryPageAllocator)?;
        }

        Ok(())
    }

    fn allocate(&self) -> Result<Page, MemoryError> {
        let allocator = &mut *self.0.lock();

        // look for the first free page and return it
        let idx = allocator.bitmap.iter().enumerate().find(|(_, bit)| !bit).ok_or(MemoryError::NotEnoughVirMemory)?.0;
        allocator.bitmap.set(idx, true);

        let page = Page::from_virt_addr(allocator.start_addr + idx * FRAME_PAGE_SIZE)?;
        serial_println!("Allocated page: {:#x}", page.0);

        Ok(page)
    }

    fn allocate_contiguous(&self) -> Result<Page, MemoryError> {
        todo!()
    }

    fn deallocate(&self, page: Page) {
        let allocator = &mut *self.0.lock();

        // make sure that the address is valid and within range
        assert!(page.addr() >= allocator.start_addr && page.addr() < (allocator.start_addr + allocator.bitmap.len() * FRAME_PAGE_SIZE));

        // make sure that the page was previously allocated
        let bit_idx = (page.addr() - allocator.start_addr) / FRAME_PAGE_SIZE;
        assert!(allocator.bitmap.get(bit_idx).is_some());

        // deallocate
        allocator.bitmap.set(bit_idx, false);

        serial_println!("Deallocated page: {:#x}", page.0);
    }
}
