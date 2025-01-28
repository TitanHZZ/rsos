use super::MbTagHeader;

#[repr(C)]
pub(crate) struct EfiBootServicesNotTerminated {
    header: MbTagHeader,
}
