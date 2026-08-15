use std::{
    ffi::{CString, c_void},
    mem::{self, MaybeUninit},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    ptr::addr_of,
};

use air_apis::{
    AF_NETLINK, CTRL_CMD_GETFAMILY, GENL_ID_CTRL, NETLINK_GENERIC, NLM_F_REQUEST, SOCK_RAW,
    air_bind, air_connect, air_if_name2idx, genlmsghdr, nlmsghdr, sockaddr, sockaddr_nl, socklen_t,
};
use libc::{
    CTRL_ATTR_FAMILY_ID, CTRL_ATTR_FAMILY_NAME, NLM_F_ACK, NLMSG_DONE, NLMSG_ERROR, id_t, iovec,
    msghdr, nlattr, nlmsgerr, recvmsg, sendmsg,
};

use crate::helper::{gen_genl_hdr, gen_nl_hdr, gen_nla_hdr};

#[derive(Debug, Default)]
pub struct InterfaceInfo {
    // Interface identity
    pub ifname: Option<String>,
    pub ifindex: Option<u32>,
    pub iftype: Option<u32>,

    // PHY / wireless device
    pub wiphy: Option<u32>,
    pub wdev: Option<u64>,

    // Interface MAC
    pub mac: Option<String>,

    // Connection state
    pub connected: bool,

    // AP BSSID that this interface is connected to
    pub bssid: Option<[u8; 6]>,

    // Connected Wi-Fi network name
    pub ssid: Option<String>,

    // Channel / frequency
    pub frequency: Option<u32>,
    pub max_frequency: Option<u32>,
    pub channel: Option<u32>,
    pub channel_width: Option<u32>,

    // Signal
    pub signal_dbm: Option<i32>,
    pub signal_avg_dbm: Option<i32>,

    // Network
    pub mtu: Option<u32>,

    pub vif_radio_mask: Option<u32>,

    // nl80211
    pub generation: Option<u32>,
    pub four_addr: bool,

    mlo_support: bool,
    max_scan_ssids: Option<u32>,
}

pub struct Service {
    sock_fd: OwnedFd,
    interface: String,
    if_index: u32,
    seq: u32,
    nladdr: Option<sockaddr_nl>,
    nl80211_family_id: Option<u16>,
}

impl Service {
    pub fn new(interface: String) -> Self {
        let c_interface = CString::new(interface.clone()).unwrap();

        let sock_fd = air_connect(AF_NETLINK, SOCK_RAW, NETLINK_GENERIC);
        let if_index = air_if_name2idx(c_interface.as_ptr());
        if sock_fd < 0 {
            panic!("Failed to create Netlink socket");
        }
        Self {
            sock_fd: unsafe { OwnedFd::from_raw_fd(sock_fd) },
            interface,
            if_index: if_index,
            seq: 0,
            nl80211_family_id: None,
            nladdr: None,
        }
    }

    pub fn sock_bind(&mut self) {
        let mut addr = unsafe { mem::zeroed::<sockaddr_nl>() };
        addr.nl_family = AF_NETLINK as u16;
        addr.nl_pid = 0;
        addr.nl_groups = 0;

        let ret = air_bind(
            self.sock_fd.as_raw_fd(),
            &addr as *const _ as *const sockaddr,
            mem::size_of::<sockaddr_nl>() as socklen_t,
        );

        if ret < 0 {
            panic!("bind failed");
        }
        self.nl80211_family_id = self.get_family_id();

        self.nladdr = Some(addr);
    }

    fn get_family_id(&mut self) -> Option<u16> {
        let family_name = b"nl80211\0";

        self.seq = self.seq.wrapping_add(1);

        let nl_hdr = gen_nl_hdr(
            32,
            GENL_ID_CTRL.try_into().unwrap(),
            NLM_F_REQUEST.try_into().unwrap(),
            self.seq,
            0,
        );

        let genl_hdr = gen_genl_hdr(CTRL_CMD_GETFAMILY.try_into().unwrap());

        let nla_hdr = gen_nla_hdr(12, CTRL_ATTR_FAMILY_NAME.try_into().unwrap());

        let mut iov = [
            iovec {
                iov_base: nl_hdr.as_ptr() as *mut c_void,
                iov_len: nl_hdr.len(),
            },
            iovec {
                iov_base: genl_hdr.as_ptr() as *mut c_void,
                iov_len: genl_hdr.len(),
            },
            iovec {
                iov_base: nla_hdr.as_ptr() as *mut c_void,
                iov_len: nla_hdr.len(),
            },
            iovec {
                iov_base: family_name.as_ptr() as *mut c_void,
                iov_len: family_name.len(),
            },
        ];

        let mut msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };

        // msg.msg_name = &mut self.nladdr as *mut _ as *mut c_void;
        // msg.msg_namelen = std::mem::size_of::<sockaddr_nl>() as u32;
        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = iov.len();

        let sent = unsafe { sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if sent < 0 {
            return None;
        }

        let mut response = [0u8; 4096];

        let mut recv_iov = iovec {
            iov_base: response.as_mut_ptr() as *mut c_void,
            iov_len: response.len(),
        };

        let mut recv_msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };

        recv_msg.msg_iov = &mut recv_iov;
        recv_msg.msg_iovlen = 1;

        let received = unsafe { recvmsg(self.sock_fd.as_raw_fd(), &mut recv_msg, 0) };

        if received <= 0 {
            return None;
        }

        let received = received as usize;

        let mut offset = 0;

        while offset + 16 <= received {
            let nlmsg_len = u16::from_ne_bytes([response[offset], response[offset + 1]]) as usize;

            let nlmsg_type = u16::from_ne_bytes([response[offset + 2], response[offset + 3]]);

            if nlmsg_len < 16 {
                return None;
            }

            if offset + nlmsg_len > received {
                return None;
            }

            if nlmsg_type == 0x2 {
                if nlmsg_len < 20 {
                    return None;
                }

                let error = i32::from_ne_bytes([
                    response[offset + 16],
                    response[offset + 17],
                    response[offset + 18],
                    response[offset + 19],
                ]);

                println!("CTRL_CMD_GETFAMILY error = {}", error);

                return None;
            }

            if nlmsg_type == 0x3 {
                return None;
            }

            let mut attr_offset = offset + 16 + 4;

            let msg_end = offset + nlmsg_len;

            while attr_offset + 4 <= msg_end {
                let nla_len =
                    u16::from_ne_bytes([response[attr_offset], response[attr_offset + 1]]) as usize;

                let nla_type =
                    u16::from_ne_bytes([response[attr_offset + 2], response[attr_offset + 3]]);

                if nla_len < 4 {
                    return None;
                }

                if attr_offset + nla_len > msg_end {
                    return None;
                }

                if nla_type == CTRL_ATTR_FAMILY_ID as u16 {
                    if nla_len < 6 {
                        return None;
                    }

                    let family_id =
                        u16::from_ne_bytes([response[attr_offset + 4], response[attr_offset + 5]]);

                    return Some(family_id);
                }

                attr_offset += self.nla_align(nla_len);
            }

            offset += self.nlmsg_align(nlmsg_len);
        }

        None
    }

    pub fn set_frequency(&mut self, freq: u32) -> bool {
        self.seq = self.seq.wrapping_add(1);

        let family_id = match self.nl80211_family_id {
            Some(id) => id,
            None => return false,
        };

        let pid = match &self.nladdr {
            Some(addr) => addr.nl_pid,
            None => 0,
        };

        let nl_hdr = gen_nl_hdr(
            36, // 16 + 4 + 8 + 8
            family_id,
            (NLM_F_REQUEST | NLM_F_ACK) as u16,
            self.seq,
            pid,
        );

        let genl_hdr = gen_genl_hdr(2);

        // IFINDEX attribute
        let ifindex_hdr = gen_nla_hdr(8, 3 as u16);
        let ifindex = self.if_index.to_ne_bytes();

        // WIPHY_FREQ attribute
        let freq_hdr = gen_nla_hdr(8, 38 as u16);
        let freq_n_bytes = (freq).to_ne_bytes();

        let mut iov = [
            iovec {
                iov_base: nl_hdr.as_ptr() as *mut c_void,
                iov_len: nl_hdr.len(),
            },
            iovec {
                iov_base: genl_hdr.as_ptr() as *mut c_void,
                iov_len: genl_hdr.len(),
            },
            iovec {
                iov_base: ifindex_hdr.as_ptr() as *mut c_void,
                iov_len: ifindex_hdr.len(),
            },
            iovec {
                iov_base: ifindex.as_ptr() as *mut c_void,
                iov_len: ifindex.len(),
            },
            iovec {
                iov_base: freq_hdr.as_ptr() as *mut c_void,
                iov_len: freq_hdr.len(),
            },
            iovec {
                iov_base: freq_n_bytes.as_ptr() as *mut c_void,
                iov_len: freq_n_bytes.len(),
            },
        ];

        let mut msg: msghdr = unsafe { std::mem::zeroed() };

        let mut kernel = sockaddr_nl {
            nl_family: AF_NETLINK as u16,
            nl_pad: 0,
            nl_pid: 0,
            nl_groups: 0,
        };

        msg.msg_name = &mut kernel as *mut _ as *mut c_void;
        msg.msg_namelen = std::mem::size_of::<sockaddr_nl>() as u32;

        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = iov.len();

        let ret = unsafe { sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if ret < 0 {
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                println!("sendmsg failed: {:?}", err);
                return false;
            }
            return false;
        }

        let mut buf = [0u8; 8192];
        let mut iov = iovec {
            iov_base: buf.as_mut_ptr() as *mut c_void,
            iov_len: buf.len(),
        };

        let mut nladdr: sockaddr_nl = unsafe { std::mem::zeroed() };

        let mut msg: msghdr = unsafe { std::mem::zeroed() };
        msg.msg_name = &mut nladdr as *mut _ as *mut c_void;
        msg.msg_namelen = std::mem::size_of::<sockaddr_nl>() as u32;
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;

        let len = unsafe { recvmsg(self.sock_fd.as_raw_fd(), &mut msg, 0) };
        if len < 0 {
            return false;
        }

        if len < std::mem::size_of::<libc::nlmsghdr>() as isize {
            return false;
        }

        for (i, b) in buf[..len as usize].iter().enumerate() {
            print!("{:02x} ", b);

            if (i + 1) % 16 == 0 {
                println!();
            }
        }

        let nl_hdr = unsafe { &*(buf.as_ptr() as *const libc::nlmsghdr) };

        match nl_hdr.nlmsg_type {
            x if x == libc::NLMSG_ERROR as u16 => {
                let err = unsafe {
                    &*(buf.as_ptr().add(std::mem::size_of::<libc::nlmsghdr>())
                        as *const libc::nlmsgerr)
                };

                if err.error == 0 {
                    return true;
                } else {
                    let errno = -err.error;
                    println!(
                        "SET_WIPHY failed: {} ({})",
                        errno,
                        std::io::Error::from_raw_os_error(errno)
                    );
                    return false;
                }
            }

            x if x == libc::NLMSG_DONE as u16 => {
                return true;
            }

            _ => {
                println!("Unexpected message type {}", nl_hdr.nlmsg_type);
                return false;
            }
        }
    }

    fn parse_bands(&self, data: &[u8]) -> u32 {
        let mut off = 0;
        let mut max_freq = 0u32;

        while off + 4 <= data.len() {
            let nla = unsafe { &*(data[off..].as_ptr() as *const libc::nlattr) };

            if nla.nla_len < 4 {
                break;
            }

            let start = off + 4;
            let end = off + nla.nla_len as usize;

            self.parse_band(&data[start..end], &mut max_freq);

            off += (nla.nla_len as usize + 3) & !3;
        }

        max_freq
    }

    fn parse_band(&self, data: &[u8], max_freq: &mut u32) {
        let mut off = 0;

        while off + 4 <= data.len() {
            let nla = unsafe { &*(data[off..].as_ptr() as *const libc::nlattr) };

            if nla.nla_len < 4 {
                break;
            }

            // NL80211_BAND_ATTR_FREQS
            if nla.nla_type == 1 {
                let start = off + 4;
                let end = off + nla.nla_len as usize;

                self.parse_freqs(&data[start..end], max_freq);
            }

            off += (nla.nla_len as usize + 3) & !3;
        }
    }

    fn parse_freqs(&self, data: &[u8], max_freq: &mut u32) {
        let mut off = 0;

        while off + 4 <= data.len() {
            let nla = unsafe { &*(data[off..].as_ptr() as *const libc::nlattr) };

            if nla.nla_len < 4 {
                break;
            }

            let start = off + 4;
            let end = off + nla.nla_len as usize;

            self.parse_freq(&data[start..end], max_freq);

            off += (nla.nla_len as usize + 3) & !3;
        }
    }

    fn parse_freq(&self, data: &[u8], max_freq: &mut u32) {
        let mut off = 0;

        while off + 4 <= data.len() {
            let nla = unsafe { &*(data[off..].as_ptr() as *const libc::nlattr) };

            if nla.nla_len < 4 {
                break;
            }

            let start = off + 4;
            let end = off + nla.nla_len as usize;

            // NL80211_FREQUENCY_ATTR_FREQ
            if nla.nla_type == 1 && end - start >= 4 {
                let freq = u32::from_ne_bytes([
                    data[start],
                    data[start + 1],
                    data[start + 2],
                    data[start + 3],
                ]);

                *max_freq = (*max_freq).max(freq);
            }

            off += (nla.nla_len as usize + 3) & !3;
        }
    }

    fn nlmsg_align(&self, len: usize) -> usize {
        (len + 3) & !3
    }

    fn nla_align(&self, len: usize) -> usize {
        (len + 3) & !3
    }

    pub fn interface_info(&mut self) -> Option<InterfaceInfo> {
        let c_interface = match CString::new(self.interface.clone()) {
            Ok(value) => value,
            Err(_) => return None,
        };
        let if_idx = air_if_name2idx(c_interface.as_ptr());

        if if_idx == 0 {
            return None;
        }

        let if_idx_data = if_idx.to_ne_bytes();

        let family_id = match self.nl80211_family_id {
            Some(id) => {
                println!("family id {}", id);
                id
            }
            None => return None,
        };

        self.seq = self.seq.wrapping_add(1);

        // GET_INTERFACE

        let nl_hdr = gen_nl_hdr(28, family_id, NLM_F_REQUEST as u16, self.seq, 0);

        let genl_hdr = gen_genl_hdr(5);

        let nla_hdr = gen_nla_hdr(8, 3);

        let mut iov = [
            iovec {
                iov_base: nl_hdr.as_ptr() as *mut c_void,
                iov_len: nl_hdr.len(),
            },
            iovec {
                iov_base: genl_hdr.as_ptr() as *mut c_void,
                iov_len: genl_hdr.len(),
            },
            iovec {
                iov_base: nla_hdr.as_ptr() as *mut c_void,
                iov_len: nla_hdr.len(),
            },
            iovec {
                iov_base: if_idx_data.as_ptr() as *mut c_void,
                iov_len: if_idx_data.len(),
            },
        ];

        let mut msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };
        // msg.msg_name = &mut self.nladdr as *mut _ as *mut c_void;
        // msg.msg_namelen = std::mem::size_of::<sockaddr_nl>() as u32;
        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = iov.len();

        let sent = unsafe { sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if sent < 0 {
            return None;
        }

        let mut response = [0u8; 4096];

        let mut recv_iov = iovec {
            iov_base: response.as_mut_ptr() as *mut c_void,
            iov_len: response.len(),
        };

        let mut recv_msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };

        recv_msg.msg_iov = &mut recv_iov;
        recv_msg.msg_iovlen = 1;

        let received = unsafe { recvmsg(self.sock_fd.as_raw_fd(), &mut recv_msg, 0) };

        if received <= 0 {
            return None;
        }

        let received = received as usize;

        let mut interface_details = InterfaceInfo::default();

        let mut offset = 0;

        while offset + 16 <= received {
            let nlmsg_len = u16::from_ne_bytes([response[offset], response[offset + 1]]) as usize;

            let nlmsg_type = u16::from_ne_bytes([response[offset + 2], response[offset + 3]]);

            if nlmsg_len < 16 || offset + nlmsg_len > received {
                return None;
            }

            // NLMSG_ERROR
            if nlmsg_type == 0x2 {
                if nlmsg_len < 20 {
                    return None;
                }

                let error = i32::from_ne_bytes([
                    response[offset + 16],
                    response[offset + 17],
                    response[offset + 18],
                    response[offset + 19],
                ]);

                if error != 0 {
                    return None;
                }

                break;
            }

            // NLMSG_DONE
            if nlmsg_type == 0x3 {
                break;
            }

            // nlmsghdr = 16
            // genlmsghdr = 4
            let mut attr_offset = offset + 20;

            let msg_end = offset + nlmsg_len;

            while attr_offset + 4 <= msg_end {
                let nla_len =
                    u16::from_ne_bytes([response[attr_offset], response[attr_offset + 1]]) as usize;

                let nla_type =
                    u16::from_ne_bytes([response[attr_offset + 2], response[attr_offset + 3]]);

                if nla_len < 4 || attr_offset + nla_len > msg_end {
                    return None;
                }

                let data_start = attr_offset + 4;
                let data_end = attr_offset + nla_len;

                let data = &response[data_start..data_end];

                match nla_type {
                    // NL80211_ATTR_WIPHY = 1
                    1 => {
                        if data.len() >= 4 {
                            interface_details.wiphy =
                                Some(u32::from_ne_bytes(data[0..4].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_WDEV = 2
                    2 => {
                        if data.len() >= 8 {
                            interface_details.wdev =
                                Some(u64::from_ne_bytes(data[0..8].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_IFINDEX = 3
                    3 => {
                        if data.len() >= 4 {
                            interface_details.ifindex =
                                Some(u32::from_ne_bytes(data[0..4].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_IFNAME = 4
                    4 => {
                        interface_details.ifname = Some(
                            String::from_utf8_lossy(data)
                                .trim_end_matches('\0')
                                .to_string(),
                        );
                    }

                    // NL80211_ATTR_IFTYPE = 5
                    5 => {
                        if data.len() >= 4 {
                            interface_details.iftype =
                                Some(u32::from_ne_bytes(data[0..4].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_MAC = 6
                    6 => {
                        if data.len() >= 6 {
                            interface_details.mac = Some(format!(
                                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                                data[0], data[1], data[2], data[3], data[4], data[5],
                            ));
                        }
                    }

                    // NL80211_ATTR_BSSID = 7
                    7 => {
                        if data.len() >= 6 {
                            interface_details.bssid =
                                Some([data[0], data[1], data[2], data[3], data[4], data[5]]);
                        }
                    }

                    // NL80211_ATTR_MTU = 29
                    29 => {
                        if data.len() >= 4 {
                            interface_details.mtu =
                                Some(u32::from_ne_bytes(data[0..4].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_WIPHY_FREQ = 38
                    38 => {
                        if data.len() >= 4 {
                            let freq = u32::from_ne_bytes(data[0..4].try_into().unwrap());

                            interface_details.frequency = Some(freq);

                            interface_details.channel = match freq {
                                // 2.4 GHz
                                2412..=2472 => Some((freq - 2407) / 5),
                                2484 => Some(14),

                                // 5 GHz
                                5000..=5900 => {
                                    if (freq - 5000) % 5 == 0 {
                                        Some((freq - 5000) / 5)
                                    } else {
                                        None
                                    }
                                }

                                // 6 GHz
                                5955..=7115 => {
                                    if (freq - 5950) % 5 == 0 {
                                        Some((freq - 5950) / 5)
                                    } else {
                                        None
                                    }
                                }

                                _ => None,
                            };
                        }
                    }

                    39 => {
                        if data.len() >= 4 {
                            interface_details.generation =
                                Some(u32::from_ne_bytes(data[..4].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_GENERATION = 46
                    46 => {
                        if data.len() >= 4 {
                            interface_details.generation =
                                Some(u32::from_ne_bytes(data[0..4].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_SSID = 52
                    52 => {
                        interface_details.ssid = Some(String::from_utf8_lossy(data).to_string());
                    }

                    // NL80211_ATTR_4ADDR = 83
                    83 => {
                        interface_details.four_addr = true;
                    }

                    98 => {
                        if data.len() >= 4 {
                            interface_details.max_scan_ssids =
                                Some(u32::from_ne_bytes(data[..4].try_into().unwrap()));
                        }
                    }

                    // NL80211_ATTR_CONNECTED = 150
                    150 => {
                        interface_details.connected = true;
                    }

                    // NL80211_ATTR_STA_INFO = 153
                    153 => {
                        let mut sta_offset = 0;

                        while sta_offset + 4 <= data.len() {
                            let sta_len =
                                u16::from_ne_bytes([data[sta_offset], data[sta_offset + 1]])
                                    as usize;

                            let sta_type =
                                u16::from_ne_bytes([data[sta_offset + 2], data[sta_offset + 3]])
                                    & 0x3fff;

                            if sta_len < 4 || sta_offset + sta_len > data.len() {
                                break;
                            }

                            let sta_data = &data[sta_offset + 4..sta_offset + sta_len];

                            match sta_type {
                                // NL80211_STA_INFO_SIGNAL = 7
                                7 => {
                                    if !sta_data.is_empty() {
                                        interface_details.signal_dbm = Some(sta_data[0] as i32);
                                    }
                                }

                                // NL80211_STA_INFO_SIGNAL_AVG = 13
                                13 => {
                                    if !sta_data.is_empty() {
                                        interface_details.signal_avg_dbm = Some(sta_data[0] as i32);
                                    }
                                }

                                _ => {}
                            }

                            sta_offset += (sta_len + 3) & !3;
                        }
                    }

                    159 => {
                        interface_details.four_addr = data.first().copied().unwrap_or(0) != 0;
                    }

                    // NL80211_ATTR_CHANNEL_WIDTH = 160
                    160 => {
                        if data.len() >= 4 {
                            interface_details.channel_width =
                                Some(u32::from_ne_bytes(data[0..4].try_into().unwrap()));
                        }
                    }

                    290 => {
                        interface_details.mlo_support = data.first().copied().unwrap_or(0) != 0;
                    }

                    // NL80211_ATTR_VIF_RADIO_MASK = 333
                    333 => {
                        if data.len() >= 4 {
                            interface_details.vif_radio_mask =
                                Some(u32::from_ne_bytes(data[0..4].try_into().unwrap()));
                        }
                    }
                    _ => {}
                }
                // NLA_ALIGN
                attr_offset += (nla_len + 3) & !3;
            }

            // NLMSG_ALIGN
            offset += (nlmsg_len + 3) & !3;
        }

        // GET_WIPHY

        let wiphy = match interface_details.wiphy {
            Some(wpy) => wpy,
            None => return None,
        };
        self.seq = self.seq.wrapping_add(1);

        let wiphy_data = wiphy.to_ne_bytes();

        let nl_hdr = gen_nl_hdr(28, family_id, NLM_F_REQUEST as u16, self.seq, 0);

        let genl_hdr = gen_genl_hdr(1); // NL80211_CMD_GET_WIPHY

        let nla_hdr = gen_nla_hdr(8, 1); // NL80211_ATTR_WIPHY

        let mut iov = [
            iovec {
                iov_base: nl_hdr.as_ptr() as *mut c_void,
                iov_len: nl_hdr.len(),
            },
            iovec {
                iov_base: genl_hdr.as_ptr() as *mut c_void,
                iov_len: genl_hdr.len(),
            },
            iovec {
                iov_base: nla_hdr.as_ptr() as *mut c_void,
                iov_len: nla_hdr.len(),
            },
            iovec {
                iov_base: wiphy_data.as_ptr() as *mut c_void,
                iov_len: wiphy_data.len(),
            },
        ];

        let mut msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };
        // msg.msg_name = &mut self.nladdr as *mut _ as *mut c_void;
        // msg.msg_namelen = std::mem::size_of::<sockaddr_nl>() as u32;
        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = iov.len();

        let sent = unsafe { sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if sent < 0 {
            return None;
        }

        let mut response = vec![0u8; 65536];

        let mut recv_iov = iovec {
            iov_base: response.as_mut_ptr() as *mut c_void,
            iov_len: response.len(),
        };

        let mut recv_msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };
        recv_msg.msg_iov = &mut recv_iov;
        recv_msg.msg_iovlen = 1;

        let received = unsafe { recvmsg(self.sock_fd.as_raw_fd(), &mut recv_msg, 0) };

        if received < 0 {
            return None;
        }

        let received = received as usize;
        response.truncate(received);

        let mut offset = 0;

        while offset + std::mem::size_of::<nlmsghdr>() <= response.len() {
            let hdr = unsafe { &*(response[offset..].as_ptr() as *const nlmsghdr) };

            if hdr.nlmsg_len < std::mem::size_of::<nlmsghdr>() as u32 {
                break;
            }

            if hdr.nlmsg_type == NLMSG_DONE as u16 {
                // DONE
                break;
            }

            if hdr.nlmsg_type == NLMSG_ERROR as u16 {
                // ERROR
                break;
            }

            let attr_start = offset + std::mem::size_of::<nlmsghdr>() + 4; // genlmsghdr

            let attr_end = offset + hdr.nlmsg_len as usize;

            let mut attr_off = attr_start;

            while attr_off + std::mem::size_of::<nlattr>() <= attr_end {
                let nla = unsafe { &*(response[attr_off..].as_ptr() as *const nlattr) };

                if nla.nla_len < 4 {
                    break;
                }

                let data_start = attr_off + 4;
                let data_end = attr_off + nla.nla_len as usize;

                let data = &response[data_start..data_end];

                match nla.nla_type {
                    22 => {
                        interface_details.max_frequency = Some(self.parse_bands(data));
                    }
                    _ => {}
                }

                // match nla.nla_type {
                //     // NL80211_ATTR_WIPHY
                //     1 => {
                //         if data.len() >= 4 {
                //             let wiphy = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);

                //             println!("WIPHY = {}", wiphy);
                //         }
                //     }

                //     // NL80211_ATTR_WIPHY_NAME
                //     2 => {
                //         println!(
                //             "Name = {}",
                //             String::from_utf8_lossy(data).trim_end_matches('\0')
                //         );
                //     }

                //     _ => {}
                // }

                attr_off += (nla.nla_len as usize + 3) & !3;
            }

            offset += (hdr.nlmsg_len as usize + 3) & !3;
        }

        Some(interface_details)
    }

    pub fn start_monitor(&mut self) -> bool {
        let family_id = match self.nl80211_family_id {
            Some(id) => id,
            None => return false,
        };

        self.seq = self.seq.wrapping_add(1);

        let nl_hdr = gen_nl_hdr(
            36,
            family_id,
            NLM_F_REQUEST as u16 | NLM_F_ACK as u16,
            self.seq,
            0,
        );

        let genl_hdr = gen_genl_hdr(6);

        let nla_ifindex = gen_nla_hdr(8, 3);
        let ifindex = self.if_index.to_ne_bytes();

        let nla_iftype = gen_nla_hdr(8, 5);
        let iftype = (6u32).to_ne_bytes();

        let mut iov = [
            iovec {
                iov_base: nl_hdr.as_ptr() as *mut c_void,
                iov_len: nl_hdr.len(),
            },
            iovec {
                iov_base: genl_hdr.as_ptr() as *mut c_void,
                iov_len: genl_hdr.len(),
            },
            iovec {
                iov_base: nla_ifindex.as_ptr() as *mut c_void,
                iov_len: nla_ifindex.len(),
            },
            iovec {
                iov_base: ifindex.as_ptr() as *mut c_void,
                iov_len: ifindex.len(),
            },
            iovec {
                iov_base: nla_iftype.as_ptr() as *mut c_void,
                iov_len: nla_iftype.len(),
            },
            iovec {
                iov_base: iftype.as_ptr() as *mut c_void,
                iov_len: iftype.len(),
            },
        ];

        let mut msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };
        // msg.msg_name = &mut self.nladdr as *mut _ as *mut c_void;
        // msg.msg_namelen = std::mem::size_of::<sockaddr_nl>() as u32;
        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = iov.len();

        let sent = unsafe { sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if sent < 0 {
            return false;
        }

        let mut buf = [0u8; 4096];

        let mut recv_iov = [iovec {
            iov_base: buf.as_mut_ptr() as *mut c_void,
            iov_len: buf.len(),
        }];

        let mut recv_msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };
        recv_msg.msg_iov = recv_iov.as_mut_ptr();
        recv_msg.msg_iovlen = 1;

        let received = unsafe { recvmsg(self.sock_fd.as_raw_fd(), &mut recv_msg, 0) };

        if received < 0 {
            return false;
        }

        // Parse netlink header
        let nl_hdr = unsafe { &*(buf.as_ptr() as *const nlmsghdr) };

        if nl_hdr.nlmsg_type == libc::NLMSG_ERROR as u16 {
            let err =
                unsafe { &*(buf.as_ptr().add(std::mem::size_of::<nlmsghdr>()) as *const nlmsgerr) };

            return err.error == 0;
        }

        false
    }

    pub fn stop_monitor(&mut self) -> bool {
        let family_id = match self.nl80211_family_id {
            Some(id) => id,
            None => return false,
        };

        self.seq = self.seq.wrapping_add(1);

        let nl_hdr = gen_nl_hdr(
            36,
            family_id,
            NLM_F_REQUEST as u16 | NLM_F_ACK as u16,
            self.seq,
            0,
        );

        let genl_hdr = gen_genl_hdr(6);

        let nla_ifindex = gen_nla_hdr(8, 3);
        let ifindex = self.if_index.to_ne_bytes();

        let nla_iftype = gen_nla_hdr(8, 5);
        let iftype = (2u32).to_ne_bytes();

        let mut iov = [
            iovec {
                iov_base: nl_hdr.as_ptr() as *mut c_void,
                iov_len: nl_hdr.len(),
            },
            iovec {
                iov_base: genl_hdr.as_ptr() as *mut c_void,
                iov_len: genl_hdr.len(),
            },
            iovec {
                iov_base: nla_ifindex.as_ptr() as *mut c_void,
                iov_len: nla_ifindex.len(),
            },
            iovec {
                iov_base: ifindex.as_ptr() as *mut c_void,
                iov_len: ifindex.len(),
            },
            iovec {
                iov_base: nla_iftype.as_ptr() as *mut c_void,
                iov_len: nla_iftype.len(),
            },
            iovec {
                iov_base: iftype.as_ptr() as *mut c_void,
                iov_len: iftype.len(),
            },
        ];

        let mut msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };
        // msg.msg_name = &mut self.nladdr as *mut _ as *mut c_void;
        // msg.msg_namelen = std::mem::size_of::<sockaddr_nl>() as u32;
        msg.msg_iov = iov.as_mut_ptr();
        msg.msg_iovlen = iov.len();

        let sent = unsafe { sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if sent < 0 {
            return false;
        }

        let mut buf = [0u8; 4096];

        let mut recv_iov = [iovec {
            iov_base: buf.as_mut_ptr() as *mut c_void,
            iov_len: buf.len(),
        }];

        let mut recv_msg: msghdr = unsafe { MaybeUninit::zeroed().assume_init() };
        recv_msg.msg_iov = recv_iov.as_mut_ptr();
        recv_msg.msg_iovlen = 1;

        let received = unsafe { recvmsg(self.sock_fd.as_raw_fd(), &mut recv_msg, 0) };

        if received < 0 {
            return false;
        }

        // Parse netlink header
        let nl_hdr = unsafe { &*(buf.as_ptr() as *const nlmsghdr) };

        if nl_hdr.nlmsg_type == libc::NLMSG_ERROR as u16 {
            let err =
                unsafe { &*(buf.as_ptr().add(std::mem::size_of::<nlmsghdr>()) as *const nlmsgerr) };

            return err.error == 0;
        }

        false
    }
}
