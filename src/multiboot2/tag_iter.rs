use super::{MbTagHeader, TagType};

pub(crate) struct MbTagIter {
    curr_tag_addr: *const MbTagHeader,
}

impl MbTagIter {
    pub(crate) fn new(curr_tag_addr: *const MbTagHeader) -> Self {
        // Safety: This assumes that, because this *should* come from MbBootInfo, the pointer is valid (non null, aligned and points to valid tags).
        MbTagIter {
            curr_tag_addr,
        }
    }
}

impl Iterator for MbTagIter {
    type Item = &'static MbTagHeader;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_tag = unsafe { &*self.curr_tag_addr };
        match curr_tag.tag_type {
            TagType::End => None,
            _ => {
                // return the current tag and update the ptr to the next one
                let ptr_offset = ((curr_tag.size as usize + 7) & !7) as isize;
                self.curr_tag_addr = unsafe { self.curr_tag_addr.byte_offset(ptr_offset) };
                Some(curr_tag)
            }
        }
    }
}
