use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
#[allow(dead_code)]
pub struct BasicMemoryInfo {
    header: MbTagHeader,
    pub mem_lower: u32,
    pub mem_upper: u32,
}

impl MbTag for BasicMemoryInfo {
    const TAG_TYPE: TagType = TagType::BasicMemoryInfo;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
