use crate::{memory::{MemoryError, FRAME_PAGE_SIZE}, multiboot2::memory_map::{MemoryMapEntry, MemoryMapEntryType}};
use super::{Frame, FrameAllocator};

pub struct SimpleFrameAllocator {
    // areas and the respective frames
    areas: &'static [MemoryMapEntry],
    current_area: usize,
    next_frame: Frame,

    // memory ranges that we need to avoid using so we don't override important memory
    k_start : Frame,
    k_end   : Frame,
    mb_start: Frame,
    mb_end  : Frame,
}

// pub static FRAME_ALLOCATOR: SimpleFrameAllocator = SimpleFrameAllocator::

impl SimpleFrameAllocator {
    pub fn new(areas: &'static [MemoryMapEntry], k_start: usize, k_end: usize, mb_start: usize, mb_end: usize) -> Result<Self, MemoryError> {
        let mut allocator = SimpleFrameAllocator {
            areas,
            current_area: 0,
            next_frame: Frame(0x0),

            k_start : Frame::from_phy_addr(k_start),
            k_end   : Frame::from_phy_addr(k_end),
            mb_start: Frame::from_phy_addr(mb_start),
            mb_end  : Frame::from_phy_addr(mb_end),
        };

        // make sure thet the allocator starts with a free frame
        if allocator.is_frame_used() {
            allocator.get_next_free_frame()?;
        }

        Ok(allocator)
    }

    fn is_frame_used(&self) -> bool {
        (self.next_frame >= self.k_start && self.next_frame <= self.k_end)
            || (self.next_frame >= self.mb_start && self.next_frame <= self.mb_end)
    }

    /*
     * Returns the next (free or used) frame if it exists.
     * This is an abstraction over the areas. With this, the frames may be seen as positions in a list.
     */
    fn get_next_frame(&mut self) -> Result<Frame, MemoryError> {
        let curr_area = &self.areas[self.current_area];
        let fr_after_last_in_curr_area= Frame::from_phy_addr((curr_area.base_addr + curr_area.length) as _);

        // check if the next frame is pointing outside the current area
        if self.next_frame == fr_after_last_in_curr_area {
            self.current_area += 1;

            // get to the next area with available ram
            while self.current_area < self.areas.len() && self.areas[self.current_area].entry_type() != MemoryMapEntryType::AvailableRAM {
                self.current_area += 1;
            }

            // no more areas to use (ran out of usable memory)
            if self.current_area >= self.areas.len() {
                return Err(MemoryError::NotEnoughPhyMemory);
            }

            // get the first frame from the next area
            self.next_frame = Frame::from_phy_addr(self.areas[self.current_area].base_addr as usize);
        } else {
            // get the next frame from the same (current) area
            self.next_frame = Frame(self.next_frame.0 + 1);
        }

        Ok(self.next_frame)
    }

    fn get_next_free_frame(&mut self) -> Result<Frame, MemoryError> {
        let mut fr = self.get_next_frame()?;

        while self.is_frame_used() {
            fr = self.get_next_frame()?;
        }

        Ok(fr)
    }
}

impl FrameAllocator for SimpleFrameAllocator {
    fn allocate_frame(&mut self) -> Result<Frame, MemoryError> {
        let frame = Ok(self.next_frame)?;
        self.get_next_free_frame()?;

        // physical address needs to be page aligned (just used to make sure that the frame allocator is behaving)
        if frame.addr() % FRAME_PAGE_SIZE != 0 {
            return Err(MemoryError::FrameInvalidAllocatorAddr);
        }

        Ok(frame)
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        // for this, we will need some way to store a record of which frames are free and which ones are not
        // this may even require allocation (just a guess)
        unimplemented!();
    }
}
