use crate::{assert_called_once, data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, KERNEL}, memory::{frames::FrameAllocator, pages::{page_table::page_table_entry::EntryFlags, Page, PageAllocator}, AddrOps, MemoryError, VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM}, serial_println};
use spin::Mutex;

// This page allocator manages the entire higher half of the 48 bit address space, 2 ** 48 // 2 bytes.
// 
// But, we don't actually need to manage all this memory, because the page tables are recursive meaning that
// the last P4 entry is mapped to the P4 page table itself. This removes 512GB of memory from the total,
// at the top of the address space so, now we "only" need to map 2 ** 48 // 2 - 2 ** 30 * 512.
// This is 140187732541440 bytes or 34225520640 4KB pages.
// So, we need 34225520640 bits and when using 16KB bitmaps, we get 34225520640 // (4096 * 8 * 4) bitmaps (261120)
// with 4096 * 8 bits per 4KB, and we want 4 4KBs.
// 
// All of this means that we need 261120, statically allocated, Option<BitmapRefMut<'a>>s where each bitmap will be of size 16KB.
// 
// Notes:
// 1MB --> 2 ** 20
// 1GB --> 2 ** 30
// 1TB --> 2 ** 40
// 
// All calculations are in Python syntax.

struct BitmapPageAllocatorInner<'a> {
    // every level 1 bitmap is 16kb
    l1: [Option<BitmapRefMut<'a>>; 261120],
    initialized: bool,
}

impl<'a> BitmapPageAllocatorInner<'a> {
    fn page_idx_to_bit_idxs(&self, page_idx: usize) -> (usize, usize) {
        assert!(page_idx < (Kernel::hh_end() / FRAME_PAGE_SIZE));
        (page_idx / BitmapPageAllocator::level2_bitmap_bit_lenght(), page_idx % BitmapPageAllocator::level2_bitmap_bit_lenght())
    }

    fn addr_to_bit_idxs(&self, addr: VirtualAddress) -> (usize, usize) {
        assert!(addr >= Kernel::k_hh_start() && addr <= Kernel::hh_end());
        let page_idx = (addr.align_down(FRAME_PAGE_SIZE) - Kernel::k_hh_start()) / FRAME_PAGE_SIZE;
        self.page_idx_to_bit_idxs(page_idx)
    }

    fn allocate_level2_bitmap(&mut self, bitmap_idx: usize) -> Result<(), MemoryError> {
        assert!(bitmap_idx < 261120);

        // allocate and map all the required pages for the second level bitmap
        let bitmap_start_addr = BitmapPageAllocator::level2_bitmaps_start_addr() + (BitmapPageAllocator::level2_bitmap_lenght() * bitmap_idx);
        for addr in (bitmap_start_addr..bitmap_start_addr + BitmapPageAllocator::level2_bitmap_lenght()).step_by(FRAME_PAGE_SIZE) {
            MEMORY_SUBSYSTEM.active_paging_context().map(addr, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)?;
        }

        // build the level 2 bitmap
        self.l1[bitmap_idx] = Some(unsafe {
            BitmapRefMut::from_raw_parts_mut(bitmap_start_addr as _, BitmapPageAllocator::level2_bitmap_lenght(), None, true)
        });

        // recursively allocate the second level bitmap that marks the current one as allocated
        let (l1_idx, l2_idx) = self.addr_to_bit_idxs(bitmap_start_addr);
        serial_println!("bitmap start addr: {:#x}, index: {}", bitmap_start_addr, bitmap_idx);
        serial_println!("l1 index: {}; l2 index: {}", l1_idx, l2_idx);
        if self.l1[l1_idx].is_none() {
            self.allocate_level2_bitmap(l1_idx)?;
        }

        // mark the current second level bitmap pages as allocated in the new, recursively allocated, second level bitmap
        for offset in 0..(BitmapPageAllocator::level2_bitmap_lenght() / FRAME_PAGE_SIZE) {
            self.l1[l1_idx].as_mut().unwrap().set(l2_idx + offset, true);
        }

        Ok(())
    }
}

pub struct BitmapPageAllocator<'a>(Mutex<BitmapPageAllocatorInner<'a>>);

impl<'a> BitmapPageAllocator<'a> {
    #[cfg(not(test))]
    pub(in crate::memory::pages) const fn new() -> Self {
        BitmapPageAllocator(Mutex::new(BitmapPageAllocatorInner {
            l1: [const { None }; 261120],
            initialized: false,
        }))
    }

    #[cfg(test)]
    pub const fn new() -> Self {
        BitmapPageAllocator(Mutex::new(BitmapPageAllocatorInner {
            l1: [const { None }; 261120],
            initialized: false,
        }))
    }

    /// Get the size of a level 2 bitmap in bytes.
    const fn level2_bitmap_lenght() -> usize {
        FRAME_PAGE_SIZE * 4
    }

    /// Get the size of a level 2 bitmap in bits.
    const fn level2_bitmap_bit_lenght() -> usize {
        BitmapPageAllocator::level2_bitmap_lenght() * 8
    }

    /// Get the address where the first level 2 bitmap will start.
    fn level2_bitmaps_start_addr() -> VirtualAddress {
        (KERNEL.fa_hh_start() + match MEMORY_SUBSYSTEM.frame_allocator().metadata_memory_range() {
            Some(metadata) => metadata.length(),
            None => 0,
        }).align_up(Self::level2_bitmap_lenght())
    }
}

unsafe impl<'a> PageAllocator for BitmapPageAllocator<'a> {
    unsafe fn init(&self) -> Result<(), MemoryError> {
        assert_called_once!("Cannot call BitmapPageAllocator::init() more than once");
        let allocator = &mut *self.0.lock();

        // the amount of bytes currently "allocated" in the higher half
        let allocated_size = BitmapPageAllocator::level2_bitmaps_start_addr() - Kernel::k_hh_start();

        let allocated_size_in_pages = allocated_size / FRAME_PAGE_SIZE;
        let level2_bitmap_count = allocated_size_in_pages.align_up(BitmapPageAllocator::level2_bitmap_bit_lenght())
            / BitmapPageAllocator::level2_bitmap_bit_lenght();

        // TODO: this can be improved for performance
        for page_idx in 0..allocated_size_in_pages {
            let (l1_idx, l2_idx) = allocator.page_idx_to_bit_idxs(page_idx);

            if allocator.l1[l1_idx].is_none() {
                allocator.allocate_level2_bitmap(l1_idx)?;
            }

            allocator.l1[l1_idx].as_mut().unwrap().set(l2_idx, true);
        }

        serial_println!("allocated size: {:#x}", allocated_size);
        serial_println!("allocated size in pages: {:#x}", allocated_size_in_pages);
        serial_println!("level2 bitmap count: {}", level2_bitmap_count);
        Ok(())
    }

    fn allocate(&self) -> Result<Page, MemoryError> {
        todo!()
    }

    fn allocate_contiguous(&self, _count: usize) -> Result<Page, MemoryError> {
        todo!()
    }

    fn deallocate(&self, _page: Page) {
        todo!()
    }
}
