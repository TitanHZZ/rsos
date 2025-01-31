use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
pub(crate) struct ImageLoadBasePhysicalAdress {
    header: MbTagHeader,
    load_base_addr: u32,
}

impl MbTag for ImageLoadBasePhysicalAdress {
    const TAG_TYPE: TagType = TagType::ImageLoadBasePhysicalAdress;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {
        ()
    }
}
