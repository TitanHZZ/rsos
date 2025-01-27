use super::MbTagHeader;

#[repr(C)]
pub(crate) struct BootLoaderName {
    header: MbTagHeader,
    pub(crate) string: [u8],
}
