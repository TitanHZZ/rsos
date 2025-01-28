use super::MbTagHeader;

#[repr(C)]
pub(crate) struct ApmTable {
    header: MbTagHeader,
    version: u16,
    cseg: u16,
    offset: u32,
    cseg_16: u16,
    dseg: u16,
    flags: u16,
    cseg_len: u16,
    cseg_16_len: u16,
    dseg_len: u16,
}
