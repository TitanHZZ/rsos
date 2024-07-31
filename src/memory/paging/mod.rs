mod entry;
mod table;

const ENTRY_COUNT: usize = 512; // 512 = 2^9 = log2(PAGE_SIZE), PAGE_SIZE = 4096

pub type Page = usize; // this usize is the page index in the virtual memory
