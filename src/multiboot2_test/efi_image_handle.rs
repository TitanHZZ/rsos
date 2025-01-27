use super::MbTagHeader;

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
