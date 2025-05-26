// https://wiki.osdev.org/RSDP
use super::{tag_trait::MbTag, MbTagHeader, TagType};

#[repr(C, packed)]
struct RsdpV2 {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32, // deprecated since version 2.0

    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[repr(C)]
pub struct AcpiNewRsdp {
    header: MbTagHeader,
    // rsdpv2: RsdpV2,
}

impl MbTag for AcpiNewRsdp {
    const TAG_TYPE: TagType = TagType::AcpiNewRsdp;

    fn dst_size(_base_tag: &MbTagHeader) -> Self::Metadata {}
}
