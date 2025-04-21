#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use rsos::{memory::{frames::simple_frame_allocator::FRAME_ALLOCATOR, pages::paging::{inactive_paging_context::InactivePagingContext, ACTIVE_PAGING_CTX}}};
use rsos::multiboot2::{MbBootInfo, elf_symbols::{ElfSectionFlags, ElfSymbols, ElfSymbolsIter}, memory_map::MemoryMap};
use rsos::memory::{AddrOps, {FRAME_PAGE_SIZE, pages::{Page, simple_page_allocator::HEAP_ALLOCATOR}}};
use alloc::{boxed::Box, string::String, vec::Vec};
use core::{cmp::max, panic::PanicInfo, ptr::null};
use rsos::{log, memory};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rsos::test_panic_handler(info);
}

#[derive(Debug)]
#[repr(align(16))]
struct Aligned16(u64);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(mb_boot_info_addr: *const u8) -> ! {
    log!(ok, "Rust kernel code started.");
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_addr) }.expect("Invalid mb2 data");

    // get the necessary mb2 tags and data
    let mem_map: &MemoryMap          = mb_info.get_tag::<MemoryMap>().expect("Memory map tag is not present");
    let elf_symbols: &ElfSymbols     = mb_info.get_tag::<ElfSymbols>().expect("Elf symbols tag is not present");
    let elf_sections: ElfSymbolsIter = elf_symbols.sections().expect("Elf sections are invalid");

    // get the kernel start and end addrs
    let k_start = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
        .map(|s: _| s.addr()).min().expect("Elf sections is empty").align_down(FRAME_PAGE_SIZE);

    let k_end   = elf_sections.filter(|s: _| s.flags().contains(ElfSectionFlags::ELF_SECTION_ALLOCATED))
        .map(|s: _| s.addr() + s.size() as usize).max().expect("Elf sections is empty").align_up(FRAME_PAGE_SIZE) - 1;

    // get the mb2 info start and end addrs
    let mb_start = mb_info.addr().align_down(FRAME_PAGE_SIZE);
    let mb_end   = mb_start + mb_info.size() as usize - 1;
    let mb_end   = mb_end.align_up(FRAME_PAGE_SIZE) - 1;

    // set up the frame allocator
    let mem_map_entries = mem_map.entries().expect("Memory map entries are invalid").0;
    unsafe {
        FRAME_ALLOCATOR.init(mem_map_entries, k_start, k_end, mb_start, mb_end).expect("Could not initialize the frame allocator");
        log!(ok, "Frame allocator initialized.");
    }

    // get the current paging context and create a new (empty) one
    log!(ok, "Remapping the kernel memory, vga buffer and mb2 info.");
    { // this scope makes sure that the inactive context does not get used again
        let inactive_paging = &mut InactivePagingContext::new(&ACTIVE_PAGING_CTX, &FRAME_ALLOCATOR).unwrap();

        // remap (identity map) the kernel, mb2 info and vga buffer with the correct flags and permissions into the new paging context
        memory::kernel_remap(&ACTIVE_PAGING_CTX, inactive_paging, elf_sections, &FRAME_ALLOCATOR, &mb_info)
            .expect("Could not remap the kernel");
        ACTIVE_PAGING_CTX.switch(inactive_paging);

        // TODO: is this really necessary?
        // the unwrap is fine as we know that the addr is valid
        ACTIVE_PAGING_CTX.unmap_page(Page::from_virt_addr(inactive_paging.p4_frame().addr()).unwrap(), &FRAME_ALLOCATOR);
    }

    // at this point, we are using a new paging context that just identity maps the kernel, mb2 info and vga buffer
    // the paging context created during the asm bootstrapping is now being used as stack for the kernel
    // except for the p4 table that is being used as a guard page
    // because of this, we now have just over 2MiB of stack

    log!(ok, "Kernel remapping completed.");
    log!(ok, "Stack guard page created.");

    // set up the heap allocator
    unsafe {
        // we know that the addr of the vga buffer and the start of the kernel will never change at runtime
        // and that the addr of the kernel is bigger so, we only need to avoid the mb2 info struct
        // and thus, we can start the kernel heap at the biggest of the 2
        HEAP_ALLOCATOR.init(max(k_end, mb_end).align_up(FRAME_PAGE_SIZE), 100 * 1024, &ACTIVE_PAGING_CTX)
            .expect("Could not initialize the heap allocator");
        log!(ok, "Heap allocator initialized.");
    }

    test_main();
    loop {}
}

#[test_case]
fn simple_allocation() {
    let a = Box::new(42);
    let b = String::from("Hello, World!");
    assert_eq!(*a, 42);
    assert_eq!(b, "Hello, World!");
}

#[test_case]
fn large_vector() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }

    for i in 0..n {
        assert_eq!(vec[i], i);
    }

    // check the sum of the 'n' numbers
    assert_eq!(vec.iter().sum::<usize>(), (n - 1) * n / 2);
}

#[test_case]
fn bigger_alignment() {
    let a = Box::new(Aligned16(13));
    assert_eq!((*a).0, 13);
}

#[test_case]
fn deallocation() {
    let mut addr: *const i32 = null();
    {
        let a = Box::new(42);
        addr = &*a;
    }

    // allocate another Box with different size
    let b: Box<u64> = Box::new(13);
    assert_eq!(addr, &*b as *const u64 as *const i32);
}
