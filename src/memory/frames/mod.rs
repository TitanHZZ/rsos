pub mod simple_frame_allocator;

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

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Result<Frame, MemoryError>;
    fn deallocate_frame(&mut self, frame: Frame);
}
