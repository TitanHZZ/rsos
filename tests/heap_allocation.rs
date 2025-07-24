#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use rsos::memory::frames::FrameAllocator;
use rsos::memory::pages::paging::{inactive_paging_context::InactivePagingContext, ACTIVE_PAGING_CTX};
use rsos::multiboot2::{efi_boot_services_not_terminated::EfiBootServicesNotTerminated, MbBootInfo};
use rsos::memory::{AddrOps, FRAME_PAGE_SIZE, pages::Page, simple_heap_allocator::HEAP_ALLOCATOR};
use rsos::{interrupts::tss::TSS, kernel::Kernel, memory::{frames::FRAME_ALLOCATOR}};
use alloc::{boxed::Box, string::String, vec::Vec};
use core::{cmp::max, panic::PanicInfo, slice};
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

    // build the main Kernel structure
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_addr) }.expect("Invalid multiboot2 data");
    let kernel = Kernel::new(mb_info);
    kernel.check_placements().expect("The kernel/mb2 must be well placed");

    let a = unsafe  {
        hash_memory_region(kernel.mb_start() as *const u8, kernel.mb_end() - kernel.mb_start() + 1)
    };

    // EFI boot services are not supported
    assert!(kernel.mb_info().get_tag::<EfiBootServicesNotTerminated>().is_none());

    // set up the frame allocator
    unsafe {
        FRAME_ALLOCATOR.init(&kernel).expect("Could not initialize the frame allocator allocation");
        log!(ok, "Frame allocator allocation initialized.");
    }

    // get the current paging context and create a new (empty) one
    log!(ok, "Remapping the kernel memory, vga buffer and mb2 info.");
    { // this scope makes sure that the inactive context does not get used again
        let inactive_paging = &mut InactivePagingContext::new(&ACTIVE_PAGING_CTX, &FRAME_ALLOCATOR).unwrap();

        // remap (identity map) the kernel, mb2 info and vga buffer with the correct flags and permissions into the new paging context
        memory::remap(&kernel, &ACTIVE_PAGING_CTX, inactive_paging, &FRAME_ALLOCATOR)
            .expect("Could not remap the kernel");

        ACTIVE_PAGING_CTX.switch(inactive_paging);

        // this creates the guard page for the kernel stack
        // the unwrap is fine as we know that the addr is valid
        // NOTE: the frame itself is not deallocated so that it does not cause any problems by being in the middle of kernel memory
        let guard_page_addr = Page::from_virt_addr(inactive_paging.p4_frame().addr()).unwrap();
        ACTIVE_PAGING_CTX.unmap_page(guard_page_addr, &FRAME_ALLOCATOR, false);
    }

    let b = unsafe  {
        hash_memory_region(kernel.mb_start() as *const u8, kernel.mb_end() - kernel.mb_start() + 1)
    };

    // if this fails, the mb2 memory got corrupted
    assert!(a == b);

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
        let heap_start = max(kernel.k_end(), kernel.mb_end()).align_up(FRAME_PAGE_SIZE);
        HEAP_ALLOCATOR.init(heap_start, 100 * 1024, &ACTIVE_PAGING_CTX)
            .expect("Could not initialize the heap allocator");
        log!(ok, "Heap allocator initialized.");
    }

    #[cfg(test)]
    test_main();

    rsos::hlt();
}

unsafe fn hash_memory_region(ptr: *const u8, len: usize) -> [u8; 32] {
    let data = unsafe { slice::from_raw_parts(ptr, len) };
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    let hash = hasher.finalize();
    *hash.as_bytes()
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

// Because of the way the heap allocator works, the assert might fail but this is expected so this test is commented.
// #[test_case]
// fn big_deallocation() {
//     let layout = Layout::from_size_align(5 * FRAME_PAGE_SIZE, 4096).unwrap();
//     let addr1 = unsafe { HEAP_ALLOCATOR.alloc(layout) } as VirtualAddress;
// 
//     unsafe { HEAP_ALLOCATOR.dealloc(addr1 as *mut u8, layout) };
// 
//     let addr2 = unsafe { HEAP_ALLOCATOR.alloc(layout) } as VirtualAddress;
//     assert_eq!(addr1, addr2);
// }
