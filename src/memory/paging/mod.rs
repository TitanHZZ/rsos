mod entry;

use super::PAGE_SIZE;

const ENTRY_COUNT: usize = 512; // 2^9

pub struct Page {
    idx: usize,
}
