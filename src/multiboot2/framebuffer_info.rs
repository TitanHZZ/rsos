use super::{tag_trait::MbTag, MbTagHeader, TagType};
use crate::memory::PhysicalAddress;

#[repr(u8)]
#[derive(Debug)]
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
struct FrameBufferPalette {
    red_value: u8,
    green_value: u8,
    blue_value: u8,
}

#[repr(C)]
struct ColorInfoIndexedColor {
    framebuffer_palette_num_colors: u32,
    framebuffer_palette: [FrameBufferPalette],
}

#[repr(C)]
struct ColorInfoDirectRGBColor {
    framebuffer_red_field_position: u8,
    framebuffer_red_mask_size: u8,
    framebuffer_green_field_position: u8,
    framebuffer_green_mask_size: u8,
    framebuffer_blue_field_position: u8,
    framebuffer_blue_mask_size: u8,
}

#[repr(C, packed)]
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
}

impl MbTag for FrameBufferInfo {
    const TAG_TYPE: TagType = TagType::FrameBufferInfo;

    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata {
        base_tag.size as usize - size_of::<MbTagHeader>()
    }
}
