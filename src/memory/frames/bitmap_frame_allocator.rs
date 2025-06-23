use crate::{data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, ORIGINALLY_IDENTITY_MAPPED}, memory::{AddrOps, MemoryError, ProhibitedMemoryRange, FRAME_PAGE_SIZE}, multiboot2::memory_map::{MemoryMap, MemoryMapEntryType}, serial_println};
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

        // // in identity mapping, the virtual addrs and the physical addrs are the same
        // allocator.areas = Some(mem_map_entries);
        // allocator.kernel_prohibited_memory_ranges = kernel.prohibited_memory_ranges();

        // // get the first area of type `MemoryMapEntryType::AvailableRAM`
        // for area in mem_map_entries.iter().enumerate() {
        //     if area.1.entry_type() == MemoryMapEntryType::AvailableRAM {
        //         allocator.current_area = area.0;
        //         break;
        //     }
        // }

        // get the amount of frames available in valid RAM
        let mut usable_frame_count: usize = mem_map_entries.iter()
            .filter(|&area| area.entry_type() == MemoryMapEntryType::AvailableRAM)
            .map(|area| area.length as usize / FRAME_PAGE_SIZE)
            .sum();

        // avoid prohibited kernel memory regions
        for prohibited_area in kernel.prohibited_memory_ranges() {
            usable_frame_count -= (prohibited_area.end_addr() - prohibited_area.start_addr() + 1) / FRAME_PAGE_SIZE;
        }

        let bitmap_frame_count = usable_frame_count.align_up(FRAME_PAGE_SIZE) / FRAME_PAGE_SIZE;
        serial_println!("Total usable memory frame count: {}", usable_frame_count);
        serial_println!("Required frames for bitmap: {}", bitmap_frame_count);
        serial_println!("Total bitmap size in bits: {}", bitmap_frame_count * FRAME_PAGE_SIZE);

        let mut index = 0;
        for area in mem_map_entries.iter().filter(|&area| area.entry_type() == MemoryMapEntryType::AvailableRAM) {
            if (area.length as usize / FRAME_PAGE_SIZE >= usable_frame_count / FRAME_PAGE_SIZE) && ((area.base_addr + area.length) as usize <= ORIGINALLY_IDENTITY_MAPPED) {
                serial_println!("Got usable area for bitmap: {}", index);
                break;
            }

            index += 1;
        }

        let suitable_area = mem_map_entries.iter()
            .enumerate()
            .filter(|&(_, area)| area.entry_type() == MemoryMapEntryType::AvailableRAM)
            .find(|&(_, area)|
                (area.length as usize / FRAME_PAGE_SIZE >= usable_frame_count / FRAME_PAGE_SIZE) &&
                ((area.base_addr + area.length) as usize <= ORIGINALLY_IDENTITY_MAPPED)
            );

        // make sure that the allocator starts with a free frame
        // if allocator.is_frame_used() {
        //     allocator.get_next_free_frame()?;
        // }

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

// impl BitmapFrameAllocatorInner {
//     fn is_frame_used(&self) -> bool {
//         let mut result = false;
//         for prohibited_mem_range in self.kernel_prohibited_memory_ranges {
//             result |= self.next_frame.addr() >= prohibited_mem_range.start_addr() && self.next_frame.addr() <= prohibited_mem_range.end_addr();
//         }
//         result
//     }
//     /// Returns the next (free or used) frame if it exists.
//     /// 
//     /// This is an abstraction over the areas. With this, the frames may be seen as positions in a list.
//     fn get_next_frame(&mut self) -> Result<Frame, MemoryError> {
//         let areas = self.areas.unwrap();
//         let curr_area = &areas[self.current_area];
//         let fr_after_last_in_curr_area= Frame::from_phy_addr((curr_area.base_addr + curr_area.length) as _);
//         // check if the next frame is pointing outside the current area
//         if self.next_frame == fr_after_last_in_curr_area {
//             self.current_area += 1;
//             // get to the next area with available ram
//             while self.current_area < areas.len() && areas[self.current_area].entry_type() != MemoryMapEntryType::AvailableRAM {
//                 self.current_area += 1;
//             }
//             // no more areas to use (ran out of usable memory)
//             if self.current_area >= areas.len() {
//                 return Err(MemoryError::NotEnoughPhyMemory);
//             }
//             // get the first frame from the next area
//             self.next_frame = Frame::from_phy_addr(areas[self.current_area].base_addr as usize);
//         } else {
//             // get the next frame from the same (current) area
//             self.next_frame = Frame(self.next_frame.0 + 1);
//         }
//         Ok(self.next_frame)
//     }
//     fn get_next_free_frame(&mut self) -> Result<Frame, MemoryError> {
//         let mut fr = self.get_next_frame()?;
//         while self.is_frame_used() {
//             fr = self.get_next_frame()?;
//         }
//         Ok(fr)
//     }
// }
