use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
pub(crate) struct ApmTable {
    header: MbTagHeader,
    pub(crate) version: u16,
    pub(crate) cseg: u16,
    pub(crate) offset: u32,
    pub(crate) cseg_16: u16,
    pub(crate) dseg: u16,
    pub(crate) flags: u16,
    pub(crate) cseg_len: u16,
    pub(crate) cseg_16_len: u16,
    pub(crate) dseg_len: u16,
}

impl MbTag for ApmTable {
    const TAG_TYPE: TagType = TagType::ApmTable;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
