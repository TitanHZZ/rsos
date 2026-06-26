use crate::{assert_called_once, graphics::KLOGGER, log, memory::{self, MEMORY_SUBSYSTEM, frames::FrameAllocator, pages::{Page, PageAllocator, paging::inactive_paging_context::InactivePagingContext}, simple_heap_allocator::HEAP_ALLOCATOR}, multiboot2::{MbBootInfo, efi_boot_services_not_terminated::EfiBootServicesNotTerminated, memory_map::{MemoryMap, MemoryMapEntryType}}, serial_println};
use crate::{memory::MemoryError, multiboot2::elf_symbols::{ElfSectionFlags, ElfSymbols, ElfSymbolsIter}};
use crate::memory::{AddrOps, MemoryRange, VirtualAddress, FRAME_PAGE_SIZE};
use spin::lock_api::{RwLock, RwLockReadGuard};
use core::{ops::Deref, slice};

// static Kernel asserts
const _: () = assert!(Kernel::originally_identity_mapped().is_multiple_of(FRAME_PAGE_SIZE));
const _: () = assert!(Kernel::originally_higher_half_mapped().is_multiple_of(FRAME_PAGE_SIZE));

pub static KERNEL: Kernel = Kernel(RwLock::new(KernelInner {
    k_start : 0,
    k_end   : 0,
    prohibited_memory_ranges: [MemoryRange::empty(); Kernel::prohibited_mem_ranges_len()],
    mb_info : None,
    mb_start: 0,
    mb_end  : 0,
    initialized: false,
    mb2_hash: [0; 32],
}));

struct KernelInner {
    // kernel (physical addrs)
    k_start: usize,
    k_end: usize,

    // these are physical addrs
    prohibited_memory_ranges: [MemoryRange; Kernel::prohibited_mem_ranges_len()],

    // multiboot2 (physical addrs)
    mb_info: Option<MbBootInfo>, // this changes from before to after the higher half remapping
    mb_start: usize,
    mb_end: usize,

    initialized: bool,

    // used to check for memory corruption on the multiboot2 memory during the lower to higher half remapping
    mb2_hash: [u8; 32],
}

pub struct Kernel(RwLock<KernelInner>);

impl KernelInner {
    /// Create a [KernelInner] structure from the `mb_info`.
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

        serial_println!("kernel start (lower half) : {:#x},\t\tkernel end: {:#x}", k_start, k_end);
        serial_println!("kernel start (higher half): {:#x}, kernel end: {:#x}", k_start + Kernel::k_lh_hh_offset(), k_end + Kernel::k_lh_hh_offset());
        serial_println!("mb start     (lower half) : {:#x},\t\tmb end:     {:#x}", mb_start, mb_end);

        KernelInner {
            k_start,
            k_end,

            prohibited_memory_ranges: [
                MemoryRange::new(0, FRAME_PAGE_SIZE - 1), // to avoid problems with NULL ptrs and detect NULL derefs
                MemoryRange::new(k_start,  k_end),
                MemoryRange::new(mb_start, mb_end),
            ],

            mb_info: Some(mb_info),
            mb_start,
            mb_end,

            initialized: true,

            mb2_hash: Self::hash_memory_region(mb_start, mb_end - mb_start + 1)
        }
    }

    fn mb_lh_hh_offset(&self) -> usize {
        (self.k_end + Kernel::k_lh_hh_offset() - self.mb_start).align_up(FRAME_PAGE_SIZE)
    }

    fn hash_memory_region(ptr: VirtualAddress, len: usize) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(unsafe { slice::from_raw_parts(ptr as _, len) });
        *hasher.finalize().as_bytes()
    }

    /// This will perform the following checks:
    /// - Checks if the kernel `prohibited_memory_ranges()` are in an invalid memory place such as in an area that is not of type **AvailableRAM**.
    /// - Check if the kernel fits well in the original (temporary) higher half mapping.
    /// - Checks the linker configs and addresses.
    /// 
    /// If any of these fail, **Err([MemoryError::BadMemoryPlacement])** or **Err([MemoryError::BadTemporaryHigherHalfMapping])** will be returned.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn initial_checks(&self) -> Result<(), MemoryError> {
        assert!(self.initialized);

        let mem_map_entries = self.mb_info.as_ref().unwrap().get_tag::<MemoryMap>()
            .ok_or(MemoryError::MemoryMapMbTagDoesNotExist)?
            .entries().map_err(MemoryError::MemoryMapErr)?;

        // check `prohibited_memory_ranges()` placements
        if self.prohibited_memory_ranges.iter().any(|range|
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
        if (self.k_end - self.k_start) > Kernel::originally_higher_half_mapped() {
            return Err(MemoryError::BadTemporaryHigherHalfMapping);
        }

        // check linker configs and addresses
        unsafe extern "C" {
            static KERNEL_HH_START: u8;
            static KERNEL_LH_START: u8;
            static KERNEL_LH_HH_OFFSET: u8;
        }

        let k_hh_start     = unsafe { &KERNEL_HH_START as *const u8 as usize };
        let k_lh_start     = unsafe { &KERNEL_LH_START as *const u8 as usize };
        let k_lh_hh_offset = unsafe { &KERNEL_LH_HH_OFFSET as *const u8 as usize };

        if k_hh_start != Kernel::k_hh_start() || k_lh_start != Kernel::k_lh_start() || k_lh_hh_offset != Kernel::k_lh_hh_offset() {
            return Err(MemoryError::BadLinkerConfig);
        }

        Ok(())
    }
}

impl Kernel {
    // TODO: document this
    // TODO: make the errors as enums??
    // TODO: explain what exactly gets initialized
    pub unsafe fn init(&self, mb_boot_info_phy_addr: *const u8) {
        assert_called_once!("Cannot call Kernel::init() more than once");

        {
            let mut inner = self.0.write();
            assert!(!inner.initialized);

            // build the main Kernel structure
            let mb_info = unsafe { MbBootInfo::new(mb_boot_info_phy_addr) }.expect("Invalid multiboot2 data");
            *inner = KernelInner::new(mb_info);

            inner.initial_checks().expect("The kernel/mb2 must be well placed and mapped");
            serial_println!("mb start     (higher half): {:#x}, mb end:     {:#x}", inner.mb_start + inner.mb_lh_hh_offset(), inner.mb_end + inner.mb_lh_hh_offset());

            // EFI boot services are not supported
            assert!(inner.mb_info.as_ref().unwrap().get_tag::<EfiBootServicesNotTerminated>().is_none());
        }

        // initialize the frame allocator
        unsafe { MEMORY_SUBSYSTEM.frame_allocator().init() }.expect("Could not initialize the frame allocator");
        serial_println!("Frame allocator initialized.");

        // initialize the first stage page allocator
        unsafe { MEMORY_SUBSYSTEM.page_allocator().init() }.expect("Could not initialize the first stage page allocator");
        serial_println!("First stage page allocator initialized.");

        {
            serial_println!("Remapping the kernel, multiboot2 info and the frame allocator metadata to the higher half.");
            let active_paging_context = MEMORY_SUBSYSTEM.active_paging_context();
            let inactive_paging = &mut InactivePagingContext::new(active_paging_context).unwrap();

            // remap (to the higher half) the kernel, the mb2 info and the frame allocator metadata
            // with the correct flags and permissions into the new paging context
            memory::remap(active_paging_context, inactive_paging).expect("Could not perform the higher half remapping");
            serial_println!("Higher half remapping completed.");

            active_paging_context.switch(inactive_paging);

            // this creates the guard page for the kernel stack (the unwrap is fine as we know that the addr is valid)
            // the frame itself is not deallocated so that it does not cause any problems by being in the middle of kernel memory
            let guard_page_addr = Page::from_virt_addr(inactive_paging.p4_frame().addr() + Kernel::k_lh_hh_offset()).unwrap();
            active_paging_context.unmap_page(guard_page_addr, false).expect("Could not unmap a page for the kernel stack guard page");
            serial_println!("Stack guard page created at: {:#x}", guard_page_addr.addr());
        }

        // at this point, we are using a new paging context that maps the kernel, mb2 and frame allocator metadata to the higher half
        // the paging context created during the asm bootstrapping is now being used as stack for the kernel
        // except for the p4 table that is being used as a guard page
        // because of this, we now have just over 2MiB of stack

        {
            let mut inner = self.0.write();

            // use the new higher half mapped multiboot2
            let mb_boot_info_virt_addr = (mb_boot_info_phy_addr as VirtualAddress + inner.mb_lh_hh_offset()) as *const u8;
            let mb_info = unsafe { MbBootInfo::new(mb_boot_info_virt_addr) }.expect("Invalid higher half multiboot2 data");

            // fix the multiboot2 info in the main Kernel structure
            inner.mb_info = Some(mb_info);
        }

        serial_println!("Main kernel structure rebuilt.");

        // fix the frame allocator
        unsafe { MEMORY_SUBSYSTEM.frame_allocator().remap() }.expect("Could not remap the frame allocator");
        serial_println!("Frame allocator remapped.");

        // switch to the permanent page allocator
        unsafe { MEMORY_SUBSYSTEM.page_allocator().switch() };
        serial_println!("Page allocator switch performed.");

        // initialize the second stage page allocator
        unsafe { MEMORY_SUBSYSTEM.page_allocator().init() }.expect("Could not initialize the second stage page allocator");
        serial_println!("Second stage page allocator initialized.");

        // set up the heap allocator
        unsafe { HEAP_ALLOCATOR.init(25) }.expect("Could not initialize the heap allocator");
        serial_println!("Heap allocator initialized.");

        // TODO: this should be initialized as soon as possible
        unsafe { KLOGGER.init(255, 255, 255, 1) }.expect("Could not initialize the Kernel logger");
        serial_println!("Kernel logger initialized.");

        // assert memory integrity
        let inner = self.0.read();
        let m = KernelInner::hash_memory_region(inner.mb_lh_hh_offset() + inner.mb_start, inner.mb_end - inner.mb_start + 1);
        assert!(inner.mb2_hash == m);
        log!(ok, "Kernel logger initialized.");
    }

    /// All the memory ranges that **must be left untouched** meaning that these regions
    /// cannot be used for allocations in the physical (frame allocator) memory space.
    /// 
    /// These ranges live in available RAM.
    /// 
    /// There are no order guarantees for the memory ranges.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn prohibited_memory_ranges(&self) -> impl Deref<Target = [MemoryRange; Kernel::prohibited_mem_ranges_len()]> {
        let inner = self.0.read();
        assert!(inner.initialized);
        RwLockReadGuard::map(inner, |data| &data.prohibited_memory_ranges)
    }

    /// Kernel start address in physical memory.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn k_start(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.k_start
    }

    /// Kernel end address in physical memory.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn k_end(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.k_end
    }

    /// Multiboot2 info start address in physical memory.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn mb_start(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.mb_start
    }

    /// Multiboot2 info end address in physical memory.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn mb_end(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.mb_end
    }

    /// Get a reference to the internal [MbBootInfo] structure.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn mb_info(&self) -> impl Deref<Target = MbBootInfo> {
        let inner = self.0.read();
        assert!(inner.initialized);
        RwLockReadGuard::map(inner, |data| data.mb_info.as_ref().unwrap())
    }

    /// Get the offset between the higher half multiboot2 mapping and the lower half multiboot2 mapping.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn mb_lh_hh_offset(&self) -> usize {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        inner.mb_lh_hh_offset()
    }

    /// Get the start address for the frame allocator to use with higher half mappings.
    /// 
    /// # Panics
    /// 
    /// If called before [initialization](Kernel::init()).
    pub fn fa_hh_start(&self) -> VirtualAddress {
        let inner = &*self.0.read();
        assert!(inner.initialized);
        (inner.k_end + Kernel::k_lh_hh_offset() + (inner.mb_end - inner.mb_start)).align_up(FRAME_PAGE_SIZE)
    }

    /// Get the lower half kernel start address.
    pub const fn k_lh_start() -> VirtualAddress {
        0x1000000
    }

    /// Get the higher half kernel start address.
    pub const fn k_hh_start() -> VirtualAddress {
        0xFFFF800000000000
    }

    /// Get the last valid higher half address.
    pub const fn hh_end() -> VirtualAddress {
        // 0xFFFF800000000000 + ((2**48 // 2 - (2**30 * 512)) - 1)
        0xFFFFFF7FFFFFFFFF
    }

    /// Get the offset between the higher half kernel start and the lower half kernel start.
    pub const fn k_lh_hh_offset() -> usize {
        Self::k_hh_start() - Self::k_lh_start()
    }

    /// Represents the number of sequential bytes starting at address 0x0 that are identity mapped when the Rust code first starts.
    /// 
    /// It is guaranteed to be a multiple of [FRAME_PAGE_SIZE].
    pub const fn originally_identity_mapped() -> usize {
        // each table maps 4096 bytes, has 512 entries and there are 512 P1 page tables
        4096 * 512 * 512
    }

    /// Represents the number of sequential bytes starting at address [k_hh_start](Self::k_hh_start()) that are mapped to [k_lh_start](Self::k_lh_start()) when the Rust code first starts.
    /// 
    /// It is guaranteed to be a multiple of [FRAME_PAGE_SIZE].
    pub const fn originally_higher_half_mapped() -> usize {
        4096 * 512 * 8
    }

    pub const fn prohibited_mem_ranges_len() -> usize {
        3
    }
}
