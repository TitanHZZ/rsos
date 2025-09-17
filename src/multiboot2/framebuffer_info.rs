use crate::{memory::PhysicalAddress, serial_println};
use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum FrameBufferType {
    IndexedColor = 0,
    DirectRGBColor = 1,
    EGAText = 2,
    Unknown,
}

#[derive(Debug)]
pub enum FrameBufferError {
    UnknownFrameBufferType,
}

#[repr(C)]
#[allow(dead_code)]
struct FrameBufferPalette {
    red_value: u8,
    green_value: u8,
    blue_value: u8,
}

#[repr(C)]
#[allow(dead_code)]
struct ColorInfoIndexedColor {
    framebuffer_palette_num_colors: u32,
    framebuffer_palette: [FrameBufferPalette],
}

#[repr(C)]
#[derive(Debug)]
struct ColorInfoDirectRGBColor {
    framebuffer_red_field_position: u8,
    framebuffer_red_mask_size: u8,
    framebuffer_green_field_position: u8,
    framebuffer_green_mask_size: u8,
    framebuffer_blue_field_position: u8,
    framebuffer_blue_mask_size: u8,
}

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub struct FrameBufferInfo {
    header: MbTagHeader,
    framebuffer_addr: u64, // physical address
    framebuffer_pitch: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    framebuffer_bpp: u8,
    framebuffer_type: u8,
    reserved: u8,
    color_info: [u8], // depends on framebuffer_type
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

impl FrameBufferInfo {
    pub fn get_type(&self) -> Result<FrameBufferType, FrameBufferError> {
        match self.framebuffer_type {
            0 => Ok(FrameBufferType::IndexedColor),
            1 => Ok(FrameBufferType::DirectRGBColor),
            2 => Ok(FrameBufferType::EGAText),
            _ => Err(FrameBufferError::UnknownFrameBufferType),
        }
    }

    pub fn get_phy_addr(&self) -> PhysicalAddress {
        self.framebuffer_addr as PhysicalAddress
    }

    fn get_color_info(&self) -> &ColorInfoDirectRGBColor {
        assert!(self.get_type().unwrap() == FrameBufferType::DirectRGBColor);
        unsafe { &*(self.color_info.as_ptr() as *const ColorInfoDirectRGBColor) }
    }

    pub fn put_pixel(&self, x: u32, y: u32, color: FrameBufferColor) {
        // TODO: this should, obviously, not be an assert
        assert!(self.framebuffer_bpp == 24);

        let pixel_addr = self.get_phy_addr() + (x * self.framebuffer_width + y * self.framebuffer_pitch) as usize;
        let color_info = self.get_color_info();

        serial_println!("color info: {:#?}", color_info);

        // ((1 << tagfb->framebuffer_blue_mask_size) - 1) << tagfb->framebuffer_blue_field_position
        let pixel = pixel_addr as *mut u8;
        unsafe {
            pixel.byte_offset((color_info.framebuffer_red_mask_size / 8).into()).write_volatile(color.r);   // red
            pixel.byte_offset((color_info.framebuffer_green_mask_size / 8).into()).write_volatile(color.g); // green
            pixel.byte_offset((color_info.framebuffer_blue_mask_size / 8).into()).write_volatile(color.b);  // blue
        }
    }
}

impl MbTag for FrameBufferInfo {
    const TAG_TYPE: TagType = TagType::FrameBufferInfo;

    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata {
        base_tag.size as usize - size_of::<MbTagHeader>()
    }
}
