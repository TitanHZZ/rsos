use crate::multiboot2::framebuffer_info::{ColorInfoDirectRGBColor, FrameBufferInfo, FrameBufferInfoError, FrameBufferType};
use crate::memory::{AddrOps, MemoryError, PhysicalAddress, VirtualAddress, FRAME_PAGE_SIZE, MEMORY_SUBSYSTEM};
use crate::memory::pages::page_table::page_table_entry::EntryFlags;
use crate::memory::pages::{Page, PageAllocator};
use crate::memory::frames::Frame;
use crate::kernel::KERNEL;

pub struct Framebuffer {
    // addrs
    phy_addr: PhysicalAddress,
    vir_addr: VirtualAddress,

    // screen 'configs'
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    pixel_width: u32, // pixel size in bytes

    // color 'configs'
    color_info: ColorInfoDirectRGBColor,
}

#[derive(Debug)]
pub enum FrameBufferError {
    WrongFrameBufferType,
    Non8BitFramebuffer,
    FrameBufferTagDoesNotExist,
    FrameBufferInfoErr(FrameBufferInfoError),
    MemoryErr(MemoryError),
}

// TODO: it would make sense to check where the framebuffer lives in memory
impl Framebuffer {
    pub fn new() -> Result<Self, FrameBufferError> {
        let mb_info = KERNEL.mb_info();
        let framebuffer = mb_info.get_tag::<FrameBufferInfo>().ok_or(FrameBufferError::FrameBufferTagDoesNotExist)?;

        // only RGB framebuffers are supported
        let fb_type = framebuffer.get_type().map_err(FrameBufferError::FrameBufferInfoErr)?;
        if fb_type != FrameBufferType::DirectRGBColor {
            return Err(FrameBufferError::WrongFrameBufferType);
        }

        // only 8bit framebuffers are supported
        let color_info = framebuffer.get_color_info();
        if color_info.red_mask_size != 8 || color_info.blue_mask_size != 8 || color_info.green_mask_size != 8 {
            return Err(FrameBufferError::Non8BitFramebuffer);
        }

        let framebuffer_page_size = (framebuffer.pitch as usize * framebuffer.height as usize).align_up(FRAME_PAGE_SIZE) / FRAME_PAGE_SIZE;
        let vir_addr = MEMORY_SUBSYSTEM.page_allocator().allocate_contiguous(framebuffer_page_size, false).map_err(FrameBufferError::MemoryErr)?.addr();
        for i in 0..framebuffer_page_size {
            let offset = i * FRAME_PAGE_SIZE;
            let frame = Frame::from_phy_addr(framebuffer.phy_addr as PhysicalAddress + offset);
            let page = Page::from_virt_addr(vir_addr + offset).map_err(FrameBufferError::MemoryErr)?;
            MEMORY_SUBSYSTEM.active_paging_context().map_page_to_frame(page, frame, EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE).map_err(FrameBufferError::MemoryErr)?;
        }

        Ok(Framebuffer {
            phy_addr: framebuffer.phy_addr as PhysicalAddress,
            vir_addr,
            pitch: framebuffer.pitch,
            width: framebuffer.width,
            height: framebuffer.height,
            bpp: framebuffer.bpp,
            pixel_width: (framebuffer.bpp / 8).into(),
            color_info: *color_info,
        })
    }

    pub fn put_pixel(&self, x: u32, y: u32, color: FrameBufferColor) {
        let pixel = (self.vir_addr + (x * self.pixel_width + y * self.pitch) as usize) as *mut u8;
        unsafe {
            pixel.byte_offset((self.color_info.red_field_position   / 8).into()).write_volatile(color.r); // red
            pixel.byte_offset((self.color_info.green_field_position / 8).into()).write_volatile(color.g); // green
            pixel.byte_offset((self.color_info.blue_field_position  / 8).into()).write_volatile(color.b); // blue
        }
    }
}

pub struct FrameBufferColor {
    r: u8,
    g: u8,
    b: u8,
}

impl FrameBufferColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        FrameBufferColor { r, g, b }
    }
}
