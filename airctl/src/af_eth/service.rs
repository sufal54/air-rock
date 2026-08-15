use std::{
    ffi::{CString, c_void},
    mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use air_apis::{
    AF_PACKET, ETH_P_ALL, SOCK_RAW, air_bind, air_connect, air_if_name2idx, air_recv, sockaddr,
    sockaddr_ll,
};
use libc::{POLLIN, poll, pollfd};

pub struct Service {
    sock_fd: OwnedFd,
    if_index: u32,
}

impl Service {
    pub fn new(interface: String) -> Self {
        let c_interface = match CString::new(interface.clone()) {
            Ok(string) => string,
            Err(_) => panic!(),
        };

        let sock_fd = air_connect(AF_PACKET, SOCK_RAW, (ETH_P_ALL as u16).to_be() as i32);

        let if_index = air_if_name2idx(c_interface.as_ptr());
        if sock_fd < 0 {
            panic!("Failed to create ETH All socket");
        }
        Self {
            sock_fd: unsafe { OwnedFd::from_raw_fd(sock_fd) },
            if_index,
        }
    }

    pub fn sock_bind(&self) {
        let mut addr = unsafe { mem::zeroed::<sockaddr_ll>() };
        addr.sll_family = AF_PACKET as u16;
        addr.sll_protocol = (ETH_P_ALL as u16).to_be();
        addr.sll_ifindex = self.if_index as i32;

        let ret = air_bind(
            self.sock_fd.as_raw_fd(),
            &addr as *const _ as *const sockaddr,
            mem::size_of::<sockaddr_ll>() as u32,
        );

        if ret < 0 {
            panic!("bind failed");
        }
    }

    pub fn accept(&self) {
        let mut buffer = [0u8; 65536];

        let len = air_recv(
            self.sock_fd.as_raw_fd(),
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len(),
            0,
        );

        if len > 0 {
            self.parse_packet(&buffer[0..len as usize]);
        }
    }

    pub fn accept_timeout(&self, timeout_ms: i32) {
        let mut pfd = pollfd {
            fd: self.sock_fd.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

        let ready = unsafe { poll(&mut pfd, 1, timeout_ms) };

        if ready > 0 && (pfd.revents & POLLIN) != 0 {
            self.accept();
        }
    }

    pub fn parse_packet(&self, packet: &[u8]) {
        // -----------------------------
        // Radiotap
        // -----------------------------
        if packet.len() < 8 {
            return;
        }

        let radiotap_len = u16::from_le_bytes([packet[2], packet[3]]) as usize;

        if packet.len() < radiotap_len + 24 {
            return;
        }

        println!("========================================");
        println!("Radiotap Length: {}", radiotap_len);

        let wifi = &packet[radiotap_len..];

        // -----------------------------------
        // MAC Header
        // -----------------------------------

        let frame_control = u16::from_le_bytes([wifi[0], wifi[1]]);

        let version = (frame_control & 0x3) as u8;
        let frame_type = ((frame_control >> 2) & 0x3) as u8;
        let subtype = ((frame_control >> 4) & 0xF) as u8;

        let to_ds = ((frame_control >> 8) & 1) != 0;
        let from_ds = ((frame_control >> 9) & 1) != 0;
        let more_frag = ((frame_control >> 10) & 1) != 0;
        let retry = ((frame_control >> 11) & 1) != 0;
        let power = ((frame_control >> 12) & 1) != 0;
        let more_data = ((frame_control >> 13) & 1) != 0;
        let protected = ((frame_control >> 14) & 1) != 0;
        let order = ((frame_control >> 15) & 1) != 0;

        let duration = u16::from_le_bytes([wifi[2], wifi[3]]);

        let mut addr1 = [0u8; 6];
        let mut addr2 = [0u8; 6];
        let mut addr3 = [0u8; 6];

        addr1.copy_from_slice(&wifi[4..10]);
        addr2.copy_from_slice(&wifi[10..16]);
        addr3.copy_from_slice(&wifi[16..22]);

        let seq_ctrl = u16::from_le_bytes([wifi[22], wifi[23]]);

        let sequence = seq_ctrl >> 4;
        let fragment = seq_ctrl & 0xF;

        println!("Version     : {}", version);
        println!("Type        : {}", frame_type);
        println!("Subtype     : {}", subtype);
        println!("Protected   : {}", protected);
        println!("To DS       : {}", to_ds);
        println!("From DS     : {}", from_ds);
        println!("Retry       : {}", retry);
        println!("More Frag   : {}", more_frag);
        println!("More Data   : {}", more_data);
        println!("Power Mgmt  : {}", power);
        println!("Order       : {}", order);

        println!("Duration    : {}", duration);

        println!("Receiver    : {}", self.mac(&addr1));
        println!("Transmitter : {}", self.mac(&addr2));
        println!("BSSID       : {}", self.mac(&addr3));

        println!("Sequence    : {}", sequence);
        println!("Fragment    : {}", fragment);

        match (frame_type, subtype) {
            (0, 0) => println!("Frame: Association Request"),
            (0, 1) => println!("Frame: Association Response"),
            (0, 2) => println!("Frame: Reassociation Request"),
            (0, 3) => println!("Frame: Reassociation Response"),
            (0, 4) => println!("Frame: Probe Request"),
            (0, 5) => println!("Frame: Probe Response"),
            (0, 8) => println!("Frame: Beacon"),
            (0, 9) => println!("Frame: ATIM"),
            (0, 10) => println!("Frame: Disassociation"),
            (0, 11) => println!("Frame: Authentication"),
            (0, 12) => println!("Frame: Deauthentication"),
            (0, 13) => println!("Frame: Action"),
            (1, _) => println!("Frame: Control"),
            (2, _) => println!("Frame: Data"),
            _ => println!("Frame: Unknown"),
        }

        // --------------------------------------------------------
        // Parse Beacon / Probe Response body
        // --------------------------------------------------------

        if (frame_type == 0 && subtype == 8) || (frame_type == 0 && subtype == 5) {
            if wifi.len() < 36 {
                return;
            }

            let mut pos = 24;

            let timestamp = u64::from_le_bytes(wifi[pos..pos + 8].try_into().unwrap());
            pos += 8;

            let interval = u16::from_le_bytes(wifi[pos..pos + 2].try_into().unwrap());
            pos += 2;

            let capability = u16::from_le_bytes(wifi[pos..pos + 2].try_into().unwrap());
            pos += 2;

            println!("Timestamp   : {}", timestamp);
            println!("Interval    : {} TU", interval);
            println!("Capability  : 0x{:04X}", capability);

            println!("----------- Information Elements -----------");

            while pos + 2 <= wifi.len() {
                let id = wifi[pos];
                let len = wifi[pos + 1] as usize;
                pos += 2;

                if pos + len > wifi.len() {
                    break;
                }

                let data = &wifi[pos..pos + len];

                match id {
                    // SSID
                    0 => {
                        let ssid = if len == 0 {
                            "<Hidden>".to_string()
                        } else {
                            String::from_utf8_lossy(data).to_string()
                        };

                        println!("SSID             : {}", ssid);
                    }

                    // Supported rates
                    1 => {
                        print!("Supported Rates  : ");

                        for rate in data {
                            let r = (*rate & 0x7F) as f32 * 0.5;
                            print!("{}Mbps ", r);
                        }

                        println!();
                    }

                    // Channel
                    3 => {
                        if len >= 1 {
                            println!("Channel          : {}", data[0]);
                        }
                    }

                    // TIM
                    5 => {
                        println!("TIM");
                    }

                    // Country
                    7 => {
                        if len >= 3 {
                            println!(
                                "Country          : {}",
                                String::from_utf8_lossy(&data[0..3])
                            );
                        }
                    }

                    // ERP
                    42 => println!("ERP Information"),

                    // RSN (WPA2/WPA3)
                    48 => println!("Security         : RSN (WPA2/WPA3)"),

                    // HT
                    45 => println!("HT Capabilities"),

                    61 => println!("HT Operation"),

                    // VHT
                    191 => println!("VHT Capabilities"),

                    192 => println!("VHT Operation"),

                    // HE (WiFi6)
                    255 => println!("Extension Element"),

                    // Vendor
                    221 => {
                        if len >= 3 {
                            println!(
                                "Vendor OUI       : {:02X}:{:02X}:{:02X}",
                                data[0], data[1], data[2]
                            );
                        }
                    }

                    _ => {}
                }

                pos += len;
            }
        }

        println!();
    }

    fn mac(&self, addr: &[u8; 6]) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            addr[0], addr[1], addr[2], addr[3], addr[4], addr[5],
        )
    }
}
