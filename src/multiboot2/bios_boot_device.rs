use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
#[allow(dead_code)]
pub struct BiosBootDevice {
    header: MbTagHeader,
    pub biosdev: u32,
    pub partition: u32,
    pub sub_partition: u32,
}

impl MbTag for BiosBootDevice {
    const TAG_TYPE: TagType = TagType::BiosBootDevice;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
