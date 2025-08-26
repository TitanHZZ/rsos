use crate::data_structures::mapped_rwlockreadguard::MappedRwLockReadGuard;
use crate::memory::{frames::FrameAllocator, AddrOps, ProhibitedMemoryRange, VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM};
use crate::{multiboot2::{memory_map::{MemoryMap, MemoryMapEntryType}, MbBootInfo}, serial_println, assert_called_once};
use crate::{memory::MemoryError, multiboot2::elf_symbols::{ElfSectionFlags, ElfSymbols, ElfSymbolsIter}};
use core::ops::Deref;
use spin::RwLock;

// each table maps 4096 bytes, has 512 entries and there are 512 P1 page tables
/// Represents the number of sequential bytes starting at address 0x0 that are identity mapped when the Rust code first starts.
/// 
/// It is guaranteed to be a multiple of FRAME_PAGE_SIZE.
pub const ORIGINALLY_IDENTITY_MAPPED: usize = 4096 * 512 * 512;
const _: () = assert!(ORIGINALLY_IDENTITY_MAPPED.is_multiple_of(FRAME_PAGE_SIZE));

/// Represents the number of sequential bytes starting at address KERNEL_HH_START that are mapped to KERNEL_LH_START when the Rust code first starts.
/// 
/// It is guaranteed to be a multiple of FRAME_PAGE_SIZE.
pub const ORIGINALLY_HIGHER_HALF_MAPPED: usize = 4096 * 512 * 8;
const _: () = assert!(ORIGINALLY_HIGHER_HALF_MAPPED.is_multiple_of(FRAME_PAGE_SIZE));

pub const KERNEL_PROHIBITED_MEM_RANGES_LEN: usize = 3;

pub static KERNEL: Kernel = Kernel(RwLock::new(KernelInner {
    k_start : 0,
    k_end   : 0,
    mb_info : None,
    mb_start: 0,
    mb_end  : 0,
    initialized: false,
}));

struct KernelInner {
    // kernel (physical addrs)
    k_start: usize,
    k_end: usize,

    // multiboot2 (physical addrs)
    mb_info: Option<MbBootInfo>,
    mb_start: usize,
    mb_end: usize,

    initialized: bool,
}

pub struct Kernel(RwLock<KernelInner>);

impl KernelInner {
    fn new(mb_info: MbBootInfo) -> Self {
        // get the necessary mb2 tags and data
        let elf_symbols: &ElfSymbols     = mb_info.get_tag::<ElfSymbols>().expect("Elf symbols tag is not present");
        let elf_sections: ElfSymbolsIter = elf_symbols.sections().expect("Elf sections are invalid");

        // get the kernel start and end addrs
        let k_start = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
            .map(|s: _| s.load_addr()).min().expect("Elf sections is empty").align_down(FRAME_PAGE_SIZE);

        let k_end   = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
            .map(|s: _| s.load_addr() + s.size() as usize).max().expect("Elf sections is empty").align_up(FRAME_PAGE_SIZE) - 1;

        // get the mb2 info start and end addrs
        let mb_start = mb_info.addr().align_down(FRAME_PAGE_SIZE);
        let mb_end   = mb_info.addr() + mb_info.size() as usize - 1;
        let mb_end   = mb_end.align_up(FRAME_PAGE_SIZE) - 1;

        serial_println!("kernel start (lower half):  {:#x}, kernel end: {:#x}", k_start, k_end);
        serial_println!("kernel start (higher half): {:#x}, kernel end: {:#x}", k_start + Kernel::k_lh_hh_offset(), k_end + Kernel::k_lh_hh_offset());
        serial_println!("mb start: {:#x}, mb end: {:#x}", mb_start, mb_end);

        KernelInner {
            k_start,
            k_end,

            mb_info: Some(mb_info),
            mb_start,
            mb_end,

            initialized: true,
        }
    }

    /// All the memory ranges that **must be left untouched** meaning that these regions
    /// cannot be used for allocations in the physical (frame allocator) memory space.
    /// 
    /// These ranges live in available RAM.
    /// 
    /// There are no order guarantees for the memory ranges.
    fn prohibited_memory_ranges(&self) -> [ProhibitedMemoryRange; KERNEL_PROHIBITED_MEM_RANGES_LEN] {
        [
            ProhibitedMemoryRange::new(0, FRAME_PAGE_SIZE - 1), // to avoid problems with NULL ptrs and detect NULL derefs
            ProhibitedMemoryRange::new(self.k_start,  self.k_end),
            ProhibitedMemoryRange::new(self.mb_start, self.mb_end),
        ]
    }
}

impl Kernel {
    /// Initialize the Kernel main structure.
    /// 
    /// # Safety
    /// 
    /// This operation is **NOT** thread safe.
    /// 
    /// # Panics
    /// 
    /// If called more than once.
    pub unsafe fn init(&self, mb_info: MbBootInfo) {
        assert_called_once!("Cannot call Kernel::init() more than once");
        *self.0.write() = KernelInner::new(mb_info);
    }

    /// Rebuilds the main kernel structure with the new, higher half, multiboot2 information structure.
    /// 
    /// # Safety
    /// 
    /// This operation is **NOT** thread safe.
    /// 
    /// # Panics
    /// 
    /// If called more than once.
    pub unsafe fn rebuild(&self, mb_info: MbBootInfo) {
        assert_called_once!("Cannot call Kernel::rebuild() more than once");
        *self.0.write() = KernelInner::new(mb_info);
    }

    /// This checks if the kernel `prohibited_memory_ranges()` are in an invalid memory
    /// place such as in an area that is not of type **AvailableRAM**.
    /// This will also check if the kernel fits well in the original (temporary) higher half mapping.
    /// 
    /// If any of these fail, **Err([MemoryError::BadMemoryPlacement])** or **Err([MemoryError::BadTemporaryHigherHalfMapping])** will be returned.
    pub fn check_placements(&self) -> Result<(), MemoryError> {
        let inner = &*self.0.read();
        assert!(inner.initialized);

        let mem_map_entries = inner.mb_info.as_ref().unwrap().get_tag::<MemoryMap>()
            .ok_or(MemoryError::MemoryMapMbTagDoesNotExist)?
            .entries().map_err(MemoryError::MemoryMapErr)?;

        // check `prohibited_memory_ranges()` placements
        if inner.prohibited_memory_ranges().iter().any(|range|
            mem_map_entries.into_iter()
            .filter(|&area| area.entry_type() != MemoryMapEntryType::AvailableRAM)
            .any(|area| {
                let area_start = area.aligned_base_addr(FRAME_PAGE_SIZE) as usize;
                let area_end   = area_start + area.aligned_length(FRAME_PAGE_SIZE) as usize - 1;
                
                area_start <= range.end_addr() && range.start_addr() <= area_end
            })
        ) {
            return Err(MemoryError::BadMemoryPlacement);
        }

        // check initial higher half mapping placement
        if (inner.k_end - inner.k_start) > ORIGINALLY_HIGHER_HALF_MAPPED {
            return Err(MemoryError::BadTemporaryHigherHalfMapping);
        }

        Ok(())
    }

    /// Kernel start address in physical memory.
    pub fn k_start(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.k_start
    }

    /// Kernel end address in physical memory.
    pub fn k_end(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.k_end
    }

    /// Multiboot2 info start address in physical memory.
    pub fn mb_start(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.mb_start
    }

    /// Multiboot2 info end address in physical memory.
    pub fn mb_end(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.mb_end
    }

    // the impl Deref "hides" the KernelInner type
    pub fn mb_info(&self) -> impl Deref<Target = MbBootInfo> {
        let inner = self.0.read();
        assert!(inner.initialized);
        MappedRwLockReadGuard::new(inner, |data| data.mb_info.as_ref().unwrap())
    }

    /// Get the offset between the higher half multiboot2 mapping and the lower half multiboot2 mapping.
    pub fn mb2_lh_hh_offset(&self) -> usize {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        (inner.k_end + Kernel::k_lh_hh_offset() - inner.mb_start).align_up(FRAME_PAGE_SIZE)
    }

    /// All the memory ranges that **must be left untouched** meaning that these regions
    /// cannot be used for allocations in the physical (frame allocator) memory space.
    /// 
    /// These ranges live in available RAM.
    /// 
    /// There are no order guarantees for the memory ranges.
    pub fn prohibited_memory_ranges(&self) -> [ProhibitedMemoryRange; KERNEL_PROHIBITED_MEM_RANGES_LEN] {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.prohibited_memory_ranges()
    }

    /// Get the lower half link time physical/virtual start address.
    pub fn k_lh_start() -> usize {
        // symbol defined in the linker script
        unsafe extern "C" {
            static KERNEL_LH_START: u32;
        }

        let k_lh_start = unsafe { &KERNEL_LH_START as *const u32 as usize };
        assert!(k_lh_start.is_multiple_of(FRAME_PAGE_SIZE));
        k_lh_start
    }

    /// Get the higher half link time virtual start address.
    /// 
    /// To get the higher half link time physical address, subtract `k_lh_hh_offset()`.
    pub fn k_hh_start() -> usize {
        // symbol defined in the linker script
        unsafe extern "C" {
            static KERNEL_HH_START: usize;
        }

        let k_hh_start = unsafe { &KERNEL_HH_START as *const usize as usize };
        assert!(k_hh_start.is_multiple_of(FRAME_PAGE_SIZE));
        k_hh_start
    }

    /// Get the offset between the higher half kernel mapping and the lower half kernel mapping.
    pub fn k_lh_hh_offset() -> usize {
        // symbol defined in the linker script
        unsafe extern "C" {
            static KERNEL_LH_HH_OFFSET: usize;
        }

        let k_lh_hh_offset = unsafe { &KERNEL_LH_HH_OFFSET as *const usize as usize };
        assert!(k_lh_hh_offset.is_multiple_of(FRAME_PAGE_SIZE));
        k_lh_hh_offset
    }

    // TODO: this should probably not be able to create deadlocks
    /// Get the offset between the higher half frame allocator mapping and the lower half frame allocator mapping.
    /// 
    /// # Panics
    /// 
    /// If [FRAME_ALLOCATOR.metadata_memory_range()](crate::memory::frames::FrameAllocator::metadata_memory_range()) is **None**.
    pub fn fa_lh_hh_offset(&self) -> usize {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        let metadata_mem_range = MEMORY_SUBSYSTEM.frame_allocator().metadata_memory_range()
            .expect("fa_lh_hh_offset() can only be called when using a frame allocator with a metadata range");
        (inner.k_end + Kernel::k_lh_hh_offset() + (inner.mb_end - inner.mb_start) - metadata_mem_range.start_addr()).align_up(FRAME_PAGE_SIZE)
    }
}
