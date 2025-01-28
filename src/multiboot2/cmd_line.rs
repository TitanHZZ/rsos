use super::{tag_trait::MbTag, MbTagHeader};

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub(crate) struct CmdLine {
    header: MbTagHeader,
    string: [u8],
}

#[derive(Debug)]
pub(crate) enum CmdLineError {
    StringMissingNull,
    StringNotUtf8,
}

impl CmdLine {
    pub(crate) fn string(&self) -> Result<&str, CmdLineError> {
        // get the cstr using ffi and return it as a &str
        let cstr = core::ffi::CStr::from_bytes_until_nul(&self.string).map_err(|_| CmdLineError::StringMissingNull)?;
        cstr.to_str().map_err(|_| CmdLineError::StringNotUtf8)
    }
}

impl MbTag for CmdLine {
    fn dst_size(base_tag: &MbTagHeader) -> usize {
        base_tag.size as usize - size_of::<MbTagHeader>()
    }
}
