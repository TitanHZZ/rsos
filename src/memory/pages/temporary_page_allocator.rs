use crate::{data_structures::bitmap::Bitmap, memory::VirtualAddress};

pub struct TemporaryPageAllocator {
    bitmap: Bitmap<1>,
    start_addr: VirtualAddress,
}

impl TemporaryPageAllocator {
    /// Creates a new **TemporaryPageAllocator** that will allocate pages from `start_addr` onwards.
    fn new(start_addr: VirtualAddress) -> Self {
        TemporaryPageAllocator {
            bitmap: Bitmap::new(None),
            start_addr,
        }
    }
}
