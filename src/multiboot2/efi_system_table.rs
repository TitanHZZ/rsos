use super::MbTagHeader;

#[repr(C)]
pub(crate) struct Efi32BitSystemTablePtr {
    header: MbTagHeader,
    pub(crate) pointer: u32,
}

#[repr(C)]
pub(crate) struct Efi64BitSystemTablePtr {
    header: MbTagHeader,
    pub(crate) pointer: u64,
}
