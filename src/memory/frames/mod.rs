pub mod simple_frame_allocator;
pub mod bitmap_frame_allocator;

use crate::memory::{frames::simple_frame_allocator::SimpleFrameAllocator, ProhibitedMemoryRange};
use super::{MemoryError, PhysicalAddress, FRAME_PAGE_SIZE};

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

    /// Get all memory regions that MUST not be touched by the page allocator.
    fn prohibited_memory_ranges(&self) -> Option<&[ProhibitedMemoryRange]>;
}

/// The global frame allocator.
pub static FRAME_ALLOCATOR: SimpleFrameAllocator = SimpleFrameAllocator::new();
