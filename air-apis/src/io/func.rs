use std::ffi::c_int;

use crate::{nfds_t, pollfd};

unsafe extern "C" {
    unsafe fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int;

}

pub fn air_poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    unsafe { poll(fds, nfds, timeout) }
}
