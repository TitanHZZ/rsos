#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod data_structures;
pub mod multiboot2;
pub mod interrupts;
pub mod graphics;
pub mod io_port;
pub mod memory;
pub mod serial;
pub mod logger;
pub mod kernel;
pub mod macros;

use crate::memory::{MEMORY_SUBSYSTEM, VirtualAddress, frames::FrameAllocator, simple_heap_allocator::HEAP_ALLOCATOR};
use crate::memory::pages::{Page, PageAllocator, paging::inactive_paging_context::InactivePagingContext};
use crate::multiboot2::{MbBootInfo, efi_boot_services_not_terminated::EfiBootServicesNotTerminated};
use crate::{graphics::KLOGGER, io_port::IoPort, kernel::{KERNEL, Kernel}};
use core::{panic::PanicInfo, arch::{global_asm, asm}};

// add all the necessary asm set up and boot code (some of this code could probably be ported to Rust)
global_asm!(include_str!("boot.asm"), options(att_syntax));

/// Panic handler for when code panic in test mode.
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]");
    serial_println!("{}", info);
    exit_qemu(0x11);
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info);
}

pub trait Testable {
    fn run(&self);
}

impl<T: Fn()> Testable for T {
    fn run(&self) {
        serial_print!("{}... ", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn hlt() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Safety: The `isa-debug-exit` I/O device must exist in qemu and be 32 bits in size
pub fn exit_qemu(ret: u32) -> ! {
    IoPort::write_u32(0xF4, ret);

    // just in case it fails to exit
    // this could be a panic!() but, that would create recursive exit_qemu() calls
    hlt();
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }

    exit_qemu(0x10);
}

/// This is the Rust entry point into the OS.
/// 
/// # Safety
/// 
/// The caller must ensure that the function never gets called more than once.
#[cfg(test)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_mb_boot_info_addr: *const u8) -> ! {
    test_main();
    hlt();
}

// TODO: should this be part of the kernel structure itself?? (m,aybe this should be the *init*)
// TODO: explain what exactly gets initialized
/// Performs the most basic initialization.
/// 
/// # Safety
/// 
/// - **Must** be called before anything else gets done by the Kernel.
/// 
/// # Panics
/// 
/// If called more than once.
pub unsafe fn basic_initialization_process(mb_boot_info_phy_addr: *const u8) {
    // at this point, the cpu is running in 64 bit long mode
    // paging is enabled (including the NXE and WP bits) and we are using identity mapping with some higher half mappings
    assert_called_once!("Cannot call rsos::basic_initialization_process() more than once");

    // build the main Kernel structure
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_phy_addr) }.expect("Invalid multiboot2 data");
    unsafe { KERNEL.init(mb_info) };
    KERNEL.initial_checks().expect("The kernel/mb2 must be well placed and mapped");
    serial_println!("mb start     (higher half): {:#x}, mb end:     {:#x}", KERNEL.mb_start() + KERNEL.mb_lh_hh_offset(), KERNEL.mb_end() + KERNEL.mb_lh_hh_offset());

    // EFI boot services are not supported
    assert!(KERNEL.mb_info().get_tag::<EfiBootServicesNotTerminated>().is_none());

    // initialize the frame allocator
    unsafe { MEMORY_SUBSYSTEM.frame_allocator().init() }.expect("Could not initialize the frame allocator");
    serial_println!("Frame allocator initialized.");

    // initialize the first stage page allocator
    unsafe { MEMORY_SUBSYSTEM.page_allocator().init() }.expect("Could not initialize the first stage page allocator");
    serial_println!("First stage page allocator initialized.");

    // this scope makes sure that the inactive context does not get used again
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

    // use the new higher half mapped multiboot2
    let mb_boot_info_virt_addr = (mb_boot_info_phy_addr as VirtualAddress + KERNEL.mb_lh_hh_offset()) as *const u8;
    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_virt_addr) }.expect("Invalid higher half multiboot2 data");

    // rebuild the main Kernel structure (with the new multiboot2)
    unsafe { KERNEL.rebuild(mb_info) };
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

    KERNEL.assert_memory_integrity();
    log!(ok, "Kernel logger initialized.");
}
