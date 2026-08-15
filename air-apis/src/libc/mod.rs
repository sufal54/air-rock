use std::ffi::{c_char, c_uint};

unsafe extern "C" {
    unsafe fn if_nametoindex(ifname: *const c_char) -> c_uint;
}

pub fn air_if_name2idx(ifname: *const c_char) -> c_uint {
    unsafe { if_nametoindex(ifname) }
}
