use super::MbTagHeader;

#[repr(C)]
pub(crate) struct ImageLoadBasePhysicalAdress {
    header: MbTagHeader,
    load_base_addr: u32,
}
