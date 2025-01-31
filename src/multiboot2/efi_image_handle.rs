use super::{tag_trait::MbTag, MbTagHeader, TagType};

/*
 * I don't think this tag will even exist in this context as the kernel is 64bit.
 * Maybe it could be removed?
 */
#[repr(C)]
pub(crate) struct Efi32BitImageHandlePtr {
    header: MbTagHeader,
    pub(crate) pointer: u32,
}

#[repr(C)]
pub(crate) struct Efi64BitImageHandlePtr {
    header: MbTagHeader,
    pub(crate) pointer: u64,
}

impl MbTag for Efi32BitImageHandlePtr {
    const TAG_TYPE: TagType = TagType::Efi32BitImageHandlePtr;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {
        ()
    }
}

impl MbTag for Efi64BitImageHandlePtr {
    const TAG_TYPE: TagType = TagType::Efi64BitImageHandlePtr;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {
        ()
    }
}
