#[repr(C)]
pub struct genlmsghdr {
    pub cmd: u8,
    pub version: u8,
    pub reserved: u16,
}


