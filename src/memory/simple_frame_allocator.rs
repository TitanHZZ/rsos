use super::{Frame, FrameAllocator};
use multiboot2::MemoryArea;

pub struct SimpleFrameAllocator<'a> {
    next_frame: Frame,
    areas: &'a [MemoryArea],
    current_area: usize,

    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl<'a> SimpleFrameAllocator<'a> {
    pub fn new(
        mem_areas: &'a [MemoryArea],
        kernel_start: usize,
        kernel_end: usize,
        multiboot_start: usize,
        multiboot_end: usize,
    ) -> Result<Self, ()> {
        let mut allocator = SimpleFrameAllocator {
            next_frame: Frame { idx: 0 },
            areas: mem_areas,
            current_area: 0,

            kernel_start: SimpleFrameAllocator::corresponding_frame(kernel_start),
            kernel_end: SimpleFrameAllocator::corresponding_frame(kernel_end),
            multiboot_start: SimpleFrameAllocator::corresponding_frame(multiboot_start),
            multiboot_end: SimpleFrameAllocator::corresponding_frame(multiboot_end),
        };

        // check if the initial frame is already in use
        if allocator.is_frame_used() {
            allocator.get_next_free_frame()?;
        }

        Ok(allocator)
    }

    fn is_frame_used(&self) -> bool {
        (self.next_frame >= self.kernel_start && self.next_frame <= self.kernel_end)
            || (self.next_frame >= self.multiboot_start && self.next_frame <= self.multiboot_end)
    }

    /*
     * Returns the next (free or used) frame if it exists.
     */
    fn get_next_frame(&mut self) -> Result<Frame, ()> {
        let last_frame_curr_area =
            Self::corresponding_frame(self.areas[self.current_area].end_address() as usize);

        // self.areas.into_iter().map(|area| {});
        if self.next_frame == last_frame_curr_area {
            self.current_area += 1;
        }

        if self.current_area >= self.areas.len() {
            return Err(());
        }

        self.next_frame =
            Self::corresponding_frame(self.areas[self.current_area].end_address() as usize);

        Ok(self.next_frame)
    }

    fn get_next_free_frame(&mut self) -> Result<Frame, ()> {
        let mut fr = self.get_next_frame()?;

        while self.is_frame_used() {
            fr = self.get_next_frame()?;
        }

        Ok(fr)
    }
}

impl<'a> FrameAllocator for SimpleFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<Frame> {
        unimplemented!();
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        unimplemented!();
    }
}
