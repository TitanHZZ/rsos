mod paging;
mod simple_frame_allocator;
pub use self::simple_frame_allocator::SimpleFrameAllocator;
pub use paging::test_paging;

const PAGE_SIZE: usize = 4096;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Frame(usize); // this usize is the frame index in the physical memory

impl Frame {
    fn corresponding_frame(addr: PhysicalAddress) -> Frame {
        Frame(addr / PAGE_SIZE)
    }

    fn start_address(&self) -> PhysicalAddress {
        self.0 * PAGE_SIZE
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}
