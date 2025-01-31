use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
pub(crate) struct EfiBootServicesNotTerminated {
    header: MbTagHeader,
}

impl MbTag for EfiBootServicesNotTerminated {
    const TAG_TYPE: TagType = TagType::EfiBootServicesNotTerminated;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {
        ()
    }
}
