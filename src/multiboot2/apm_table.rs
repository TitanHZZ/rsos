use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
#[allow(dead_code)]
pub struct ApmTable {
    header: MbTagHeader,
    pub version: u16,
    pub cseg: u16,
    pub offset: u32,
    pub cseg_16: u16,
    pub dseg: u16,
    pub flags: u16,
    pub cseg_len: u16,
    pub cseg_16_len: u16,
    pub dseg_len: u16,
}

impl MbTag for ApmTable {
    const TAG_TYPE: TagType = TagType::ApmTable;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
