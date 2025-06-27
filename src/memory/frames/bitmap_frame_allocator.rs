use crate::{data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, ORIGINALLY_IDENTITY_MAPPED}};
use crate::{serial_println, multiboot2::memory_map::{MemoryMap, MemoryMapEntry, MemoryMapEntryType}};
use crate::memory::{AddrOps, MemoryError, PhysicalAddress, ProhibitedMemoryRange, FRAME_PAGE_SIZE};
use super::{Frame, FrameAllocator};
use spin::Mutex;

struct BitmapFrameAllocatorInner<'a> {
    mem_map_entries: Option<&'static [MemoryMapEntry]>,

    // a reference to the bitmap
    bitmap: Option<BitmapRefMut<'a>>,
    current_frame: usize,
}

unsafe impl<'a> Send for BitmapFrameAllocatorInner<'a> {}

pub struct BitmapFrameAllocator<'a>(Mutex<BitmapFrameAllocatorInner<'a>>);

impl<'a> BitmapFrameAllocator<'a> {
    pub const fn new() -> Self {
        BitmapFrameAllocator(Mutex::new(BitmapFrameAllocatorInner {
            mem_map_entries: None,

            bitmap: None,
            current_frame: 0,
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

        allocator.mem_map_entries = Some(mem_map.entries().map_err(|e| MemoryError::MemoryMapErr(e))?.0);
        let mem_map_entries = allocator.mem_map_entries.unwrap();

        // get the amount of frames available in valid RAM
        let usable_frame_count: usize = mem_map_entries.iter()
            .filter(|&area| area.entry_type() == MemoryMapEntryType::AvailableRAM)
            .map(|area| area.length as usize / FRAME_PAGE_SIZE)
            .sum();

        // `bitmap_frame_count` is the total number of frames that the bitmap will take (even if the last frame is not fully utilized by the bitmap)
        // `bitmap_bytes_count` is the total number of bytes that `bitmap_frame_count` will take
        // these are the sizes that actually need to be allocated since a frame allocator does not allocate less than a frame (so we need to align the size up)
        let bitmap_frame_count = usable_frame_count.align_up(FRAME_PAGE_SIZE) / FRAME_PAGE_SIZE;
        let bitmap_bytes_count = bitmap_frame_count * FRAME_PAGE_SIZE;

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
            .find_map(|(idx, area)| {
                let area_start = area.base_addr as usize;
                let area_end = area_start + area.length as usize - 1;

                let mut cursor_start = area_start;
                let mut cursor_end = cursor_start + bitmap_bytes_count - 1;

                // the chosen region must not overlap with any of the prohibited regions
                while cursor_end <= area_end {
                    // https://stackoverflow.com/a/3269471/22836431
                    let overlaps = kernel.prohibited_memory_ranges().iter().any(|range|
                        cursor_start <= range.end_addr() && range.start_addr() <= cursor_end
                    );

                    if !overlaps {
                        return Some((idx, area, cursor_start as *mut u8));
                    }

                    cursor_start += FRAME_PAGE_SIZE;
                    cursor_end = area_start + area.length as usize - 1;
                }

                None
        });

        // this should not, realistically, happen
        // but in case it does, there is not really anything we can do as we just don't have enough, contiguous, memory
        if suitable_area.is_none() {
            return Err(MemoryError::NotEnoughPhyMemory);
        }

        let (area_index, area_ref, bitmap_start_addr) = suitable_area.unwrap();

        // the real size of the bitmap is a bit for each frame of available RAM so it is `usable_frame_count / 8` when counted in bytes
        allocator.bitmap = Some(unsafe {
            BitmapRefMut::from_raw_parts_mut(bitmap_start_addr, usable_frame_count / 8)
        });

        serial_println!("Bitmap created!");

        // TODO: initialize `current_frame`
        // TODO: set the kernel prohibited ranges as allocated in the allocator
        // TODO: set the area for the bitmap itself as allocated in the allocator

        Ok(())
    }

    fn addr_to_bit_idx(&self, allocator: &BitmapFrameAllocatorInner<'a>, addr: PhysicalAddress) -> Option<usize> {
        self.frame_to_bit_idx(allocator, Frame::from_phy_addr(addr))
    }

    fn frame_to_bit_idx(&self, allocator: &BitmapFrameAllocatorInner<'a>, frame: Frame) -> Option<usize> {
        let mem_map_entries = allocator.mem_map_entries.unwrap();

        // tracks the total number of usable frames already checked
        let mut frame_acc = 0;

        for area in mem_map_entries.iter().filter(|area| area.entry_type() == MemoryMapEntryType::AvailableRAM) {
            let start = area.base_addr as PhysicalAddress;
            let end = start + area.length as usize - 1;

            if (frame.addr() >= start) && (frame.addr() <= end) {
                let offset = (frame.addr() - start) / FRAME_PAGE_SIZE;
                return Some(frame_acc + offset);
            }

            frame_acc += area.length as usize / FRAME_PAGE_SIZE;
        }

        None
    }

    fn bit_idx_to_frame(&self, allocator: &BitmapFrameAllocatorInner<'a>, mut bit_idx: usize) -> Option<Frame> {
        let mem_map_entries = allocator.mem_map_entries.unwrap();

        for area in mem_map_entries.iter().filter(|area| area.entry_type() == MemoryMapEntryType::AvailableRAM) {
            if bit_idx < area.length as usize / FRAME_PAGE_SIZE {
                return Some(Frame::from_phy_addr(area.base_addr as PhysicalAddress + bit_idx * FRAME_PAGE_SIZE));
            }

            bit_idx -= area.length as usize / FRAME_PAGE_SIZE;
        }

        None
    }
}

unsafe impl<'a> FrameAllocator for BitmapFrameAllocator<'a> {
    fn allocate_frame(&self) -> Result<Frame, MemoryError> {
        todo!()
    }

    fn deallocate_frame(&self, _frame: Frame) {
        todo!()
    }

    fn prohibited_memory_ranges(&self) -> Option<&[ProhibitedMemoryRange]> {
        None
    }
}
