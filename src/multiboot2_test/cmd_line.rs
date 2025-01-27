use super::tag_trait::MbTag;
use crate::multiboot2_test::MbTagHeader;
use core::ptr::slice_from_raw_parts;

#[repr(C)]
pub(crate) struct CmdLine {
    header: MbTagHeader,
    pub(crate) string: [u8],
}

impl MbTag for CmdLine {
    unsafe fn from_base_tag(tag: &MbTagHeader) -> &Self {
        // construct the cmd line tag from raw bytes
        let ptr = tag as *const MbTagHeader as *const u8;
        let bytes: *const [u8] = slice_from_raw_parts(ptr, tag.size as usize);
        &*(bytes as *const CmdLine)
    }
}
