use std::{
    ffi::CString,
    io, mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use air_apis::{
    AF_NETLINK, NETLINK_ROUTE, SOCK_RAW, air_connect, air_if_name2idx, nlmsghdr, sockaddr_nl,
};

pub struct Service {
    sock_fd: OwnedFd,
    if_index: u32,
    seq: u32,
}

impl Service {
    pub fn new(inteface: String) -> Self {
        let c_interface = CString::new(inteface).unwrap();
        let sock_fd = air_connect(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE);
        let if_index = air_if_name2idx(c_interface.as_ptr());
        if sock_fd < 0 {
            panic!("Failed to create Netlink socket");
        }
        Self {
            sock_fd: unsafe { OwnedFd::from_raw_fd(sock_fd) },
            if_index: if_index,
            seq: 1,
        }
    }

    pub fn set_interface_power(&mut self, is_on: bool) {
        let mut nlh = unsafe { mem::zeroed::<nlmsghdr>() };
        nlh.nlmsg_len = (mem::size_of::<nlmsghdr>() + mem::size_of::<libc::ifinfomsg>()) as u32;
        nlh.nlmsg_type = libc::RTM_NEWLINK;
        nlh.nlmsg_flags = (libc::NLM_F_REQUEST | libc::NLM_F_ACK) as u16;
        nlh.nlmsg_seq = self.seq;
        nlh.nlmsg_pid = 0;

        self.seq = self.seq.wrapping_add(1);

        let mut ifi = unsafe { mem::zeroed::<libc::ifinfomsg>() };

        ifi.ifi_family = libc::AF_UNSPEC as u8;
        ifi.ifi_type = 0;
        ifi.ifi_index = self.if_index as i32;

        ifi.ifi_flags = if is_on { libc::IFF_UP as u32 } else { 0 };

        ifi.ifi_change = libc::IFF_UP as u32;

        let mut addr = unsafe { mem::zeroed::<sockaddr_nl>() };

        addr.nl_family = libc::AF_NETLINK as u16;
        addr.nl_pid = 0;
        addr.nl_groups = 0;

        let mut iov = [
            libc::iovec {
                iov_base: &mut nlh as *mut _ as *mut libc::c_void,
                iov_len: mem::size_of::<libc::nlmsghdr>(),
            },
            libc::iovec {
                iov_base: &mut ifi as *mut _ as *mut libc::c_void,
                iov_len: mem::size_of::<libc::ifinfomsg>(),
            },
        ];

        let mut msg: libc::msghdr = unsafe { mem::zeroed() };

        msg.msg_name = &mut addr as *mut _ as *mut libc::c_void;
        msg.msg_namelen = mem::size_of::<libc::sockaddr_nl>() as u32;

        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = iov.len();

        let ret = unsafe { libc::sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if ret < 0 {
            let err = io::Error::last_os_error();
            panic!("{:?}", err);
        }

        let mut response = [0u8; 4096];

        let ret = unsafe {
            libc::recv(
                self.sock_fd.as_raw_fd(),
                response.as_mut_ptr() as *mut libc::c_void,
                response.len(),
                0,
            )
        };

        if ret < 0 {
            let err = io::Error::last_os_error();
            panic!("{:?}", err);
        }

        if ret as usize >= mem::size_of::<libc::nlmsghdr>() {
            let response_hdr = unsafe { &*(response.as_ptr() as *const libc::nlmsghdr) };

            if response_hdr.nlmsg_type == libc::NLMSG_ERROR as u16 {
                let error_ptr = unsafe { response.as_ptr().add(mem::size_of::<libc::nlmsghdr>()) }
                    as *const libc::nlmsgerr;

                let nlerr = unsafe { &*error_ptr };

                if nlerr.error != 0 {
                    let errno = -nlerr.error;

                    panic!(
                        "netlink error: {} ({})",
                        errno,
                        io::Error::from_raw_os_error(errno)
                    );
                }
            }
        }
    }
}
