mod entry;
mod table;

use super::{Frame, FrameAllocator, PhysicalAddress, VirtualAddress, PAGE_SIZE};
use crate::{print, println};
use core::{marker::PhantomData, ptr::NonNull, arch::asm};
use entry::EntryFlags;
use table::{Level4, Table, P4};

const ENTRY_COUNT: usize = 512; // 512 = 2^9 = log2(PAGE_SIZE), PAGE_SIZE = 4096
pub struct Page(usize); // this usize is the page index in the virtual memory

/* ----------------- SOME NOTES ON PAGE TABLE INDEX CALCULATION -----------------
 * Let´s assume this address: 0xdeadbeef, with 4KiB pages (12 bits)
 * The calculated page index is: 912091 (0xdeadbeef / PAGE_SIZE)
 *
 * Page table indexes to translate the addr:
 * 0xdeadbeef:
 *  p4_idx   -> 0    (0xdeadbeef >> 39 & 0o777)
 *  p3_idx   -> 3    (0xdeadbeef >> 30 & 0o777)
 *  p2_idx   -> 245  (0xdeadbeef >> 21 & 0o777)
 *  p1_idx   -> 219  (0xdeadbeef >> 12 & 0o777)
 *  page_idx -> 239  (0xdeadbeef & 0o777)
 *
 * To calculate the table indexes but with the page index instead:
 * idx:
 *  p4_idx   -> 0    (912091 >> (39 - 12) & 0o777)
 *  p3_idx   -> 3    (912091 >> (30 - 12) & 0o777)
 *  p2_idx   -> 245  (912091 >> (21 - 12) & 0o777)
 *  p1_idx   -> 219  (912091 >> (12 - 12) & 0o777)
 *
 * We need to subtract 12 because the page index is 4096 (4KiB) times smaller than the original addr.
 */
impl Page {
    fn from_virt_addr(addr: VirtualAddress) -> Page {
        // in x86_64, the top 16 bits of a virtual addr must be sign extension bits. if they are not, its an invalid addr
        assert!(addr < 0x0000_8000_0000_0000 || addr >= 0xffff_8000_0000_0000, "Invalid virtual address: 0x{:x}", addr);
        Page(addr / PAGE_SIZE)
    }

    fn addr(&self) -> VirtualAddress {
        self.0 * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.0 >> 27) & 0o777
    }

    fn p3_index(&self) -> usize {
        (self.0 >> 18) & 0o777
    }

    fn p2_index(&self) -> usize {
        (self.0 >> 9) & 0o777
    }

    fn p1_index(&self) -> usize {
        (self.0 >> 0) & 0o777
    }
}

/*
 * Safety: Raw pointers are not Send/Sync so `Paging` cannot be used between threads as it would cause data races.
 */
pub struct Paging {
    p4: NonNull<Table<Level4>>,

    // makes this struct `own` a `Table<Level4>`
    _marker: PhantomData<Table<Level4>>,
}

impl Paging {
    /*
     * Safety: This should be unsafe because the p4 addr will always be the same (at least for now),
     * and that means that creating multiple `Paging` objects could result in undefined behaviour
     * as all the `Paging` objects woulb be pointing to the same memory (and own it).
     */
    pub unsafe fn new() -> Self {
        Paging {
            // this can be unchecked as we know that the ptr is non null
            p4: NonNull::new_unchecked(P4),
            _marker: PhantomData,
        }
    }

    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn map_page_to_frame<A: FrameAllocator>(&mut self, page: Page, frame: Frame, frame_allocator: &mut A, flags: EntryFlags) {
        let p4 = self.p4_mut();
        let p3 = p4.create_next_table(page.p4_index(), frame_allocator);
        let p2 = p3.create_next_table(page.p3_index(), frame_allocator);
        let p1 = p2.create_next_table(page.p2_index(), frame_allocator);

        // the entry must be unused
        debug_assert!(!p1.entries[page.p1_index()].is_used());

        p1.entries[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
    }

    pub fn map_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &mut A, flags: EntryFlags) {
        // get a random (free) frame
        let frame = frame_allocator.allocate_frame().expect("Out of memory. Could not allocate new frame.");
        self.map_page_to_frame(page, frame, frame_allocator, flags);
    }

    pub fn map<A: FrameAllocator>(&mut self, virtual_addr: VirtualAddress, frame_allocator: &mut A, flags: EntryFlags) {
        let page = Page::from_virt_addr(virtual_addr);
        self.map_page(page, frame_allocator, flags);
    }

    /*
     * This will unmap a page and the respective frame.
     * If an invalidd page is given, it will simply be ignored as there is nothing to unmap.
     */
    // TODO: - free P1, P2 and P3 if they get empty
    //       - deallocate the frame
    pub fn unmap_page<A: FrameAllocator>(&mut self, page: Page, frame_allocator: &mut A) {
        // set the entry in p1 as unused and free the respective frame
        self.p4_mut().next_table(page.p4_index())
            .and_then(|p3: _| p3.next_table_mut(page.p3_index()))
            .and_then(|p2: _| p2.next_table_mut(page.p2_index()))
            .and_then(|p1: _| {
                let entry = &mut p1.entries[page.p1_index()];
                let frame = entry.pointed_frame();
                entry.set_unused();

                frame
            }).and_then(|frame| {
                // deallocate the frame
                // frame_allocator.deallocate_frame(frame);

                Some(()) // `and_then()` requires an Option to be returned
            });

        // invalidate the TLB entry
        unsafe {
            asm!("invlpg [{}]", in(reg) page.addr() as u64, options(nostack, preserves_flags));
        }
    }

    /*
     * This takes a Page and returns the respective Frame if the address is mapped.
     */
    fn translate_page(&self, page: Page) -> Option<Frame> {
        self.p4().next_table(page.p4_index())
            .and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1.entries[page.p1_index()].pointed_frame())
    }

    /*
     * Takes a virtual address and returns the respective physical address if it exists (if it is mapped).
     */
    pub fn translate(&self, virtual_addr: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_addr % PAGE_SIZE;
        let page = Page::from_virt_addr(virtual_addr);
        let frame = self.translate_page(page)?;

        Some(frame.addr() + offset)
    }
}

pub fn test_paging<A: FrameAllocator>(frame_allocator: &mut A) {
    let mut page_table = unsafe { Paging::new() };

    let virt_addr = 42 * 512 * 512 * PAGE_SIZE; // 42 th entry in p3
    let page = Page::from_virt_addr(virt_addr);
    let frame = frame_allocator.allocate_frame().expect("out of memory");

    println!("None = {:?}, map to {:?}", page_table.translate(virt_addr), frame);
    page_table.map_page_to_frame(page, frame, frame_allocator, EntryFlags::empty());
    println!("Some = {:?}", page_table.translate(virt_addr));
    println!("next free frame: {:?}", frame_allocator.allocate_frame());

    println!("-------------------");

    println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).addr() as *const u64) });
    page_table.unmap_page(Page::from_virt_addr(virt_addr), frame_allocator);
    // println!("virt addr contents: {:#x}", unsafe { *(Page::from_virt_addr(virt_addr).addr() as *const u64) });
    println!("None = {:?}", page_table.translate(virt_addr));
}
