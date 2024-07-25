// for more information about the multiboot2 standard and the boot information structures, please see:
// https://wiki.osdev.org/Multiboot
// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

use core::marker::PhantomData;
use core::ptr;

#[repr(u32)]
#[allow(dead_code)]
#[derive(PartialEq, Eq)]
pub enum TagType {
    End = 0,
    MemInfo = 4,
    MemMap = 6,
    // TODO: finish
}

#[derive(Debug)]
pub enum MbError {
    IllegalAddr,
    InvalidReserveValue,
}

pub struct MbInfoHeader {
    pub total_size: u32, // total size of boot information
    pub reserved: u32,   // always set to zero
}

/*
 * This represents all the Multiboot2 info (that we support) given by the bootloader.
 */
#[repr(C)]
pub struct MbInfo {
    pub header: MbInfoHeader,
    tags: [u8],
}

impl MbInfo {
    pub fn new<'a>(mb_boot_info_addr: usize) -> Result<&'a MbInfo, MbError> {
        // check for null or unaligned pointer
        let mb_header_ptr = mb_boot_info_addr as *const MbInfoHeader;
        if mb_header_ptr.is_null() || mb_header_ptr.align_offset(align_of::<u64>()) != 0 {
            return Err(MbError::IllegalAddr);
        }

        // check value that should be constant
        let mb_header = unsafe { &*(mb_boot_info_addr as *const MbInfoHeader) };
        if mb_header.reserved != 0 {
            return Err(MbError::InvalidReserveValue);
        }

        let mb_info = mb_boot_info_addr as *const ();
        let mb_info: *const MbInfo = ptr::from_raw_parts(mb_info, mb_header.total_size as usize);
        let mb_info = unsafe { &*mb_info };

        Ok(mb_info)
    }

    pub fn basic_mem_info_tag(&self) -> Option<&MbMemInfo> {
        self.tags()
            .find(|tag| tag.tag_type == TagType::MemInfo)
            .map(|tag| unsafe { &*(tag as *const MbTagHeader as *const MbMemInfo) })
    }

    pub fn mem_map_tag(&self) -> Option<&MbMemInfo> {
        self.tags()
            .find(|tag| tag.tag_type == TagType::MemMap)
            .map(|tag| unsafe { &*(tag as *const MbTagHeader as *const MbMemInfo) })
    }

    fn tags(&self) -> MbTagIter {
        MbTagIter::new(self.tags.as_ptr().cast())
    }
}

pub struct MbTagHeader {
    tag_type: TagType,
    size: u32,
}

#[repr(C)]
pub struct MbMemInfo {
    header: MbTagHeader,
    pub mem_lower: u32,
    pub mem_upper: u32,
}

#[repr(C)]
struct MbMemMap {
    header: MbTagHeader,
    entry_size: u32,
    entry_version: u32,
    entries: [MbMemMapEntry],
}

#[repr(C)]
struct MbMemMapEntry {
    base_addr: u64,
    length: u64,
    entry_type: u32,
    reserved: u32, // always 0
}

pub struct MbTagIter<'a> {
    pub current: *const MbTagHeader,
    _mem: PhantomData<&'a ()>, // remaining data for the tag
}

impl<'a> MbTagIter<'a> {
    fn new(ptr: *const MbTagHeader) -> MbTagIter<'a> {
        MbTagIter {
            current: ptr,
            _mem: PhantomData,
        }
    }
}

impl<'a> Iterator for MbTagIter<'a> {
    type Item = &'a MbTagHeader;

    fn next(&mut self) -> Option<Self::Item> {
        let tag = unsafe { &*self.current };

        match tag.tag_type {
            TagType::End => None,
            _ => {
                // return the current tag and get the next one
                let ptr_offset = (tag.size as usize + 7) & !7;

                self.current = unsafe {
                    self.current
                        .cast::<u8>()
                        .add(ptr_offset)
                        .cast::<MbTagHeader>()
                };

                Some(tag)
            }
        }
    }
}

// impl MultibootTag for MbMemInfo {
//     const TAG_TYPE: u32 = 0;
// }
