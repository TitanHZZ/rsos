use super::MbTagHeader;

#[repr(C)]
pub(crate) struct Modules {
    header: MbTagHeader,
    mod_start: u32,
    mod_end: u32,
    pub(crate) string: [u8],
}
