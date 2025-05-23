#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use rsos::{interrupts::{self, gdt::{self, Descriptor, NormalSegmentDescriptor, SystemSegmentDescriptor}, tss::{TssStackNumber, TSS, TSS_SIZE}}, multiboot2::acpi_new_rsdp::AcpiNewRsdp};
use rsos::{memory::frames::simple_frame_allocator::FRAME_ALLOCATOR, interrupts::{InterruptArgs, InterruptDescriptorTable}};
use rsos::multiboot2::{MbBootInfo, elf_symbols::{ElfSectionFlags, ElfSymbols, ElfSymbolsIter}, memory_map::MemoryMap};
use rsos::interrupts::gdt::{NormalDescAccessByteArgs, NormalDescAccessByte, SegmentDescriptor, SegmentFlags};
use rsos::interrupts::gdt::{SystemDescAccessByteArgs, SystemDescAccessByte, SystemDescAccessByteType, GDT};
use rsos::memory::{pages::paging::{inactive_paging_context::InactivePagingContext, ACTIVE_PAGING_CTX}};
use rsos::memory::{AddrOps, {FRAME_PAGE_SIZE, pages::{Page, simple_page_allocator::HEAP_ALLOCATOR}}};
use core::{arch::asm, cmp::max, panic::PanicInfo};
use rsos::{log, memory, println};
use alloc::boxed::Box;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log!(failed, "Kernel Panic occurred!");
    println!("{}", info);
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
//     println!("Memory areas:");
//     for entry in mem_map_entries {
//         println!(
//             "\tstart: 0x{:x}, length: {:.2} MB, type: {:?}",
//             entry.base_addr,
//             entry.length as f64 / 1024.0 / 1024.0,
//             entry.entry_type()
//         );
//     }
//     let total_memory: u64 = mem_map_entries.into_iter()
//         .filter(|entry| entry.entry_type() == MemoryMapEntryType::AvailableRAM)
//         .map(|entry| entry.length)
//         .sum();
//     println!(
//         "Total (available) memory: {} bytes ({:.2} GB)",
//         total_memory,
//         total_memory as f64 / 1024.0 / 1024.0 / 1024.0
//     );
// }

// TODO: look into stack probes
// TODO: the majority of this code could be put into lib.rs to minimize boilerplate in tests
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

    let mut code_seg = NormalSegmentDescriptor::new();
    code_seg.set_flags(SegmentFlags::LONG_MODE_CODE);
    code_seg.set_access_byte(NormalDescAccessByteArgs::new(NormalDescAccessByte::EXECUTABLE | NormalDescAccessByte::PRESENT | NormalDescAccessByte::IS_CODE_OR_DATA));

    let mut tss_seg = SystemSegmentDescriptor::new();
    tss_seg.set_access_byte(SystemDescAccessByteArgs::new(SystemDescAccessByte::PRESENT, SystemDescAccessByteType::TssAvailable64bit));

    let mut tss = Box::new(TSS::new());
    tss.new_stack(TssStackNumber::TssStack1, 4, true).expect("Could not create an interrupt stack");
    tss_seg.set_base(Box::leak(tss));
    tss_seg.set_limit(TSS_SIZE);

    // the unwraps() should be fine as we know that the gdt as space left for these 2 descriptors
    let mut gdt = Box::new(GDT::new());
    let code_seg_sel = gdt.new_descriptor(Descriptor::NormalDescriptor(&code_seg)).unwrap();
    let tss_seg_sel = gdt.new_descriptor(Descriptor::SystemDescriptor(&tss_seg)).unwrap();

    // set up the IDT
    let mut idt = Box::new(InterruptDescriptorTable::new());
    idt.breakpoint.set_fn(breakpoint_handler);

    idt.double_fault.set_fn(double_fault_handler);
    idt.double_fault.set_ist(TssStackNumber::TssStack1);

    interrupts::disable_pics();
    unsafe {
        GDT::load(Box::leak(gdt));
        TSS::load(tss_seg_sel);
        gdt::reload_seg_regs(code_seg_sel);
        InterruptDescriptorTable::load(Box::leak(idt));
        interrupts::enable_interrupts();
    }

    // trigger a breakpoint interrupt
    unsafe {
        asm!("int3");
    }

    // let acpi_new_rsdp = mb_info.get_tag::<AcpiNewRsdp>().expect("Could not get acpi new rsdp struct");

    #[cfg(test)]
    test_main();

    println!("Hello, World!");
    rsos::hlt();
}

extern "x86-interrupt" fn breakpoint_handler(args: InterruptArgs) {
    println!("Got breakpoint exception!");
    println!("{:#?}", args);
}

extern "x86-interrupt" fn double_fault_handler(args: InterruptArgs, error_code: u64) {
    println!("Got Double Fault exception!");
    println!("{:#?}", args);
    println!("error code: {}", error_code);

    rsos::hlt();
}

/*
 * Current physical memory layout (NOT UP TO DATE):
 *
 * +--------------------+ (higher addresses)
 * |      Unused        |
 * +--------------------+ 0x513000
 * |                    |
 * |      Kernel        |
 * |                    |
 * +--------------------+ 0x200000
 * |                    |
 * |   Multiboot Info   |
 * |                    |
 * +--------------------+ 0x1FF000
 * |      Unused        |
 * +--------------------+ 0x0B9000
 * |                    |
 * |    VGA Buffer      |
 * |                    |
 * +--------------------+ 0x0B8000
 * |      Unused        |
 * +--------------------+ 0x000000
 */
