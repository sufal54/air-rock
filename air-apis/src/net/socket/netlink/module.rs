use std::ffi::{c_int, c_void};

#[repr(C)]
pub struct sockaddr_nl {
    pub nl_family: u16,
    pub nl_pad: u16,
    pub nl_pid: u32,
    pub nl_groups: u32,
}

#[repr(C)]

pub struct nlattr {
    pub nla_len: u16,
    pub nla_type: u16,
}

#[repr(C)]
pub struct msghdr {
    pub msg_name: *mut c_void,
    pub msg_namelen: crate::socklen_t,
    pub msg_iov: *mut crate::iovec,
    pub msg_iovlen: usize,
    pub msg_control: *mut c_void,
    pub msg_controllen: usize,
    pub msg_flags: c_int,
}

#[repr(C)]
pub struct nlmsghdr {
    pub nlmsg_len: u32,
    pub nlmsg_type: u16,
    pub nlmsg_flags: u16,
    pub nlmsg_seq: u32,
    pub nlmsg_pid: u32,
}

#[repr(C)]

pub struct nlmsgerr {
    pub error: i32,
    pub msg: nlmsghdr,
}
