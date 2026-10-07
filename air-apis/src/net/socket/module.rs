use std::ffi::{c_int, c_void};

#[repr(C)]
pub struct sockaddr {
    pub sa_family: u16,
    pub sa_data: [i8; 14],
}

#[repr(C)]
pub struct sockaddr_ll {
    pub sll_family: u16,
    pub sll_protocol: u16,
    pub sll_ifindex: i32,
    pub sll_hatype: u16,
    pub sll_pkttype: u8,
    pub sll_halen: u8,
    pub sll_addr: [u8; 8],
}

#[repr(C)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: usize,
}
