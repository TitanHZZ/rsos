use super::{tag_trait::MbTag, MbTagHeader, TagType};
use core::ptr::{addr_of, slice_from_raw_parts};

#[repr(C)]
#[derive(ptr_meta::Pointee)]
pub struct MemoryMap {
    header: MbTagHeader,
    pub entry_size: u32,
    pub entry_version: u32,

    entries: [MemoryMapEntry],
}

#[repr(C)]
pub struct MemoryMapEntry {
    pub base_addr: u64,
    pub length: u64,
    entry_type: u32,
    pub reserved: u32,
}

#[repr(u32)]
#[derive(Debug, PartialEq)]
pub enum MemoryMapEntryType {
    AvailableRAM,
    ACPIInformation,
    ReservedForHibernation,
    DefectiveRAM,
    Reserved(u32),
}

impl MemoryMapEntry {
    pub fn entry_type(&self) -> MemoryMapEntryType {
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
pub enum MemoryMapError {
    EntriesInvalidSize,
}

impl MemoryMap {
    pub fn entries(&self) -> Result<MemoryMapEntries, MemoryMapError> {
        // make sure that the data in the tag is consistent
        if self.entry_size as usize != size_of::<MemoryMapEntry>() {
            return Err(MemoryMapError::EntriesInvalidSize);
        }

        // build the slice ref with the correct metadata
        let entry_count = (self.header.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 2) / size_of::<MemoryMapEntry>();
        let ptr = addr_of!(self.entries) as *const MemoryMapEntry;
        let entries = unsafe { &*slice_from_raw_parts(ptr, entry_count) };

        Ok(MemoryMapEntries(entries))
    }
}

impl MbTag for MemoryMap {
    const TAG_TYPE: TagType = TagType::MemoryMap;

    fn dst_size(base_tag: &MbTagHeader) -> Self::Metadata {
        base_tag.size as usize - size_of::<MbTagHeader>() - size_of::<u32>() * 2
    }
}

// wrapper to be able to implement IntoIterator and still have access to the slice
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct MemoryMapEntries(pub &'static [MemoryMapEntry]);

impl IntoIterator for MemoryMapEntries {
    type Item = &'static MemoryMapEntry;
    type IntoIter = MemoryMapEntryIter;

    fn into_iter(self) -> Self::IntoIter {
        MemoryMapEntryIter {
            entries: self.0,
            curr_mem_entry_idx: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct MemoryMapEntryIter{
    entries: &'static [MemoryMapEntry],
    curr_mem_entry_idx: usize,
}

impl Iterator for MemoryMapEntryIter {
    type Item = &'static MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_mem_entry_idx >= self.entries.len() {
            return None;
        }

        // go to the next entry and return the current one
        self.curr_mem_entry_idx += 1;
        return Some(&self.entries[self.curr_mem_entry_idx - 1]);
    }
}
