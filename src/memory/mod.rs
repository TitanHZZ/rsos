const PAGE_SIZE: usize = 4096;

mod simple_frame_allocator;

#[repr(transparent)]
pub struct Frame {
    idx: usize,
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);

    fn corresponding_frame(phy_addr: usize) -> Frame {
        Frame {
            idx: phy_addr / PAGE_SIZE,
        }
    }
}
