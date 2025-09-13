// https://www.reddit.com/r/rust/comments/req4w2/everything_you_never_wanted_to_know_about_linker/
// https://wiki.osdev.org/Higher_Half_x86_Bare_Bones
// https://mcyoung.xyz/2021/06/01/linker-script/
// https://wiki.osdev.org/Higher_Half_Kernel
// https://medium.com/@connorstack/how-does-a-higher-half-kernel-work-107194e46a64
// https://simonis.github.io/Memory/

// Rust Docs problem tracking issue:
// https://github.com/rust-lang/rust-analyzer/issues/20356#issue-3284255455

#![no_std]
#![no_main]
#![feature(lazy_get)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(rsos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// TODO: look into stack probes
// TODO: the majority of this code could be put into lib.rs to minimize boilerplate in tests

extern crate alloc;

use rsos::{interrupts::{self, gdt::{self, Descriptor, NormalSegmentDescriptor, SystemSegmentDescriptor}, tss::{TssStackNumber, TSS, TSS_SIZE}}, kernel::KERNEL, memory::{frames::FrameAllocator, pages::PageAllocator, VirtualAddress, MEMORY_SUBSYSTEM}};
use rsos::{interrupts::gdt::{NormalDescAccessByteArgs, NormalDescAccessByte, SegmentDescriptor, SegmentFlags}, serial_print, serial_println};
use rsos::{multiboot2::{acpi_new_rsdp::AcpiNewRsdp, efi_boot_services_not_terminated::EfiBootServicesNotTerminated}, kernel::Kernel};
use rsos::multiboot2::{MbBootInfo, framebuffer_info::{FrameBufferColor, FrameBufferInfo}, memory_map::MemoryMap};
use rsos::interrupts::gdt::{SystemDescAccessByteArgs, SystemDescAccessByte, SystemDescAccessByteType, GDT};
use rsos::memory::{FRAME_PAGE_SIZE, pages::Page, simple_heap_allocator::HEAP_ALLOCATOR};
use rsos::memory::{pages::paging::{inactive_paging_context::InactivePagingContext}};
use rsos::memory::{frames::Frame, pages::page_table::page_table_entry::EntryFlags};
use rsos::{interrupts::{InterruptArgs, InterruptDescriptorTable}};
use core::{arch::asm, panic::PanicInfo, slice};
use rsos::{log, memory};
use alloc::boxed::Box;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log!(failed, "Kernel Panic occurred!");
    serial_println!("{}", info);
    rsos::hlt();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rsos::test_panic_handler(info);
}

fn print_mem_status(mb_info: &MbBootInfo) {
    let mem_map = mb_info.get_tag::<MemoryMap>().expect("Mem map tag is not present.");
    let mem_map_entries = mem_map.entries().expect("Only 64bit mem map entries are supported.");

    serial_println!("Memory areas:");
    for entry in mem_map_entries {
        serial_println!(
            "\tstart: 0x{:x}, length: {:.2} MB, type: {:?}",
            entry.base_addr,
            entry.length as f64 / 1024.0 / 1024.0,
            entry.entry_type()
        );
    }

    let total_memory: u64 = mem_map_entries.usable_areas()
        .map(|entry| entry.length)
        .sum();

    serial_println!(
        "Total (available) memory: {} bytes ({:.2} GB)",
        total_memory,
        total_memory as f64 / 1024.0 / 1024.0 / 1024.0
    );
}

/// This is the Rust entry point into the OS.
/// 
/// # Safety
/// 
/// The caller must ensure that the function never gets called more than once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(mb_boot_info_phy_addr: *const u8) -> ! {
    // at this point, the cpu is running in 64 bit long mode
    // paging is enabled (including the NXE and WP bits) and we are using identity mapping with some higher half mappings
    log!(ok, "Rust kernel code started.");

    let mb_info = unsafe { MbBootInfo::new(mb_boot_info_phy_addr) }.expect("Invalid multiboot2 data");
    print_mem_status(&mb_info);

    // build the main Kernel structure
    unsafe { KERNEL.init(mb_info) };
    KERNEL.initial_checks().expect("The kernel/mb2 must be well placed and mapped");
    serial_println!("mb start     (higher half): {:#x}, mb end:     {:#x}", KERNEL.mb_start() + KERNEL.mb_lh_hh_offset(), KERNEL.mb_end() + KERNEL.mb_lh_hh_offset());

    let a = unsafe  {
        hash_memory_region(KERNEL.mb_start(), KERNEL.mb_end() - KERNEL.mb_start() + 1)
    };

    // EFI boot services are not supported
    assert!(KERNEL.mb_info().get_tag::<EfiBootServicesNotTerminated>().is_none());

    // initialize the frame allocator
    unsafe { MEMORY_SUBSYSTEM.frame_allocator().init() }.expect("Could not initialize the frame allocator");
    log!(ok, "Frame allocator initialized.");

    // initialize the first stage page allocator
    unsafe { MEMORY_SUBSYSTEM.page_allocator().init() }.expect("Could not initialize the first stage page allocator");
    log!(ok, "First stage page allocator initialized.");

    // get the current paging context and create a new (empty) one
    log!(ok, "Remapping the kernel, multiboot2 info and the frame allocator metadata to the higher half.");
    { // this scope makes sure that the inactive context does not get used again
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
        serial_println!("Guard page addr: {:#x}", guard_page_addr.addr());
    }

    // at this point, we are using a new paging context that maps the kernel, mb2 and frame allocator metadata to the higher half
    // the paging context created during the asm bootstrapping is now being used as stack for the kernel
    // except for the p4 table that is being used as a guard page
    // because of this, we now have just over 2MiB of stack

    log!(ok, "Higher half remapping completed.");
    log!(ok, "Stack guard page created.");

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

    // TODO: this should be improved
    // set up the heap allocator
    unsafe {
        let heap_bytes_size = 100 * 1024;
        let heap_start = MEMORY_SUBSYSTEM.page_allocator().allocate_contiguous(heap_bytes_size / FRAME_PAGE_SIZE).unwrap().addr();
        HEAP_ALLOCATOR.init(heap_start, heap_bytes_size).expect("Could not initialize the heap allocator");
        log!(ok, "Heap allocator initialized.");
        serial_println!("Heap allocator initialized.");
    }

    // TODO: all these Box::leak will cause large memory usage if these tables keep being replaced and the previous memory is not deallocated
    //       this needs to be solved

    let mut code_seg = NormalSegmentDescriptor::new();
    code_seg.set_flags(SegmentFlags::LONG_MODE_CODE);
    code_seg.set_access_byte(NormalDescAccessByteArgs::new(NormalDescAccessByte::EXECUTABLE | NormalDescAccessByte::PRESENT | NormalDescAccessByte::IS_CODE_OR_DATA));

    let mut tss_seg = SystemSegmentDescriptor::new();
    tss_seg.set_access_byte(SystemDescAccessByteArgs::new(SystemDescAccessByte::PRESENT, SystemDescAccessByteType::TssAvailable64bit));

    let mut tss = Box::new(TSS::new());
    tss.new_stack(TssStackNumber::TssStack1, 4, true).expect("Could not create an interrupt stack");
    tss_seg.set_base(Box::leak(tss));
    tss_seg.set_limit(TSS_SIZE);

    // the unwraps() *should* be fine as we know that the gdt as space left for these 2 descriptors
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

    // to be used later
    let mb_info = KERNEL.mb_info();
    assert!(mb_info.get_tag::<AcpiNewRsdp>().is_some());

    let framebuffer = mb_info.get_tag::<FrameBufferInfo>().expect("Framebuffer tag is required");
    let fb_type = framebuffer.get_type().expect("Framebuffer type is unknown");
    serial_println!("Framebuffer type: {:#?}", fb_type);

    MEMORY_SUBSYSTEM.active_paging_context().identity_map(Frame::from_phy_addr(framebuffer.get_phy_addr()), EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE).unwrap();
    framebuffer.put_pixel(0, 0, FrameBufferColor::new(255, 255, 255));

    let b = unsafe  {
        hash_memory_region(KERNEL.mb_lh_hh_offset() + KERNEL.mb_start(), KERNEL.mb_end() - KERNEL.mb_start() + 1)
    };

    // if this fails, the mb2 memory got corrupted
    assert!(a == b);

    #[cfg(test)]
    test_main();

    serial_println!("Hello, World!");
    rsos::hlt();
}

// TODO: this should probably be part of the kernel so we could check integrity at any point
unsafe fn hash_memory_region(ptr: VirtualAddress, len: usize) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(unsafe { slice::from_raw_parts(ptr as _, len) });
    *hasher.finalize().as_bytes()
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
