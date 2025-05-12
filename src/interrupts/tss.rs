// https://wiki.osdev.org/Task_State_Segment
use crate::memory::VirtualAddress;

// https://wiki.osdev.org/Task_State_Segment#Long_Mode
#[repr(C, packed)]
pub struct TSS {
    reserved_0: u32,
    rsp0: VirtualAddress,
    rsp1: VirtualAddress,
    rsp2: VirtualAddress,
    reserved_1: u32,
    reserved_2: u32,
    pub ist: [VirtualAddress; 7],
    reserved_3: u32,
    reserved_4: u32,
    reserved_5: u16,
    iopb: u16,
}

impl TSS {
    /// Creates a new, completly zeroed out, TSS struct.
    pub const fn new() -> Self {
        TSS {
            reserved_0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved_1: 0,
            reserved_2: 0,
            ist: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            reserved_5: 0,
            iopb: 0
        }
    }
}
