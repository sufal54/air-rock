use air_apis::{genlmsghdr, nlmsghdr};
use libc::nlattr;

pub fn gen_nl_hdr(len: u32, msg_type: u16, flags: u16, seq: u32, pid: u32) -> [u8; 16] {
    let mut buf = [0u8; 16];

    let nl_msg_hdr = unsafe { &mut *(buf.as_mut_ptr() as *mut nlmsghdr) };

    nl_msg_hdr.nlmsg_len = len;
    nl_msg_hdr.nlmsg_type = msg_type;
    nl_msg_hdr.nlmsg_flags = flags;
    nl_msg_hdr.nlmsg_seq = seq;
    nl_msg_hdr.nlmsg_pid = pid;

    buf
}

pub fn gen_genl_hdr(cmd: u8) -> [u8; 4] {
    let mut buf = [0u8; 4];

    let gen_msg_hdr = unsafe { &mut *(buf.as_mut_ptr() as *mut genlmsghdr) };

    gen_msg_hdr.cmd = cmd;
    gen_msg_hdr.version = 1;
    gen_msg_hdr.reserved = 0;

    return buf;
}

pub fn gen_nla_hdr(len: u16, attr_type: u16) -> [u8; 4] {
    let mut buf = [0u8; 4];

    let attr_hdr = unsafe { &mut *(buf.as_mut_ptr() as *mut nlattr) };

    attr_hdr.nla_len = len;
    attr_hdr.nla_type = attr_type;

    buf
}

pub fn freq_to_channel(freq: u32) -> Option<u16> {
    match freq {
        2412..=2472 => Some(((freq - 2407) / 5) as u16),
        2484 => Some(14),
        5000..=5895 => Some(((freq - 5000) / 5) as u16),
        5955..=7115 => Some(((freq - 5950) / 5) as u16),
        _ => None,
    }
}

pub fn channel_to_freq(channel: u16) -> Option<u32> {
    match channel {
        1..=13 => Some(2407 + (channel as u32 * 5)),
        14 => Some(2484),
        32..=177 => Some(5000 + (channel as u32 * 5)),
        1..=233 => Some(5950 + (channel as u32 * 5)),
        _ => None,
    }
}

pub const CHANNEL_FREQ_MAP: &[(u16, u32)] = &[
    // 2.4 GHz
    (1, 2412),
    (2, 2417),
    (3, 2422),
    (4, 2427),
    (5, 2432),
    (6, 2437),
    (7, 2442),
    (8, 2447),
    (9, 2452),
    (10, 2457),
    (11, 2462),
    (12, 2467),
    (13, 2472),
    (14, 2484),
    // 5 GHz
    (32, 5160),
    (34, 5170),
    (36, 5180),
    (38, 5190),
    (40, 5200),
    (42, 5210),
    (44, 5220),
    (46, 5230),
    (48, 5240),
    (52, 5260),
    (56, 5280),
    (60, 5300),
    (64, 5320),
    (68, 5340),
    (96, 5480),
    (100, 5500),
    (104, 5520),
    (108, 5540),
    (112, 5560),
    (116, 5580),
    (120, 5600),
    (124, 5620),
    (128, 5640),
    (132, 5660),
    (136, 5680),
    (140, 5700),
    (144, 5720),
    (149, 5745),
    (153, 5765),
    (157, 5785),
    (161, 5805),
    (165, 5825),
    (169, 5845),
    (173, 5865),
    (177, 5885),
];
