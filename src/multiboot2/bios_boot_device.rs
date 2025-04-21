use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
pub(crate) struct BiosBootDevice {
    header: MbTagHeader,
    pub(crate) biosdev: u32,
    pub(crate) partition: u32,
    pub(crate) sub_partition: u32,
}

impl MbTag for BiosBootDevice {
    const TAG_TYPE: TagType = TagType::BiosBootDevice;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
