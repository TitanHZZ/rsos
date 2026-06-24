#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, string::String, vec::Vec};
use rsos::{interrupts::tss::TSS, kernel::KERNEL};
use core::{panic::PanicInfo};

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
    unsafe { KERNEL.init(mb_boot_info_phy_addr) }

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
