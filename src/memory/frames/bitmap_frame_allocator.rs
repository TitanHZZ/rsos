use crate::{data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, KERNEL_PROHIBITED_MEM_RANGES_LEN}, memory::{MemoryError, ProhibitedMemoryRange, FRAME_PAGE_SIZE}};
use crate::multiboot2::memory_map::{MemoryMap, MemoryMapEntry, MemoryMapEntryType};
use super::{Frame, FrameAllocator};
use spin::Mutex;

struct BitmapFrameAllocatorInner<'a> {
    // the first frame containing the bitmap
    bitmap: Option<BitmapRefMut<'a>>,

    // how many frames are being used by the bitmap
    // bitmap_frame_count: usize,

    // // available areas
    // areas: Option<&'static [MemoryMapEntry]>,
    // current_area: usize,
    // // next frame to be used
    // // this frame points to an area of type `AvailableRAM` and is not inside the prohibited memory ranges below
    // next_frame: Frame,
    // // memory ranges that we need to avoid using so we don't override important memory
    // kernel_prohibited_memory_ranges: [ProhibitedMemoryRange; KERNEL_PROHIBITED_MEM_RANGES_LEN],
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
    pub unsafe fn init_alloc(&self, kernel: &Kernel) -> Result<(), MemoryError> {
        // let allocator = &mut *self.0.lock();
        // let mem_map = kernel.mb_info().get_tag::<MemoryMap>().ok_or(MemoryError::MemoryMapMbTagDoesNotExist)?;
        // let mem_map_entries = mem_map.entries().map_err(|e| MemoryError::MemoryMapErr(e))?.0;
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
        // // let mut usable_frame_count = 0;
        // // for area in mem_map_entries {
        // //     if area.entry_type() == MemoryMapEntryType::AvailableRAM {
        // //         usable_frame_count += area.length as usize / FRAME_PAGE_SIZE;
        // //     }
        // // }
        // // usable_frame_count -= (kernel.mb_end() - kernel.mb_start() + 1) / FRAME_PAGE_SIZE;
        // // usable_frame_count -= (kernel.k_end() - kernel.k_start() + 1) / FRAME_PAGE_SIZE;
        // // serial_println!("Total frame count: {}", usable_frame_count);
        // // serial_println!("Required frames for bitmap: {}", usable_frame_count / FRAME_PAGE_SIZE);
        // // let mut index = 0;
        // // for area in mem_map_entries {
        // //     if area.entry_type() == MemoryMapEntryType::AvailableRAM {
        // //         if area.length as usize / FRAME_PAGE_SIZE >= usable_frame_count / FRAME_PAGE_SIZE {
        // //             serial_println!("Got usable area for bitmap: {}", index);
        // //             break;
        // //         }
        // //         index += 1;
        // //     }
        // // }
        // // make sure that the allocator starts with a free frame
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
