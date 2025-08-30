use crate::{assert_called_once, data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, KERNEL}, memory::{frames::{Frame, FrameAllocator}, pages::{page_table::page_table_entry::EntryFlags, Page, PageAllocator}, AddrOps, MemoryError, VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM}, serial_println};
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
    fn addr_to_bit_idxs(&self, addr: VirtualAddress) -> (usize, usize) {
        assert!(addr >= Kernel::k_hh_start() && addr <= Kernel::hh_end());

        todo!()
    }

    fn allocate_level2_bitmap(&mut self, bitmap_idx: usize) -> Result<(), MemoryError> {
        assert!(bitmap_idx < 261120);

        let bitmap_start_addr = BitmapPageAllocator::level2_bitmaps_start_addr() + (BitmapPageAllocator::level2_bitmap_lenght() * bitmap_idx);
        for addr in (bitmap_start_addr..bitmap_start_addr + BitmapPageAllocator::level2_bitmap_lenght()).step_by(FRAME_PAGE_SIZE) {
            MEMORY_SUBSYSTEM.active_paging_context().map(addr, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE)?;
        }

        self.l1[bitmap_idx] = Some(unsafe {
            BitmapRefMut::from_raw_parts_mut(bitmap_start_addr as _, BitmapPageAllocator::level2_bitmap_lenght(), None, true)
        });

        serial_println!("aspdksaçldsadjskd");
        self.addr_to_bit_idxs(bitmap_start_addr);
        serial_println!("aspdksaçldsadjskdsakldjsaldjskdjks");

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
        KERNEL.fa_hh_start() + match MEMORY_SUBSYSTEM.frame_allocator().metadata_memory_range() {
            Some(metadata) => metadata.length(),
            None => 0,
        }
    }
}

unsafe impl<'a> PageAllocator for BitmapPageAllocator<'a> {
    unsafe fn init(&self) -> Result<(), MemoryError> {
        assert_called_once!("Cannot call BitmapPageAllocator::init() more than once");
        let allocator = &mut *self.0.lock();

        // TODO: - set following memory ranges as "allocated": kernel, multiboot2, frame allocator
        //       - this will require the mapping of the first few level 2 bitmaps

        // the amount of bytes currently "allocated" in the higher half
        let allocated_size = BitmapPageAllocator::level2_bitmaps_start_addr() - Kernel::k_hh_start();

        let allocated_size_in_pages = allocated_size / FRAME_PAGE_SIZE;
        let level2_bitmap_count = allocated_size_in_pages.align_up(BitmapPageAllocator::level2_bitmap_bit_lenght())
            / BitmapPageAllocator::level2_bitmap_bit_lenght();

        serial_println!("allocated size: {:#x}", allocated_size);
        serial_println!("level2 bitmap count: {}", level2_bitmap_count);

        allocator.allocate_level2_bitmap(0)?;

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
