use super::{tag_trait::MbTag, MbTagHeader, TagType};
use core::marker::PhantomData;

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub(crate) struct MemoryMap<'a> {
    header: MbTagHeader,
    pub(crate) entry_size: u32,
    pub(crate) entry_version: u32,

    _mem: PhantomData<&'a ()>, // capture the entries lifetime
    entries: [MemoryMapEntry],
}

#[repr(C)]
pub(crate) struct MemoryMapEntry {
    pub(crate) base_addr: u64,
    pub(crate) length: u64,
    entry_type: u32,
    pub(crate) reserved: u32,
}

#[repr(u32)]
#[derive(Debug, PartialEq)]
pub(crate) enum MemoryMapEntryType {
    AvailableRAM,
    ACPIInformation,
    ReservedForHibernation,
    DefectiveRAM,
    Reserved(u32),
}

impl MemoryMapEntry {
    pub(crate) fn entry_type(&self) -> MemoryMapEntryType {
        match self.entry_type {
            1 => MemoryMapEntryType::AvailableRAM,
            3 => MemoryMapEntryType::ACPIInformation,
            4 => MemoryMapEntryType::ReservedForHibernation,
            5 => MemoryMapEntryType::DefectiveRAM,
            other => MemoryMapEntryType::Reserved(other)
        }
    }
}

#[derive(Debug)]
pub(crate) enum MemoryMapError {
    EntriesInvalidSize,
}

impl<'a> MemoryMap<'a> {
    pub(crate) fn entries(&self) -> Result<MemoryMapEntryIter, MemoryMapError> {
        // make sure that the dat in the tag is consistent
        if self.entry_size as usize != size_of::<MemoryMapEntry>() {
            return Err(MemoryMapError::EntriesInvalidSize);
        }

        Ok(MemoryMapEntryIter::new(self))
    }
}

impl<'a> MbTag for MemoryMap<'a> {
    const TAG_TYPE: TagType = TagType::MemoryMap;

    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata {
        base_tag.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 2
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MemoryMapEntryIter<'a> {
    entries: &'a [MemoryMapEntry],
    curr_mem_entry_idx: usize,
    entry_count: usize,
}

impl<'a> MemoryMapEntryIter<'a> {
    fn new(memory_map: &'a MemoryMap) -> Self {
        let entry_count = (memory_map.header.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 2) / size_of::<MemoryMapEntry>();

        MemoryMapEntryIter {
            entries: &memory_map.entries,
            curr_mem_entry_idx: 0,
            entry_count,
        }
    }
}

impl<'a> Iterator for MemoryMapEntryIter<'a> {
    type Item = &'a MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_mem_entry_idx >= self.entry_count {
            return None;
        }

        // go to the next entry and return the current one
        self.curr_mem_entry_idx += 1;
        return Some(&self.entries[self.curr_mem_entry_idx - 1]);
    }
}
