use super::MbTagHeader;

pub(crate) trait MbTag {
    unsafe fn from_base_tag(tag: &MbTagHeader) -> &Self;
}
