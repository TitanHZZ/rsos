#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use rsos::memory::frames::FrameAllocator;
use rsos::memory::pages::paging::inactive_paging_context::InactivePagingContext;
use rsos::multiboot2::{efi_boot_services_not_terminated::EfiBootServicesNotTerminated, MbBootInfo};
use rsos::memory::{pages::Page, simple_heap_allocator::HEAP_ALLOCATOR, MEMORY_SUBSYSTEM, VirtualAddress};
use rsos::interrupts::tss::TSS;
use alloc::{boxed::Box, string::String, vec::Vec};
use core::{panic::PanicInfo, slice};
use rsos::{memory, kernel::{Kernel, KERNEL}};
use rsos::memory::pages::PageAllocator;

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
pub unsafe extern "C" fn main(mb_boot_info_phy_addr: *const u8) -> ! {
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_phy_addr) }.expect("Invalid multiboot2 data");

    // build the main Kernel structure
    unsafe { KERNEL.init(mb_info) };
    KERNEL.initial_checks().expect("The kernel/mb2 must be well placed and mapped");

    let a = unsafe  {
        hash_memory_region(KERNEL.mb_start(), KERNEL.mb_end() - KERNEL.mb_start() + 1)
    };

    // EFI boot services are not supported
    assert!(KERNEL.mb_info().get_tag::<EfiBootServicesNotTerminated>().is_none());

    // initialize the frame allocator
    unsafe { MEMORY_SUBSYSTEM.frame_allocator().init() }.expect("Could not initialize the frame allocator");

    // initialize the first stage page allocator
    unsafe { MEMORY_SUBSYSTEM.page_allocator().init() }.expect("Could not initialize the first stage page allocator");

    // this scope makes sure that the inactive context does not get used again
    {
        let active_paging_context = MEMORY_SUBSYSTEM.active_paging_context();
        let inactive_paging = &mut InactivePagingContext::new(active_paging_context).unwrap();

        // remap (to the higher half) the kernel, the mb2 info and the frame allocator metadata
        // with the correct flags and permissions into the new paging context
        memory::remap(active_paging_context, inactive_paging).expect("Could not perform the higher half remapping");

        active_paging_context.switch(inactive_paging);

        // this creates the guard page for the kernel stack (the unwrap is fine as we know that the addr is valid)
        // the frame itself is not deallocated so that it does not cause any problems by being in the middle of kernel memory
        let guard_page_addr = Page::from_virt_addr(inactive_paging.p4_frame().addr() + Kernel::k_lh_hh_offset()).unwrap();
        active_paging_context.unmap_page(guard_page_addr, false).expect("Could not unmap a page for the kernel stack guard page");
    }

    // at this point, we are using a new paging context that maps the kernel, mb2 and frame allocator metadata to the higher half
    // the paging context created during the asm bootstrapping is now being used as stack for the kernel
    // except for the p4 table that is being used as a guard page
    // because of this, we now have just over 2MiB of stack

    // use the new higher half mapped multiboot2
    let mb_boot_info_virt_addr = (mb_boot_info_phy_addr as VirtualAddress + KERNEL.mb_lh_hh_offset()) as *const u8;
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_virt_addr) }.expect("Invalid higher half multiboot2 data");

    // rebuild the main Kernel structure (with the new multiboot2)
    unsafe { KERNEL.rebuild(mb_info) };

    // fix the frame allocator
    unsafe { MEMORY_SUBSYSTEM.frame_allocator().remap() }.expect("Could not remap the frame allocator");

    // switch to the permanent page allocator
    unsafe { MEMORY_SUBSYSTEM.page_allocator().switch() };

    // initialize the second stage page allocator
    unsafe { MEMORY_SUBSYSTEM.page_allocator().init() }.expect("Could not initialize the second stage page allocator");

    // set up the heap allocator
    unsafe { HEAP_ALLOCATOR.init(25) }.expect("Could not initialize the heap allocator");

    let b = unsafe  {
        hash_memory_region(KERNEL.mb_lh_hh_offset() + KERNEL.mb_start(), KERNEL.mb_end() - KERNEL.mb_start() + 1)
    };

    // if this fails, the mb2 memory got corrupted
    assert!(a == b);

    #[cfg(test)]
    test_main();

    rsos::hlt();
}

unsafe fn hash_memory_region(ptr: VirtualAddress, len: usize) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(unsafe { slice::from_raw_parts(ptr as _, len) });
    *hasher.finalize().as_bytes()
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
