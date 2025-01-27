use super::MbTagHeader;

#[repr(C)]
pub(crate) struct BiosBootDevice {
    header: MbTagHeader,
    biosdev: u32,
    partition: u32,
    sub_partition: u32,
}
