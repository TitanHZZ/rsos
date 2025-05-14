#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use rsos::{interrupts::tss::{TssStackNumber, TSS}, memory::{frames::simple_frame_allocator::FRAME_ALLOCATOR, pages::paging::{inactive_paging_context::InactivePagingContext, ACTIVE_PAGING_CTX}, VirtualAddress}, println, serial_println};
use rsos::multiboot2::{MbBootInfo, elf_symbols::{ElfSectionFlags, ElfSymbols, ElfSymbolsIter}, memory_map::MemoryMap};
use rsos::memory::{AddrOps, {FRAME_PAGE_SIZE, pages::{Page, simple_page_allocator::HEAP_ALLOCATOR}}};
use alloc::{boxed::Box, string::String, vec::Vec};
use core::{alloc::Layout, cmp::max, panic::PanicInfo};
use core::alloc::GlobalAlloc;
use rsos::{log, memory};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rsos::test_panic_handler(info);
}

#[derive(Debug)]
#[repr(align(16))]
struct Aligned16(u64);

/// # Safety
/// 
/// The caller (the asm) must ensure that `mb_boot_info` is non null and points to a valid Mb2 struct.  
/// This function may only be called once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(mb_boot_info_addr: *const u8) -> ! {
    // at this point, the cpu is running in 64 bit long mode
    // paging is enabled (including the NXE and WP bits) and we are using identity mapping
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

    // serial_println!("kernel: {:#x} : {:#x}", k_start, k_end);
    // serial_println!("MB:     {:#x} : {:#x}", mb_start, mb_end);

    #[cfg(test)]
    test_main();

    rsos::hlt();
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

    assert_eq!(vec.len(), n);
    for (i, &item) in vec.iter().enumerate() {
        assert_eq!(item, i);
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
    let addr: *const i32;
    {
        let a = Box::new(42);
        addr = &*a;
    }

    // allocate another Box with different size
    let b: Box<u64> = Box::new(13);
    assert_eq!(addr, &*b as *const u64 as *const i32);
}

#[test_case]
fn big_struct_small_align() {
    // the TSS struct has a unique combination of size (104 bytes) and align (1 byte)
    // and this cases some problems if the real_align and real_size are not calculated correctly
    let _tss = Box::new(TSS::new());
}

#[test_case]
fn bruh() {
    let layout = Layout::from_size_align(FRAME_PAGE_SIZE, 2048).unwrap();
    let stack1 = unsafe { HEAP_ALLOCATOR.alloc(layout) } as VirtualAddress;
    // let stack2 = unsafe { HEAP_ALLOCATOR.alloc(layout) } as VirtualAddress;
    // let stack3 = unsafe { HEAP_ALLOCATOR.alloc(layout) } as VirtualAddress;

    // serial_println!("stack addrs -> {:#x} : {:#x} : {:#x}", stack1, stack2, stack3);

    // unsafe {
    //     *(stack1 as *mut u64) = 10;
    //     *(stack2 as *mut u64) = 10;
    //     *(stack3 as *mut u64) = 10;
    // }

    // let addr2 = unsafe { HEAP_ALLOCATOR.alloc(layout) } as VirtualAddress;
    // assert_eq!(addr, addr2);

    // let mut tss = Box::new(TSS::new());
    // tss.new_stack(TssStackNumber::TssStack1, 4, true);
    // tss.new_stack(TssStackNumber::TssStack1, 4, true);
}
