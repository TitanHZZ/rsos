use crate::data_structures::bitmap_ref_mut::BitmapRefMut;

struct PageAllocator<'a> {
    l1: [Option<BitmapRefMut<'a>>; 1042800],
}
