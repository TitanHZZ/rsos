use super::MbTagHeader;

#[repr(C)]
pub(crate) struct MemoryMap {
    header: MbTagHeader,
    entry_size: u32,
    entry_version: u32,
    entries: [MemoryMapEntry],
}

#[repr(C)]
pub(crate) struct MemoryMapEntry {
    base_addr: u64,
    length: u64,
    entry_type: u32,
    reserved: u32,
}
