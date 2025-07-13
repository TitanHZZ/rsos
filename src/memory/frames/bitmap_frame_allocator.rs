use crate::{data_structures::bitmap_ref_mut::BitmapRefMut, kernel::{Kernel, ORIGINALLY_IDENTITY_MAPPED}, multiboot2::memory_map::MemoryMapEntries};
use crate::memory::{AddrOps, MemoryError, PhysicalAddress, ProhibitedMemoryRange, FRAME_PAGE_SIZE};
use crate::{serial_println, multiboot2::memory_map::MemoryMap};
use super::{Frame, FrameAllocator};
use spin::Mutex;

struct BitmapFrameAllocatorInner<'a> {
    mem_map_entries: Option<MemoryMapEntries>,

    // a reference to the bitmap
    bitmap: Option<BitmapRefMut<'a>>,
    next_free_frame: usize,

    prohibited_mem_range: ProhibitedMemoryRange,
}

unsafe impl<'a> Send for BitmapFrameAllocatorInner<'a> {}

pub struct BitmapFrameAllocator<'a>(Mutex<BitmapFrameAllocatorInner<'a>>);

impl<'a> Default for BitmapFrameAllocator<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> BitmapFrameAllocatorInner<'a> {
    fn addr_to_bit_idx(&self, addr: PhysicalAddress) -> Option<usize> {
        self.frame_to_bit_idx(Frame::from_phy_addr(addr))
    }

    fn frame_to_bit_idx(&self, frame: Frame) -> Option<usize> {
        let mem_map_entries = self.mem_map_entries.unwrap();

        // tracks the total number of usable frames already checked
        let mut frame_acc = 0;

        for area in mem_map_entries.usable_areas() {
            let start = area.aligned_base_addr(FRAME_PAGE_SIZE) as usize;
            let end   = start + area.aligned_length(FRAME_PAGE_SIZE) as usize - 1;

            if (frame.addr() >= start) && (frame.addr() <= end) {
                let offset = (frame.addr() - start) / FRAME_PAGE_SIZE;
                return Some(frame_acc + offset);
            }

            frame_acc += area.aligned_length(FRAME_PAGE_SIZE) as usize / FRAME_PAGE_SIZE;
        }

        None
    }

    fn bit_idx_to_frame(&self, mut bit_idx: usize) -> Option<Frame> {
        let mem_map_entries = self.mem_map_entries.unwrap();

        for area in mem_map_entries.usable_areas() {
            if bit_idx < area.aligned_length(FRAME_PAGE_SIZE) as usize / FRAME_PAGE_SIZE {
                return Some(Frame::from_phy_addr(area.aligned_base_addr(FRAME_PAGE_SIZE) as PhysicalAddress + bit_idx * FRAME_PAGE_SIZE));
            }

            bit_idx -= area.aligned_length(FRAME_PAGE_SIZE) as usize / FRAME_PAGE_SIZE;
        }

        None
    }

    /// Obtain the bit index for the next free frame to be used when the allocator allocated again.
    /// 
    /// This assumes that the current **self.next_free_frame** (before calling this), will be marked as used so, it is ignored meaning that,
    /// the value of **self.next_free_frame** is irrelevant.
    fn get_next_free_frame(&self) -> Option<usize> {
        let bitmap = self.bitmap.as_ref().unwrap();

        let next_free_frame = bitmap.iter()
            .skip(self.next_free_frame + 1)
            .enumerate()
            .find(|(_, bit)| !(*bit))
            .map(|(idx, _)| idx + self.next_free_frame + 1);

        if next_free_frame.is_some() {
            return next_free_frame;
        }

        bitmap.iter()
            .take(self.next_free_frame)
            .enumerate()
            .find(|(_, bit)| !(*bit))
            .map(|(idx, _)| idx)
    }
}

impl<'a> BitmapFrameAllocator<'a> {
    pub const fn new() -> Self {
        BitmapFrameAllocator (Mutex::new(BitmapFrameAllocatorInner {
            mem_map_entries: None,

            bitmap: None,
            next_free_frame: 0,

            prohibited_mem_range: ProhibitedMemoryRange::empty(),
        }))
    }
}

unsafe impl<'a> FrameAllocator for BitmapFrameAllocator<'a> {
    unsafe fn init(&self, kernel: &Kernel) -> Result<(), MemoryError> {
        let allocator = &mut *self.0.lock();
        let mem_map = kernel.mb_info().get_tag::<MemoryMap>().ok_or(MemoryError::MemoryMapMbTagDoesNotExist)?;

        allocator.mem_map_entries = Some(mem_map.entries().map_err(MemoryError::MemoryMapErr)?);
        let mem_map_entries = allocator.mem_map_entries.unwrap();

        // get the amount of frames available in valid RAM
        let usable_frame_count: usize = mem_map_entries.usable_areas()
            // make sure that we only count the space that can actually be used for frames (aligned to FRAME_PAGE_SIZE)
            .map(|area| area.aligned_length(FRAME_PAGE_SIZE) as usize / FRAME_PAGE_SIZE)
            .sum();

        let bitmap_bytes_count = usable_frame_count.align_up(8) / 8;

        // look for a suitable area to hold the bitmap
        let suitable_area = mem_map_entries.usable_areas().enumerate()
            // must be large enough and sit below the identity-mapped ceiling
            .filter(|&(_, area)|
                (area.aligned_length(FRAME_PAGE_SIZE) as usize >= bitmap_bytes_count) &&
                (area.aligned_base_addr(FRAME_PAGE_SIZE) as usize + bitmap_bytes_count - 1 < ORIGINALLY_IDENTITY_MAPPED)
            )
            // must not overlap any prohibited kernel range
            .find_map(|(idx, area)| {
                let area_start = area.aligned_base_addr(FRAME_PAGE_SIZE) as usize;
                let area_end   = area_start + area.aligned_length(FRAME_PAGE_SIZE) as usize - 1;

                let mut cursor_start = area_start;
                let mut cursor_end   = cursor_start + bitmap_bytes_count - 1;

                // the chosen region must not overlap with any of the prohibited regions
                while (cursor_end <= area_end) && (cursor_end < ORIGINALLY_IDENTITY_MAPPED) {
                    // https://stackoverflow.com/a/3269471/22836431
                    let overlaps = kernel.prohibited_memory_ranges().iter().any(|range|
                        cursor_start <= range.end_addr() && range.start_addr() <= cursor_end
                    );

                    if !overlaps {
                        return Some((idx, area, cursor_start as *mut u8));
                    }

                    cursor_start += FRAME_PAGE_SIZE;
                    cursor_end   += FRAME_PAGE_SIZE;
                }

                None
        });

        // this should not, realistically, happen
        // but in case it does, there is not really anything we can do as we just don't have enough, contiguous, memory
        if suitable_area.is_none() {
            return Err(MemoryError::NotEnoughPhyMemory);
        }

        // create the actual bitmap
        let (_, _, bitmap_start_addr) = suitable_area.unwrap();
        allocator.bitmap = Some(unsafe {
            BitmapRefMut::from_raw_parts_mut(bitmap_start_addr, bitmap_bytes_count, None)
        });

        // mark the prohibited kernel memory ranges as allocated
        for range in kernel.prohibited_memory_ranges() {
            // this *must* work
            let start_bit_idx = allocator.addr_to_bit_idx(range.start_addr()).unwrap();
            let bitmap = allocator.bitmap.as_mut().unwrap();

            for i in 0..range.frame_length() {
                bitmap.set(start_bit_idx + i, true);
            }
        }

        // the unwrap() *must* work
        // mark the bitmap memory itself as allocated
        let bitmap_frames_count = bitmap_bytes_count.align_up(FRAME_PAGE_SIZE) / FRAME_PAGE_SIZE;
        let start_bit_idx = allocator.addr_to_bit_idx(bitmap_start_addr as PhysicalAddress).unwrap();
        let bitmap = allocator.bitmap.as_mut().unwrap();
        for i in 0..bitmap_frames_count {
            bitmap.set(start_bit_idx + i, true);
        }

        allocator.next_free_frame = 0;
        allocator.next_free_frame = allocator.get_next_free_frame().ok_or(MemoryError::NotEnoughPhyMemory)?;

        let end_addr = bitmap_start_addr as PhysicalAddress + bitmap_frames_count * FRAME_PAGE_SIZE - 1;
        allocator.prohibited_mem_range = ProhibitedMemoryRange::new(bitmap_start_addr as PhysicalAddress, end_addr);

        serial_println!("Bitmap created! Starting ar addr : {:#x}", bitmap_start_addr as PhysicalAddress);

        Ok(())
    }

    fn allocate_frame(&self) -> Result<Frame, MemoryError> {
        let allocator = &mut *self.0.lock();
        let frame = allocator.bit_idx_to_frame(allocator.next_free_frame).ok_or(MemoryError::NotEnoughPhyMemory)?;
        allocator.bitmap.as_mut().unwrap().set(allocator.next_free_frame, true);
        allocator.next_free_frame = allocator.get_next_free_frame().ok_or(MemoryError::NotEnoughPhyMemory)?;

        serial_println!("Allocated frame: {:#x}", frame.0);

        Ok(frame)
    }

    // TODO: maybe it would make sense to check if the frame to be deallocated is in the kernel prohibited ranges
    fn deallocate_frame(&self, frame: Frame) {
        let allocator = &mut *self.0.lock();
        let bit_idx = allocator.frame_to_bit_idx(frame).unwrap_or_else(|| panic!("Got Invalid frame for deallocation: {:#x}", frame.0));

        let bitmap = allocator.bitmap.as_mut().unwrap();
        assert!(bitmap.get(bit_idx) == Some(true)); // make sure that the frame was previously allocated
        bitmap.set(bit_idx, false);

        serial_println!("Deallocated frame: {:#x}", frame.0);
    }

    fn prohibited_memory_range(&self) -> Option<ProhibitedMemoryRange> {
        let allocator = &mut *self.0.lock();
        Some(allocator.prohibited_mem_range)
    }
}
