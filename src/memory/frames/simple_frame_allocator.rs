use crate::{assert_called_once, kernel::{Kernel, KERNEL, KERNEL_PROHIBITED_MEM_RANGES_LEN}, memory::{MemoryError, PhysicalAddress, ProhibitedMemoryRange, FRAME_PAGE_SIZE}};
use crate::multiboot2::memory_map::{MemoryMap, MemoryMapEntryType, MemoryMapEntries};
use super::{Frame, FrameAllocator};
use spin::Mutex;

struct SimpleFrameAllocatorInner {
    // available areas
    areas: Option<MemoryMapEntries>,
    current_area: usize,

    // next frame to be used
    // this frame points to an area of type `AvailableRAM` and is not inside the prohibited memory ranges below
    next_frame: Frame,

    // memory ranges that we need to avoid using so we don't override important memory
    kernel_prohibited_memory_ranges: [ProhibitedMemoryRange; KERNEL_PROHIBITED_MEM_RANGES_LEN],

    initialized: bool,
}

pub struct SimpleFrameAllocator(Mutex<SimpleFrameAllocatorInner>);

impl Default for SimpleFrameAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleFrameAllocator {
    #[cfg(not(test))]
    pub(in crate::memory::frames) const fn new() -> Self {
        SimpleFrameAllocator(Mutex::new(SimpleFrameAllocatorInner {
            areas: None,
            current_area: 0,
            next_frame: Frame(0x0),
            kernel_prohibited_memory_ranges: [ProhibitedMemoryRange::empty(); KERNEL_PROHIBITED_MEM_RANGES_LEN],
            initialized: false,
        }))
    }

    #[cfg(test)]
    pub const fn new() -> Self {
        SimpleFrameAllocator(Mutex::new(SimpleFrameAllocatorInner {
            areas: None,
            current_area: 0,
            next_frame: Frame(0x0),
            kernel_prohibited_memory_ranges: [ProhibitedMemoryRange::empty(); KERNEL_PROHIBITED_MEM_RANGES_LEN],
            initialized: false,
        }))
    }
}

unsafe impl FrameAllocator for SimpleFrameAllocator {
    unsafe fn init(&self) -> Result<(), MemoryError> {
        assert_called_once!("Cannot call SimpleFrameAllocator::init() more than once");
        let allocator = &mut *self.0.lock();

        let mem_map_entries = KERNEL.mb_info().get_tag::<MemoryMap>()
            .ok_or(MemoryError::MemoryMapMbTagDoesNotExist)?
            .entries().map_err(MemoryError::MemoryMapErr)?;

        allocator.areas = Some(mem_map_entries);
        allocator.kernel_prohibited_memory_ranges = KERNEL.prohibited_memory_ranges();

        // get the first area of type `MemoryMapEntryType::AvailableRAM` with enough space
        // if this does not work, it means that we do not have enough physical memory (more specifically, available RAM)
        allocator.current_area = mem_map_entries.usable_areas().enumerate().find(|&(_, area)|
            area.aligned_length(FRAME_PAGE_SIZE) as usize >= FRAME_PAGE_SIZE
        ).ok_or(MemoryError::NotEnoughPhyMemory)?.0;

        // make sure that the allocator starts with a free frame (always aligned)
        allocator.next_frame = Frame::from_phy_addr(mem_map_entries.0[allocator.current_area].aligned_base_addr(FRAME_PAGE_SIZE) as PhysicalAddress);
        if allocator.is_frame_used() {
            allocator.get_next_free_frame()?;
        }

        allocator.initialized = true;
        Ok(())
    }

    fn allocate(&self) -> Result<Frame, MemoryError> {
        let allocator = &mut *self.0.lock();
        assert!(allocator.initialized);

        let frame = Ok(allocator.next_frame)?;
        allocator.get_next_free_frame()?;

        // physical address needs to be page aligned (just used to make sure that the frame allocator is behaving)
        if !frame.addr().is_multiple_of(FRAME_PAGE_SIZE) {
            return Err(MemoryError::FrameInvalidAllocatorAddr);
        }

        Ok(frame)
    }

    // This allocator does not support deallocation.
    fn deallocate(&self, _frame: Frame) {
        assert!(self.0.lock().initialized);
    }

    fn metadata_memory_range(&self) -> Option<ProhibitedMemoryRange> {
        assert!(self.0.lock().initialized);
        None
    }

    unsafe fn remap(&self) {
        assert!(self.0.lock().initialized);
        assert_called_once!("Cannot call SimpleFrameAllocator::remap() more than once");
    }
}

impl SimpleFrameAllocatorInner {
    fn is_frame_used(&self) -> bool {
        self.kernel_prohibited_memory_ranges.iter().any(|range|
            self.next_frame.addr() >= range.start_addr() && self.next_frame.addr() <= range.end_addr()
        )
    }

    /// Returns the next (free or used) frame if it exists.
    /// 
    /// This is an abstraction over the areas. With this, the frames may be seen as positions in a list.
    fn get_next_frame(&mut self) -> Result<Frame, MemoryError> {
        let areas = self.areas.unwrap().0;
        let curr_area = &areas[self.current_area];
        let fr_after_last_in_curr_area= Frame::from_phy_addr(
            (curr_area.aligned_base_addr(FRAME_PAGE_SIZE) + curr_area.aligned_length(FRAME_PAGE_SIZE)) as PhysicalAddress
        );

        // check if the next frame is pointing outside the current area
        if self.next_frame == fr_after_last_in_curr_area {
            self.current_area += 1;

            // get to the next area with available ram and enough space
            while self.current_area < areas.len() && (areas[self.current_area].entry_type() != MemoryMapEntryType::AvailableRAM || areas[self.current_area].aligned_length(FRAME_PAGE_SIZE) == 0) {
                self.current_area += 1;
            }

            // no more areas to use (ran out of usable memory)
            if self.current_area >= areas.len() {
                return Err(MemoryError::NotEnoughPhyMemory);
            }

            // get the first frame from the next area (always aligned)
            self.next_frame = Frame::from_phy_addr(areas[self.current_area].aligned_base_addr(FRAME_PAGE_SIZE) as PhysicalAddress);
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
