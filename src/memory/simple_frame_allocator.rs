use core::usize;

use super::{Frame, FrameAllocator};
use multiboot2::MemoryArea;
use multiboot2::{BootInformation, BootInformationHeader};

pub struct SimpleFrameAllocator<'a> {
    current_frame: Frame,
    areas: &'a [MemoryArea],
    current_area: Option<&'a MemoryArea>,

    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl<'a> SimpleFrameAllocator<'a> {
    fn new(
        mem_areas: &'a [MemoryArea],
        kernel_start: usize,
        kernel_end: usize,
        multiboot_start: usize,
        multiboot_end: usize,
    ) -> Self {
        SimpleFrameAllocator {
            current_frame: Frame { idx: usize::MAX },
            areas: mem_areas,
            current_area: None,

            kernel_start: SimpleFrameAllocator::corresponding_frame(kernel_start),
            kernel_end: SimpleFrameAllocator::corresponding_frame(kernel_end),
            multiboot_start: SimpleFrameAllocator::corresponding_frame(multiboot_start),
            multiboot_end: SimpleFrameAllocator::corresponding_frame(multiboot_end),
        }
    }
}

impl<'a> FrameAllocator for SimpleFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<Frame> {
        // no free frames left (and areas)
        if self.current_area == None {
            return None;
        }

        // get frame to return
        // let frame = self.next_frame;
        //

        Some(Frame { idx: 0 })

        // prepare the next frame to be returned by the next call
    }

    fn deallocate_frame(&mut self, frame: Frame) {}
}
