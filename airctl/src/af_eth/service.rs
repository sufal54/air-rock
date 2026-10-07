use std::{
    ffi::{CString, c_void},
    io::IoSlice,
    mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use air_apis::{
    AF_PACKET, ETH_P_ALL, POLLIN, SOCK_RAW, air_bind, air_connect, air_if_name2idx, air_poll,
    air_recv, pollfd, sockaddr, sockaddr_ll,
};

use crate::af_eth::{ControlFrame, FrameName, PacketInfo, RadioInfo, RadioTab, Security};

pub struct Service {
    sock_fd: OwnedFd,
    poll_fd: pollfd,
    if_index: u32,
}

impl Service {
    pub fn new(interface: String) -> Self {
        let c_interface = CString::new(interface).expect("Invalid interface name");

        let sock_fd = air_connect(AF_PACKET, SOCK_RAW, (ETH_P_ALL as u16).to_be() as i32);

        if sock_fd < 0 {
            panic!("Failed to create ETH_ALL socket");
        }

        let if_index = air_if_name2idx(c_interface.as_ptr());

        let sock_fd = unsafe { OwnedFd::from_raw_fd(sock_fd) };

        Self {
            poll_fd: pollfd {
                fd: sock_fd.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
            sock_fd,
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

    pub fn accept(&self) -> Option<(RadioInfo, PacketInfo, Vec<u8>)> {
        let mut buffer = [0u8; 65536];

        let len = air_recv(
            self.sock_fd.as_raw_fd(),
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len(),
            0,
        );
        if len <= 0 {
            return None;
        }
        let packet = &buffer[..len as usize];
        let (radio_info, packet_info) = self.parse_packet(packet)?;

        Some((radio_info, packet_info, packet.to_vec()))
    }

    pub fn accept_timeout(&mut self, timeout_ms: i32) -> Option<(RadioInfo, PacketInfo, Vec<u8>)> {
        // let mut pfd = pollfd {
        //     fd: self.sock_fd.as_raw_fd(),
        //     events: POLLIN,
        //     revents: 0,
        // };

        self.poll_fd.revents = 0;

        let ready = air_poll(&mut self.poll_fd, 1, timeout_ms);

        if ready > 0 && (self.poll_fd.revents & POLLIN) != 0 {
            return self.accept();
        }
        return None;
    }

    pub fn drain(&mut self) {
        let mut buffer = [0u8; 65536];

        loop {
            // let mut pfd = pollfd {
            //     fd: self.sock_fd.as_raw_fd(),
            //     events: POLLIN,
            //     revents: 0,
            // };

            let ready = air_poll(&mut self.poll_fd, 1, 0);

            if ready <= 0 || (self.poll_fd.revents & POLLIN) == 0 {
                break;
            }

            let len = air_recv(
                self.sock_fd.as_raw_fd(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                0,
            );

            if len <= 0 {
                break;
            }
        }
    }

    pub fn parse_packet(&self, packet: &[u8]) -> Option<(RadioInfo, PacketInfo)> {
        let mut packet_info: PacketInfo = PacketInfo::default();

        // Radiotap header

        if packet.len() < 8 {
            return None;
        }
        let (radio_info, radiotap_len) = self.parse_radiotap(packet).unwrap();
        // let version = packet[0];

        // if version != 0 {
        //     return None;
        // }

        // let radiotap_len = u16::from_le_bytes([packet[2], packet[3]]) as usize;

        if radiotap_len < 8 || packet.len() < radiotap_len {
            return None;
        }

        // let radiotap = &packet[..radiotap_len];
        let wifi = &packet[radiotap_len..];

        // Parse Radiotap

        let capture_freq = radio_info.frequency;

        // We need at least Frame Control.
        if wifi.len() < 2 {
            return None;
        }

        // Frame Control

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

        // Frame name

        let frame_name = match (frame_type, subtype) {
            (0, 0) => FrameName::AssociationRequest,
            (0, 1) => FrameName::AssociationResponse,
            (0, 2) => FrameName::ReassociationRequest,
            (0, 3) => FrameName::ReassociationResponse,
            (0, 4) => FrameName::ProbeRequest,
            (0, 5) => FrameName::ProbeResponse,
            (0, 8) => FrameName::Beacon,
            (0, 9) => FrameName::Atim,
            (0, 10) => FrameName::Disassociation,
            (0, 11) => FrameName::Authentication,
            (0, 12) => FrameName::Deauthentication,
            (0, 13) => FrameName::Action,

            (1, 8) => FrameName::BlockAckRequest,
            (1, 9) => FrameName::BlockAck,
            (1, 10) => FrameName::PsPoll,
            (1, 11) => FrameName::Rts,
            (1, 12) => FrameName::Cts,
            (1, 13) => FrameName::Ack,
            (1, 14) => FrameName::CfEnd,
            (1, 15) => FrameName::CfEndCfAck,

            (2, _) => FrameName::Data,

            _ => FrameName::Unknown,
        };

        // Output

        packet_info.radiotap_length = radiotap_len;

        match capture_freq {
            Some(freq) => {
                packet_info.capture_frequency = Some(freq);
            }
            None => {
                println!("Capture Frequency: Unknown");
            }
        }

        packet_info.version = version;
        packet_info.frame_type = frame_type;
        packet_info.subtype = subtype;

        packet_info.protected = protected;
        packet_info.to_ds = to_ds;
        packet_info.from_ds = from_ds;
        packet_info.retry = retry;
        packet_info.more_frag = more_frag;
        packet_info.more_data = more_data;
        packet_info.power_mgmt = power;
        packet_info.order = order;

        packet_info.frame_name = frame_name;

        // Control frames
        //
        // Control frames DO NOT necessarily have 24-byte headers.

        if frame_type == 1 {
            self.parse_control_frame(wifi, subtype);
            return None;
        }

        // Management/Data frames need at least 24 bytes

        if wifi.len() < 24 {
            return None;
        }

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

        packet_info.duration = Some(duration);

        packet_info.receiver = self.mac(&addr1);
        packet_info.transmitter = self.mac(&addr2);
        packet_info.bssid = self.mac(&addr3);

        packet_info.sequence = Some(sequence);
        packet_info.fragment = Some(fragment);

        // Beacon / Probe Response

        if frame_type == 0 && (subtype == 8 || subtype == 5) {
            self.parse_beacon(wifi, &mut packet_info);
        }

        if frame_type == 2 {
            if let Some(eapol_frame) = self.parse_eapol(wifi) {
                packet_info.frame_name = eapol_frame;
            }
        }

        packet_info.security = if let Some(rsn) = &packet_info.rsn {
            self.parse_rsn_security(rsn)
        } else {
            Security::Open
        };
        Some((radio_info, packet_info))
    }

    fn parse_radiotap(&self, packet: &[u8]) -> Option<(RadioInfo, usize)> {
        if packet.len() < 8 {
            return None;
        }

        // Radiotap header

        let version = packet[0];

        if version != 0 {
            return None;
        }

        let length = u16::from_le_bytes([packet[2], packet[3]]);

        let length_usize = length as usize;

        if length_usize < 8 || packet.len() < length_usize {
            return None;
        }

        // Present bitmaps

        let mut present_offset = 4;
        let mut words = Vec::new();

        loop {
            if present_offset + 4 > length_usize {
                return None;
            }

            let word = u32::from_le_bytes([
                packet[present_offset],
                packet[present_offset + 1],
                packet[present_offset + 2],
                packet[present_offset + 3],
            ]);

            words.push(word);

            present_offset += 4;

            // EXT bit
            if (word & (1 << 31)) == 0 {
                break;
            }
        }

        let mut radio = RadioInfo {
            length,
            ..Default::default()
        };

        // Fields

        let mut offset = present_offset;

        for (word_index, word) in words.iter().enumerate() {
            for bit in 0..32 {
                // Bit 31 is the EXT flag, not a field.
                if bit == 31 {
                    continue;
                }

                if (word & (1 << bit)) == 0 {
                    continue;
                }

                let field = word_index * 32 + bit;

                let (align, size) = match field {
                    // TSFT
                    0 => (8, 8),

                    // FLAGS
                    1 => (1, 1),

                    // RATE
                    2 => (1, 1),

                    // CHANNEL
                    3 => (2, 4),

                    // FHSS
                    4 => (2, 2),

                    // DBM Antenna Signal
                    5 => (1, 1),

                    // DBM Antenna Noise
                    6 => (1, 1),

                    // Lock Quality
                    7 => (2, 2),

                    // TX Attenuation
                    8 => (2, 2),

                    // dB TX Attenuation
                    9 => (2, 2),

                    // dBm TX Power
                    10 => (1, 1),

                    // Antenna
                    11 => (1, 1),

                    // dB Antenna Signal
                    12 => (1, 1),

                    // dB Antenna Noise
                    13 => (1, 1),

                    // RX Flags
                    14 => (2, 2),

                    // TX Flags
                    15 => (4, 4),

                    // MCS
                    19 => (1, 3),

                    // VHT
                    21 => (2, 12),

                    // HE
                    23 => (2, 12),

                    // HE-MU
                    24 => (2, 12),

                    // Unknown field
                    _ => {
                        // We don't know its size, so we cannot
                        // safely continue parsing.
                        return Some((radio, length_usize));
                    }
                };

                // Alignment

                let remainder = offset % align;

                if remainder != 0 {
                    offset += align - remainder;
                }

                if offset + size > length_usize {
                    return None;
                }

                let data = &packet[offset..offset + size];

                // Extract field

                match field {
                    // TSFT
                    0 => {
                        radio.tsft = Some(u64::from_le_bytes(data.try_into().unwrap()));
                    }

                    // Flags
                    1 => {
                        radio.flags = Some(data[0]);
                    }

                    // Rate
                    //
                    // Radiotap stores legacy rate in 500 Kbps units.
                    // 54 Mbps => 108 => 54.0 Mbps
                    2 => {
                        radio.rate = Some(data[0] as f32 * 0.5);
                    }

                    // Channel
                    3 => {
                        radio.frequency = Some(u16::from_le_bytes([data[0], data[1]]));

                        radio.channel_flags = Some(u16::from_le_bytes([data[2], data[3]]));
                    }

                    // FHSS
                    4 => {
                        // Currently not stored.
                    }

                    // dBm Antenna Signal
                    5 => {
                        radio.signal_power = Some(data[0] as i8);
                    }

                    // dBm Antenna Noise
                    6 => {
                        radio.noise_power = Some(data[0] as i8);
                    }

                    // Lock Quality
                    7 => {
                        radio.lock_quality = Some(u16::from_le_bytes([data[0], data[1]]));
                    }

                    // TX attenuation
                    8 => {
                        radio.tx_attenuation = Some(u16::from_le_bytes([data[0], data[1]]));
                    }

                    // dB TX attenuation
                    9 => {
                        radio.tx_attenuation_db = Some(u16::from_le_bytes([data[0], data[1]]));
                    }

                    // dBm TX power
                    10 => {
                        radio.tx_power = Some(data[0] as i8);
                    }

                    // Antenna
                    11 => {
                        radio.antenna = Some(data[0]);
                    }

                    // dB Antenna Signal
                    12 => {
                        radio.signal_quality = Some(data[0]);
                    }

                    // dB Antenna Noise
                    13 => {
                        radio.noise_quality = Some(data[0]);
                    }

                    // RX Flags
                    14 => {
                        radio.rx_flags = Some(u16::from_le_bytes([data[0], data[1]]));
                    }

                    // TX Flags
                    15 => {
                        radio.tx_flags = Some(u16::from_le_bytes([data[0], data[1]]));
                    }

                    // MCS
                    19 => {
                        radio.mcs = Some([data[0], data[1], data[2]]);
                    }

                    // VHT
                    21 => {
                        radio.vht = Some(data.try_into().unwrap());
                    }

                    // HE
                    23 => {
                        radio.he = Some(data.try_into().unwrap());
                    }

                    // HE-MU
                    24 => {
                        // You can add a field for this later.
                    }

                    _ => {}
                }

                offset += size;
            }
        }

        Some((radio, length_usize))
    }
    // Control frame parser

    fn parse_control_frame(&self, wifi: &[u8], subtype: u8) {
        // Most control frames contain:
        //
        // Frame Control : 2
        // Duration      : 2
        // Receiver      : 6
        //
        // Some subtypes additionally contain Transmitter Address.
        //
        if wifi.len() < 10 {
            return;
        }

        // let duration = u16::from_le_bytes([wifi[2], wifi[3]]);

        let mut receiver = [0u8; 6];
        receiver.copy_from_slice(&wifi[4..10]);

        // println!("Duration    : {}", duration);
        // println!("Receiver    : {}", self.mac(&receiver));

        // RTS (subtype 11) has TA after RA.
        //
        // PS-Poll (10) also has BSSID/TA-like second address.
        //
        // CF-End / CF-End+CF-Ack have additional address.
        match subtype {
            10 | 11 | 12 | 13 => {
                if wifi.len() >= 16 {
                    let mut addr2 = [0u8; 6];
                    addr2.copy_from_slice(&wifi[10..16]);

                    // println!("Address 2   : {}", self.mac(&addr2));
                }
            }

            _ => {}
        }
    }

    // Beacon parser

    fn parse_beacon(&self, wifi: &[u8], packet_info: &mut PacketInfo) {
        // Management header = 24 bytes
        //
        // Beacon fixed fields:
        //
        // Timestamp  8
        // Interval   2
        // Capability 2
        //
        // Total = 36
        if wifi.len() < 36 {
            return;
        }

        let mut pos = 24;

        let timestamp = u64::from_le_bytes(wifi[pos..pos + 8].try_into().unwrap());

        pos += 8;

        let interval = u16::from_le_bytes([wifi[pos], wifi[pos + 1]]);

        pos += 2;

        let capability = u16::from_le_bytes([wifi[pos], wifi[pos + 1]]);

        pos += 2;

        packet_info.timestamp = Some(timestamp);
        packet_info.beacon_interval = Some(interval);
        packet_info.capability = Some(capability);

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
                    packet_info.ssid = Some(ssid);
                }

                // Supported rates
                1 => {
                    let mut supported_rates = Vec::<f32>::new();

                    for rate in data {
                        let r = (*rate & 0x7F) as f32 * 0.5;

                        supported_rates.push(r);
                    }

                    packet_info.supported_rates = supported_rates;
                }

                // DS Parameter Set
                3 => {
                    if len >= 1 {
                        packet_info.beacon_channel = Some(data[0]);
                    }
                }

                // TIM
                5 => {
                    if len >= 3 {
                        packet_info.dtim_count = Some(data[0]);
                        packet_info.dtim_period = Some(data[1]);
                        packet_info.tim_bitmap_control = Some(data[2]);

                        if len > 3 {
                            packet_info.tim_partial_bitmap = Some(data[3..].to_vec());
                        }
                    }
                }

                // Country
                7 => {
                    if len >= 3 {
                        let country = String::from_utf8_lossy(&data[0..3]).to_string();

                        packet_info.country = Some(country);
                    }
                }

                // ERP
                42 => {
                    if len >= 1 {
                        packet_info.erp_information = Some(data[0]);
                    }
                }

                // RSN
                48 => {
                    packet_info.rsn = Some(data.to_vec());
                    // println!("Security         : RSN (WPA2/WPA3)");
                }

                // HT Capabilities
                45 => {
                    // println!("HT Capabilities");
                }

                // HT Operation
                61 => {
                    // println!("HT Operation");
                }

                // VHT Capabilities
                191 => {
                    // println!("VHT Capabilities");
                }

                // VHT Operation
                192 => {
                    // println!("VHT Operation");
                }

                // Extension
                255 => {
                    // println!("Extension Element");
                }

                // Vendor specific
                221 => {
                    if len >= 3 {
                        // println!(
                        //     "Vendor OUI       : {:02X}:{:02X}:{:02X}",
                        //     data[0], data[1], data[2]
                        // );
                    }
                }

                _ => {}
            }

            pos += len;
        }
    }

    fn parse_eapol(&self, wifi: &[u8]) -> Option<FrameName> {
        if wifi.len() < 24 {
            return None;
        }

        let frame_control = u16::from_le_bytes([wifi[0], wifi[1]]);

        let frame_type = ((frame_control >> 2) & 0x3) as u8;
        let subtype = ((frame_control >> 4) & 0xF) as u8;

        // let to_ds = (frame_control & (1 << 8)) != 0;
        // let from_ds = (frame_control & (1 << 9)) != 0;
        let order = (frame_control & (1 << 15)) != 0;

        // Must be a Data frame (Type 2)
        if frame_type != 2 {
            return None;
        }

        let to_ds = ((frame_control >> 8) & 1) != 0;
        let from_ds = ((frame_control >> 9) & 1) != 0;

        // Calculate dynamic 802.11 Header size
        let mut pos = 24;

        // 4th address present in WDS frames (ToDS=1 and FromDS=1)
        if to_ds && from_ds {
            pos += 6;
        }

        // QoS Data frames (Subtypes 8..15) add 2 bytes for QoS Control
        if subtype >= 8 {
            pos += 2; // QoS Control

            // HT Control is present when Order bit is set
            if order {
                pos += 4; // HT Control
            }
        }

        // Verify buffer length after dynamic header parsing
        if wifi.len() < pos + 8 {
            return None;
        }

        // LLC / SNAP Header Check: AA-AA-03-00-00-00-88-8E
        if wifi[pos..pos + 6] != [0xAA, 0xAA, 0x03, 0x00, 0x00, 0x00] {
            return None;
        }

        let ether_type = u16::from_be_bytes([wifi[pos + 6], wifi[pos + 7]]);
        if ether_type != 0x888E {
            return None;
        }
        pos += 8; // Past LLC/SNAP

        // EAPOL Header: Version(1), Type(1), Length(2)
        if wifi.len() < pos + 4 {
            return Some(FrameName::Eapol);
        }

        let eapol_type = wifi[pos + 1];
        if eapol_type != 3 {
            // 3 = EAPOL-Key
            return Some(FrameName::Eapol);
        }
        pos += 4; // Past EAPOL Header

        // EAPOL-Key Descriptor Type & Key Info
        if wifi.len() < pos + 3 {
            return Some(FrameName::Eapol);
        }

        let descriptor_type = wifi[pos];
        // Descriptor type 2 (WPA2/RSN) or 1 (RC4 / WPA1)
        if descriptor_type != 2 && descriptor_type != 1 {
            return Some(FrameName::Eapol);
        }

        let key_info = u16::from_be_bytes([wifi[pos + 1], wifi[pos + 2]]);

        // Bit positions in Big-Endian Key Info:
        // Bit 3  (1 << 3)  -> Key Type (1 = Pairwise, 0 = Group)
        // Bit 6  (1 << 6)  -> Install
        // Bit 7  (1 << 7)  -> Key Ack
        // Bit 8  (1 << 8)  -> Key MIC
        // Bit 9  (1 << 9)  -> Secure
        // Bit 12 (1 << 12) -> Encrypted Key Data   <-- was incorrectly bit 13
        let key_type_pairwise = (key_info & (1 << 3)) != 0;
        let install = (key_info & (1 << 6)) != 0;
        let key_ack = (key_info & (1 << 7)) != 0;
        let key_mic = (key_info & (1 << 8)) != 0;
        let secure = (key_info & (1 << 9)) != 0;
        let encrypted_data = (key_info & (1 << 12)) != 0; // fixed: was (1 << 13)

        // Ignore Group Key handshake frames (bit 3 == 0)
        if !key_type_pairwise {
            return Some(FrameName::Eapol);
        }

        // Classification Logic for 4-Way Pairwise Handshake
        if key_ack && !key_mic {
            // Msg 1: AP -> STA (Ack set, MIC clear)
            Some(FrameName::EapolKeyMessage1)
        } else if !key_ack && key_mic && !secure {
            // Msg 2: STA -> AP (Ack clear, MIC set, Secure clear)
            Some(FrameName::EapolKeyMessage2)
        } else if key_ack && key_mic && (install || encrypted_data) {
            // Msg 3: AP -> STA (Ack set, MIC set, Install or Encrypted Data set)
            Some(FrameName::EapolKeyMessage3)
        } else if !key_ack && key_mic && secure {
            // Msg 4: STA -> AP (Ack clear, MIC set, Secure set)
            Some(FrameName::EapolKeyMessage4)
        } else {
            Some(FrameName::Eapol)
        }
    }

    fn parse_rsn_security(&self, data: &[u8]) -> Security {
        // RSN:
        //
        // Version              2 bytes
        // Group Cipher         4 bytes
        // Pairwise Count       2 bytes
        // Pairwise Suites      4 * count
        // AKM Count            2 bytes
        // AKM Suites           4 * count

        if data.len() < 8 {
            return Security::Unknown;
        }

        let mut pos = 0;

        // Version
        pos += 2;

        // Group cipher suite
        pos += 4;

        // Pairwise cipher count
        if pos + 2 > data.len() {
            return Security::Unknown;
        }

        let pairwise_count = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;

        pos += 2;

        // Pairwise cipher suites
        let pairwise_len = match pairwise_count.checked_mul(4) {
            Some(len) => len,
            None => return Security::Unknown,
        };

        if pos + pairwise_len > data.len() {
            return Security::Unknown;
        }

        pos += pairwise_len;

        // AKM count
        if pos + 2 > data.len() {
            return Security::Unknown;
        }

        let akm_count = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;

        pos += 2;

        let mut has_psk = false;
        let mut has_sae = false;

        for _ in 0..akm_count {
            if pos + 4 > data.len() {
                return Security::Unknown;
            }

            let oui = &data[pos..pos + 3];
            let akm_type = data[pos + 3];

            // 00:0F:AC = IEEE 802.11 RSN
            if oui == [0x00, 0x0F, 0xAC] {
                match akm_type {
                    // PSK
                    2 | 4 | 6 => {
                        has_psk = true;
                    }

                    // SAE / FT-SAE
                    8 | 9 => {
                        has_sae = true;
                    }

                    _ => {}
                }
            }

            pos += 4;
        }

        match (has_psk, has_sae) {
            (true, true) => Security::Wpa2Wpa3,
            (false, true) => Security::Wpa3,
            (true, false) => Security::Wpa2,
            (false, false) => Security::Unknown,
        }
    }

    pub fn deauth(&self, target: [u8; 6], bssid: [u8; 6]) {
        let mut radio_tab = RadioTab::default();
        radio_tab.len = 8;

        let mut control_frame = ControlFrame::default();
        control_frame.frame_control = 0x00C0;
        control_frame.reason_code = 3;
        control_frame.addr1 = target;
        control_frame.addr2 = bssid;
        control_frame.addr3 = bssid;

        let mut packet = Vec::with_capacity(34);

        packet.extend_from_slice(&radio_tab.to_bytes());
        packet.extend_from_slice(&control_frame.to_bytes());

        let mut addr = sockaddr_ll {
            sll_family: AF_PACKET as u16,
            sll_protocol: (ETH_P_ALL as u16).to_be(),
            sll_ifindex: self.if_index as i32,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 6,
            sll_addr: [0; 8],
        };
        addr.sll_addr[..6].copy_from_slice(&target);

        let iov = [IoSlice::new(&packet)];

        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };

        msg.msg_name = &mut addr as *mut _ as *mut libc::c_void;
        msg.msg_namelen = std::mem::size_of::<sockaddr_ll>() as libc::socklen_t;
        msg.msg_iov = iov.as_ptr() as *mut libc::iovec;
        msg.msg_iovlen = iov.len();

        let result = unsafe { libc::sendmsg(self.sock_fd.as_raw_fd(), &msg, 0) };

        if result < 0 {
            eprintln!("sendmsg failed: {}", std::io::Error::last_os_error());
        } else {
            println!("sendmsg accepted {} bytes", result);
        }
    }

    fn mac(&self, addr: &[u8; 6]) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            addr[0], addr[1], addr[2], addr[3], addr[4], addr[5],
        )
    }
}
