use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub(crate) struct Modules {
    header: MbTagHeader,
    mod_start: u32,
    mod_end: u32,
    string: [u8],
}

#[derive(Debug)]
pub(crate) enum ModulesError {
    StringMissingNull,
    StringNotUtf8,
}

impl Modules {
    pub(crate) fn string(&self) -> Result<&str, ModulesError> {
        // get the cstr using ffi and return it as a &str
        let cstr = core::ffi::CStr::from_bytes_until_nul(&self.string).map_err(|_| ModulesError::StringMissingNull)?;
        cstr.to_str().map_err(|_| ModulesError::StringNotUtf8)
    }
}

impl MbTag for Modules {
    const TAG_TYPE: TagType = TagType::Modules;

    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata {
        base_tag.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 2
    }
}
