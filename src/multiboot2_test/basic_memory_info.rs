use super::MbTagHeader;

#[repr(C)]
pub(crate) struct BasicMemoryInfo {
    header: MbTagHeader,
    mem_lower: u32,
    mem_upper: u32,
}
