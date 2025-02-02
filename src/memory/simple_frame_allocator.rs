use crate::multiboot2::{MemoryArea, MemoryAreaType};
use super::{Frame, FrameAllocator};

pub struct SimpleFrameAllocator<'a> {
    // areas and the respective frames
    areas: &'a [MemoryArea],
    current_area: usize,
    next_frame: Frame,

    // memory ranges that we need to avoid using so we don't override important memory
    k_start: Frame,
    k_end: Frame,
    mb_start: Frame,
    mb_end: Frame,
}

impl<'a> SimpleFrameAllocator<'a> {
    pub fn new(areas: &'a [MemoryArea], k_start: usize, k_end: usize, mb_start: usize, mb_end: usize) -> Option<Self> {
        let mut allocator = SimpleFrameAllocator {
            areas,
            current_area: 0,
            next_frame: Frame(0x0),

            k_start: Frame::from_phy_addr(k_start),
            k_end: Frame::from_phy_addr(k_end),
            mb_start: Frame::from_phy_addr(mb_start),
            mb_end: Frame::from_phy_addr(mb_end),
        };

        // make sure thet the allocator starts with a free frame
        if allocator.is_frame_used() {
            allocator.get_next_free_frame()?;
        }

        Some(allocator)
    }

    fn is_frame_used(&self) -> bool {
        (self.next_frame >= self.k_start && self.next_frame <= self.k_end)
            || (self.next_frame >= self.mb_start && self.next_frame <= self.mb_end)
    }

    /*
     * Returns the next (free or used) frame if it exists.
     * This is an abstraction over the areas. With this, the frames may be seen as positions in a list.
     */
    fn get_next_frame(&mut self) -> Option<Frame> {
        let fr_after_last_in_curr_area = Frame::from_phy_addr(self.areas[self.current_area].end_address() as usize + 1);

        // check if the next frame is pointing outside the current area
        if self.next_frame == fr_after_last_in_curr_area {
            self.current_area += 1;

            // get to the next area with available ram
            while self.current_area < self.areas.len() && self.areas[self.current_area].typ() != MemoryAreaType::Available {
                self.current_area += 1;
            }

            // no more areas to use (ran out of usable memory)
            if self.current_area >= self.areas.len() {
                return None;
            }

            // get the first frame from the next area
            self.next_frame = Frame::from_phy_addr(self.areas[self.current_area].start_address() as usize);
        } else {
            // get the next frame from the same (current) area
            self.next_frame = Frame(self.next_frame.0 + 1);
        }

        Some(self.next_frame)
    }

    fn get_next_free_frame(&mut self) -> Option<Frame> {
        let mut fr = self.get_next_frame()?;

        while self.is_frame_used() {
            fr = self.get_next_frame()?;
        }

        Some(fr)
    }
}

impl<'a> FrameAllocator for SimpleFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<Frame> {
        let ret = Some(self.next_frame);
        self.get_next_free_frame()?;

        ret
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        unimplemented!();
    }
}
