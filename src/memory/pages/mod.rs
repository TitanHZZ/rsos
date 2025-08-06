pub mod temporary_page_allocator;
pub mod simple_page_allocator;
pub mod page_table;
pub mod paging;

use core::ops::Deref;

use crate::memory::{pages::{paging::ActivePagingContext, simple_page_allocator::BitmapPageAllocator}, FRAME_PAGE_SIZE};
use super::{MemoryError, VirtualAddress};

#[derive(Clone, Copy)]
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
 * To calculate the table indexes but with the page index instead:
 * idx:
 *  p4_idx   -> 0    (912091 >> (39 - 12) & 0o777)
 *  p3_idx   -> 3    (912091 >> (30 - 12) & 0o777)
 *  p2_idx   -> 245  (912091 >> (21 - 12) & 0o777)
 *  p1_idx   -> 219  (912091 >> (12 - 12) & 0o777)
 *
 * We need to subtract 12 because the page index is 4096 (4KiB) times smaller than the original addr.
 */
impl Page {
    pub fn from_virt_addr(addr: VirtualAddress) -> Result<Page, MemoryError> {
        // in x86_64, the top 16 bits of a virtual addr must be sign extension bits. if they are not, its an invalid addr
        // !(addr < 0x0000_8000_0000_0000 || addr >= 0xffff_8000_0000_0000)
        if (0x0000_8000_0000_0000..0xffff_8000_0000_0000).contains(&addr) {
            return Err(MemoryError::PageInvalidVirtualAddress);
        }

        Ok(Page(addr / FRAME_PAGE_SIZE))
    }

    pub fn addr(&self) -> VirtualAddress {
        self.0 * FRAME_PAGE_SIZE
    }

    pub fn p4_index(&self) -> usize {
        (self.0 >> 27) & 0o777
    }

    pub fn p3_index(&self) -> usize {
        (self.0 >> 18) & 0o777
    }

    pub fn p2_index(&self) -> usize {
        (self.0 >> 9) & 0o777
    }

    #[allow(clippy::identity_op)]
    pub fn p1_index(&self) -> usize {
        (self.0 >> 0) & 0o777
    }
}

// TODO: this needs an option to allocate consecutive pages
/// A Page allocator.
/// 
/// # Safety
/// 
/// Whoever implements this must ensure its correctness since the compiler has no way of ensuring that memory will be correctly managed.
pub unsafe trait PageAllocator: Send + Sync {
    fn allocate_page(&self) -> Result<Page, MemoryError>;
    fn deallocate_page(&self, page: Page);

    /// Resets the page allocator state.
    /// 
    /// # Safety
    /// 
    /// This must be called (before any allocation) as the allocator expects it.
    /// In the case that it does not get called, memory corruption is the most likely outcome.
    unsafe fn init(&self, active_paging: &ActivePagingContext) -> Result<(), MemoryError>;
}

/// The global frame allocator.
pub struct GlobalPageAllocator {
    pa: Option<&'static dyn PageAllocator>,
}

impl GlobalPageAllocator {
    pub const fn new(pa: &'static dyn PageAllocator) -> Self {
        GlobalPageAllocator {
            pa: Some(pa)
        }
    }
}

impl Deref for GlobalPageAllocator {
    type Target = dyn PageAllocator;

    fn deref(&self) -> &Self::Target {
        self.pa.unwrap()
    }
}
