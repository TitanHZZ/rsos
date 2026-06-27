#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// https://www.reddit.com/r/rust/comments/req4w2/everything_you_never_wanted_to_know_about_linker/
// https://wiki.osdev.org/Higher_Half_x86_Bare_Bones
// https://mcyoung.xyz/2021/06/01/linker-script/
// https://wiki.osdev.org/Higher_Half_Kernel
// https://medium.com/@connorstack/how-does-a-higher-half-kernel-work-107194e46a64
// https://simonis.github.io/Memory/

// TODO: look into stack probes
// TODO: the majority of this code could be put into lib.rs to minimize boilerplate in tests

extern crate alloc;

use rsos::{interrupts::gdt::{NormalDescAccessByte, NormalDescAccessByteArgs, SegmentDescriptor, SegmentFlags}, kernel::KERNEL, serial_println};
use rsos::interrupts::gdt::{SystemDescAccessByteArgs, SystemDescAccessByte, SystemDescAccessByteType, GDT};
use rsos::{interrupts::{self, gdt::{Descriptor, NormalSegmentDescriptor, SystemSegmentDescriptor}}};
use rsos::{interrupts::{InterruptArgs, InterruptDescriptorTable}};
use rsos::{interrupts::tss::{TSS, TSS_SIZE, TssStackNumber}};
use core::{arch::asm, panic::PanicInfo};
use alloc::boxed::Box;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // log!(failed, "Kernel Panic occurred!");
    serial_println!("{}", info);
    rsos::hlt();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rsos::test_panic_handler(info);
}

// fn print_mem_status(mb_info: &MbBootInfo) {
//     let mem_map = mb_info.get_tag::<MemoryMap>().expect("Mem map tag is not present.");
//     let mem_map_entries = mem_map.entries().expect("Only 64bit mem map entries are supported.");
//     serial_println!("Memory areas:");
//     for entry in mem_map_entries {
//         serial_println!(
//             "\tstart: 0x{:x}, length: {:.2} MB, type: {:?}",
//             entry.base_addr,
//             entry.length as f64 / 1024.0 / 1024.0,
//             entry.entry_type()
//         );
//     }
//     let total_memory: u64 = mem_map_entries.usable_areas()
//         .map(|entry| entry.length)
//         .sum();
//     serial_println!(
//         "Total (available) memory: {} bytes ({:.2} GB)",
//         total_memory,
//         total_memory as f64 / 1024.0 / 1024.0 / 1024.0
//     );
// }

/// This is the Rust entry point into the OS.
/// 
/// # Safety
/// 
/// The caller must ensure that the function never gets called more than once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(mb_boot_info_phy_addr: *const u8) -> ! {
    unsafe { KERNEL.init(mb_boot_info_phy_addr) }

    // TODO: all these Box::leak will cause large memory usage if these tables keep being replaced and the previous memory is not deallocated
    //       this needs to be solved

    // TODO: should these descriptors be in the heap??

    let mut code_seg = NormalSegmentDescriptor::new();
    code_seg.set_flags(SegmentFlags::LONG_MODE_CODE);
    code_seg.set_access_byte(NormalDescAccessByteArgs::new(NormalDescAccessByte::EXECUTABLE | NormalDescAccessByte::PRESENT | NormalDescAccessByte::IS_CODE_OR_DATA));

    let mut tss_seg = SystemSegmentDescriptor::new();
    tss_seg.set_access_byte(SystemDescAccessByteArgs::new(SystemDescAccessByte::PRESENT, SystemDescAccessByteType::TssAvailable64bit));

    let mut tss = TSS::new();
    tss.new_stack(TssStackNumber::TssStack1, 4, true).expect("Could not create an interrupt stack");

    tss_seg.set_base(tss);
    tss_seg.set_limit(TSS_SIZE);

    // the unwraps() *should* be fine as we know that the gdt as space left for these 2 descriptors
    let mut gdt = GDT::new();
    let code_seg_sel = gdt.new_descriptor(Descriptor::NormalDescriptor(&code_seg)).unwrap();
    let tss_seg_sel = gdt.new_descriptor(Descriptor::SystemDescriptor(&tss_seg)).unwrap();

    // set up the IDT
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_fn(breakpoint_handler);
    idt.double_fault.set_fn(double_fault_handler);
    idt.double_fault.set_ist(TssStackNumber::TssStack1);

    interrupts::disable_pics();
    unsafe {
        gdt.load();
        TSS::load(tss_seg_sel);
        GDT::reload_seg_regs(code_seg_sel);
        InterruptDescriptorTable::load(Box::leak(idt));
        interrupts::enable_interrupts();
    }

    // trigger a breakpoint interrupt
    unsafe {
        asm!("int3");
    }

    // // to be used later
    // assert!(mb_info.get_tag::<AcpiNewRsdp>().is_some());

    #[cfg(test)]
    test_main();

    serial_println!("Hello, World!");
    rsos::hlt();
}

extern "x86-interrupt" fn breakpoint_handler(args: InterruptArgs) {
    serial_println!("Got breakpoint exception!");
    serial_println!("{:#?}", args);
}

extern "x86-interrupt" fn double_fault_handler(args: InterruptArgs, error_code: u64) {
    serial_println!("Got Double Fault exception!");
    serial_println!("{:#?}", args);
    serial_println!("error code: {}", error_code);
    rsos::hlt();
}
