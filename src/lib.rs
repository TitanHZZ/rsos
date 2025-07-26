#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

// TODO: the tests forr this file now fail in release mode with the changes to the linker script and kernel placement checks

pub mod data_structures;
pub mod multiboot2;
// pub mod vga_buffer;
pub mod interrupts;
pub mod io_port;
pub mod memory;
pub mod serial;
pub mod logger;
pub mod kernel;

use core::{panic::PanicInfo, arch::{global_asm, asm}};
use crate::io_port::IoPort;

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

/// # Safety
/// 
/// The caller (the asm) must ensure that `mb_boot_info` is non null and points to a valid Mb2 struct.  
/// This function may only be called once.
#[cfg(test)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_mb_boot_info_addr: *const u8) -> ! {
    test_main();
    hlt();
}
