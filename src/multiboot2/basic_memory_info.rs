use super::{tag_trait::MbTag, MbTagHeader};

#[repr(C)]
pub(crate) struct BasicMemoryInfo {
    header: MbTagHeader,
    pub(crate) mem_lower: u32,
    pub(crate) mem_upper: u32,
}

impl MbTag for BasicMemoryInfo {
    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {
        ()
    }
}
