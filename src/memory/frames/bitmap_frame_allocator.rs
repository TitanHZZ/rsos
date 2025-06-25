use crate::{data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, ORIGINALLY_IDENTITY_MAPPED}, memory::{AddrOps, MemoryError, PhysicalAddress, ProhibitedMemoryRange, FRAME_PAGE_SIZE}, multiboot2::memory_map::{MemoryMap, MemoryMapEntryType}, serial_println};
use super::{Frame, FrameAllocator};
use spin::Mutex;

struct BitmapFrameAllocatorInner<'a> {
    //a rreference to the bitmap
    bitmap: Option<BitmapRefMut<'a>>,
}

unsafe impl<'a> Send for BitmapFrameAllocatorInner<'a> {}

pub struct BitmapFrameAllocator<'a>(Mutex<BitmapFrameAllocatorInner<'a>>);

impl<'a> BitmapFrameAllocator<'a> {
    pub const fn new() -> Self {
        BitmapFrameAllocator(Mutex::new(BitmapFrameAllocatorInner {
            bitmap: None,
        }))
    }
}

impl<'a> BitmapFrameAllocator<'a> {
    /// # Safety
    /// 
    /// Resets the frame allocator state.
    /// 
    /// However, it must be called (before any allocation) as the allocator expects it.
    pub unsafe fn init(&self, kernel: &Kernel) -> Result<(), MemoryError> {
        let allocator = &mut *self.0.lock();
        let mem_map = kernel.mb_info().get_tag::<MemoryMap>().ok_or(MemoryError::MemoryMapMbTagDoesNotExist)?;
        let mem_map_entries = mem_map.entries().map_err(|e| MemoryError::MemoryMapErr(e))?.0;

        // get the amount of frames available in valid RAM
        let usable_frame_count: usize = mem_map_entries.iter()
            .filter(|&area| area.entry_type() == MemoryMapEntryType::AvailableRAM)
            .map(|area| area.length as usize / FRAME_PAGE_SIZE)
            .sum();

        let bitmap_frame_count = usable_frame_count.align_up(FRAME_PAGE_SIZE) / FRAME_PAGE_SIZE;
        let bitmap_bytes_count = bitmap_frame_count * FRAME_PAGE_SIZE;
        serial_println!("Total usable memory frame count: {}", usable_frame_count);
        serial_println!("Required frames for bitmap: {}", bitmap_frame_count);
        serial_println!("Total bitmap size in bytes: {}", bitmap_bytes_count);

        // look for a suitable area to hold the bitmap
        let suitable_area = mem_map_entries.iter().enumerate()
            // must be available RAM
            .filter(|&(_, area)| area.entry_type() == MemoryMapEntryType::AvailableRAM)
            // must be large enough and sit below the identity-mapped ceiling
            .filter(|&(_, area)|
                (area.length as usize / FRAME_PAGE_SIZE >= bitmap_frame_count) &&
                (((area.base_addr + area.length) as usize) < ORIGINALLY_IDENTITY_MAPPED)
            )
            // must not overlap any prohibited kernel range
            .find(|&(_, area)| {
                let area_start = area.base_addr as usize;
                let area_end = area_start + area.length as usize - 1;

                // must fit before or after the prohibited memory range
                kernel.prohibited_memory_ranges().iter().all(|range|
                    (area_start + bitmap_bytes_count - 1 < range.start_addr()) ||
                    (range.end_addr() + 1 + bitmap_bytes_count <= area_end)
                )
        });

        // this should not, realistically, happen
        // but in case it does, there is not really anything we can do as we just don't have enough, contiguous, memory
        if suitable_area.is_none() {
            return Err(MemoryError::NotEnoughPhyMemory);
        }

        let (area_index, suitable_area) = suitable_area.unwrap();
        serial_println!("Found valid area: {}", area_index);

        Ok(())
    }
}

unsafe impl<'a> FrameAllocator for BitmapFrameAllocator<'a> {
    fn allocate_frame(&self) -> Result<Frame, MemoryError> {
        // let allocator = &mut *self.0.lock()
        // let frame = Ok(allocator.next_frame)?;
        // allocator.get_next_free_frame()?;
        // // physical address needs to be page aligned (just used to make sure that the frame allocator is behaving)
        // if frame.addr() % FRAME_PAGE_SIZE != 0 {
        //     return Err(MemoryError::FrameInvalidAllocatorAddr);
        // }

        Ok(Frame(0))
    }

    fn deallocate_frame(&self, _frame: Frame) {
        todo!()
    }

    fn prohibited_memory_ranges(&self) -> Option<&[ProhibitedMemoryRange]> {
        None
    }
}
