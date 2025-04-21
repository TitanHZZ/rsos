use super::{MbTagHeader, TagType};
use ptr_meta::Pointee;

pub trait MbTag: Pointee {
    const TAG_TYPE: TagType;

    // each tag must implement a valid dst_size()
    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata;

    /// # Safety
    /// 
    /// The caller must ensure that `base_tag` is of the type it will be casted to.
    unsafe fn from_base_tag(base_tag: &MbTagHeader) -> &Self {
        let ptr = core::ptr::addr_of!(*base_tag);
        let ptr = ptr_meta::from_raw_parts(ptr.cast(), Self::dst_size(base_tag));
        unsafe { &*ptr }
    }
}
