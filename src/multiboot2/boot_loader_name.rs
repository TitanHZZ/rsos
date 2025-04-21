#![allow(dead_code)]

use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub(crate) struct BootLoaderName {
    header: MbTagHeader,
    string: [u8],
}

#[derive(Debug)]
pub(crate) enum BootLoaderNameError {
    StringMissingNull,
    StringNotUtf8,
}

impl BootLoaderName {
    pub(crate) fn string(&self) -> Result<&str, BootLoaderNameError> {
        // get the cstr using ffi and return it as a &str
        let cstr = core::ffi::CStr::from_bytes_until_nul(&self.string).map_err(|_| BootLoaderNameError::StringMissingNull)?;
        cstr.to_str().map_err(|_| BootLoaderNameError::StringNotUtf8)
    }
}

impl MbTag for BootLoaderName {
    const TAG_TYPE: TagType = TagType::BootLoaderName;

    fn dst_size(base_tag: &MbTagHeader) -> usize {
        base_tag.size as usize - size_of::<MbTagHeader>()
    }
}
