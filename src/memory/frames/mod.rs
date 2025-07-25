pub mod simple_frame_allocator;
pub mod bitmap_frame_allocator;

use crate::{kernel::Kernel, memory::{frames::bitmap_frame_allocator::BitmapFrameAllocator}};
use super::{MemoryError, PhysicalAddress, FRAME_PAGE_SIZE};
use crate::memory::ProhibitedMemoryRange;

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Frame(usize); // this usize is the frame index in the physical memory

impl Frame {
    pub fn from_phy_addr(addr: PhysicalAddress) -> Frame {
        Frame(addr / FRAME_PAGE_SIZE)
    }

    pub fn addr(&self) -> PhysicalAddress {
        self.0 * FRAME_PAGE_SIZE
    }
}

/// A Frame allocator to be used OS wide.
pub unsafe trait FrameAllocator: Send + Sync {
    fn allocate_frame(&self) -> Result<Frame, MemoryError>;
    fn deallocate_frame(&self, frame: Frame);

    /// Resets the frame allocator state.
    /// 
    /// # Safety
    /// 
    /// This must be called (before any allocation) as the allocator expects it.
    /// In the case that it does not get called, memory corruption is the most likely outcome.
    unsafe fn init(&self, kernel: &Kernel) -> Result<(), MemoryError>;

    /// Remaps the frame allocator to use the new mapping to its underlying control structure.
    /// 
    /// # Safety
    /// 
    /// This **does not** recreate the frame allocator, it simply adjusts the internal ptr to the control structure so,
    /// if the new mapping is wrong, this *will* result in **undefined behavior**.
    /// **Must** be called right after changing the mapping and before any more frame allocations to avoid problems.
    unsafe fn remap(&self, kernel: &Kernel);

    /// Get the physical memory region that **MUST** be mapped and cannot be used for allocations by frame allocators.
    fn prohibited_memory_range(&self) -> Option<ProhibitedMemoryRange>;
}

/// The global frame allocator.
// TODO: a good idea would be to create a simple mechanism that would allow an easy way to switch the frame allocator
// even different allocators for different tests and "runners"
pub static FRAME_ALLOCATOR: BitmapFrameAllocator = BitmapFrameAllocator::new();
