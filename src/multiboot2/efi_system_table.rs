use super::{tag_trait::MbTag, MbTagHeader, TagType};

// I don't think this tag will even exist in this context as the kernel is 64bit.
// Maybe it could be removed?
#[repr(C)]
#[allow(dead_code)]
pub(crate) struct Efi32BitSystemTablePtr {
    header: MbTagHeader,
    pub(crate) pointer: u32,
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct Efi64BitSystemTablePtr {
    header: MbTagHeader,
    pub(crate) pointer: u64,
}

impl MbTag for Efi32BitSystemTablePtr {
    const TAG_TYPE: TagType = TagType::Efi32BitSystemTablePtr;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}

impl MbTag for Efi64BitSystemTablePtr {
    const TAG_TYPE: TagType = TagType::Efi64BitSystemTablePtr;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
