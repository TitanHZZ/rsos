use super::{tag_trait::MbTag, MbTagHeader, TagType};

// AFAIK, EFI systems (including this one) should not have this tag.
// But i am not sure, so this will stay here.
#[repr(C)]
#[allow(dead_code)]
pub struct VbeInfo {
    header: MbTagHeader,
    pub vbe_mode: u16,
    pub vbe_interface_seg: u16,
    pub vbe_interface_off: u16,
    pub vbe_interface_len: u16,
    pub vbe_control_info: [u8; 512],
    pub vbe_mode_info: [u8; 256],
}

impl MbTag for VbeInfo {
    const TAG_TYPE: TagType = TagType::VbeInfo;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
