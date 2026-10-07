use std::ffi::{c_int, c_void};

type size_t = usize;
type ssize_t = isize;

unsafe extern "C" {

    unsafe fn socket(domain: c_int, type_: c_int, protocol: c_int) -> c_int;

    unsafe fn bind(
        socket: c_int,
        address: *const crate::sockaddr,
        address_len: crate::socklen_t,
    ) -> c_int;
    unsafe fn recv(socket: c_int, buf: *mut c_void, len: size_t, flags: c_int) -> ssize_t;
    unsafe fn close(fd: c_int) -> c_int;

    unsafe fn recvmsg(fd: c_int, msg: *mut crate::msghdr, flags: c_int) -> ssize_t;

    unsafe fn sendmsg(fd: c_int, msg: *const crate::msghdr, flags: c_int) -> ssize_t;
}

pub fn air_connect(domain: c_int, type_: c_int, protocol: c_int) -> c_int {
    unsafe { socket(domain, type_, protocol) }
}

pub fn air_bind(
    socket: c_int,
    address: *const crate::sockaddr,
    address_len: crate::socklen_t,
) -> c_int {
    unsafe { bind(socket, address, address_len) }
}

pub fn air_recv(socket: c_int, buf: *mut c_void, len: size_t, flags: c_int) -> ssize_t {
    unsafe { recv(socket, buf, len, flags) }
}

pub fn air_close(fd: c_int) -> c_int {
    unsafe { close(fd) }
}

pub fn air_recvmsg(fd: c_int, msg: *mut crate::msghdr, flags: c_int) -> ssize_t {
    unsafe { recvmsg(fd, msg, flags) }
}

pub fn air_sendmsg(fd: c_int, msg: *const crate::msghdr, flags: c_int) -> ssize_t {
    unsafe { sendmsg(fd, msg, flags) }
}
