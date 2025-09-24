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
pub enum FrameBufferInfoError {
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
#[derive(Debug, Clone, Copy)]
pub struct ColorInfoDirectRGBColor {
    pub red_field_position: u8,
    pub red_mask_size: u8,
    pub green_field_position: u8,
    pub green_mask_size: u8,
    pub blue_field_position: u8,
    pub blue_mask_size: u8,
}

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub struct FrameBufferInfo {
    header: MbTagHeader,
    pub phy_addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    framebuffer_type: u8,
    reserved: u16,
    color_info: [u8], // depends on framebuffer_type
}

impl FrameBufferInfo {
    /// Get the framebuffer type.
    pub fn get_type(&self) -> Result<FrameBufferType, FrameBufferInfoError> {
        match self.framebuffer_type {
            0 => Ok(FrameBufferType::IndexedColor),
            1 => Ok(FrameBufferType::DirectRGBColor),
            2 => Ok(FrameBufferType::EGAText),
            _ => Err(FrameBufferInfoError::UnknownFrameBufferType),
        }
    }

    /// Get the RGB color information.
    /// 
    /// Panics
    /// 
    /// If the [framebuffer type](FrameBufferInfo::get_type()) is not [FrameBufferType::DirectRGBColor].
    pub fn get_color_info(&self) -> &ColorInfoDirectRGBColor {
        assert!(self.get_type().unwrap() == FrameBufferType::DirectRGBColor);
        unsafe { &*(self.color_info.as_ptr() as *const ColorInfoDirectRGBColor) }
    }
}

impl MbTag for FrameBufferInfo {
    const TAG_TYPE: TagType = TagType::FrameBufferInfo;

    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata {
        base_tag.size as usize - size_of::<MbTagHeader>()
    }
}
