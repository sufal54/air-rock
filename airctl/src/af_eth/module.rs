use std::{fmt, hash::Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameName {
    #[default]
    Unknown,

    // Management
    AssociationRequest,
    AssociationResponse,
    ReassociationRequest,
    ReassociationResponse,
    ProbeRequest,
    ProbeResponse,
    Beacon,
    Atim,
    Disassociation,
    Authentication,
    Deauthentication,
    Action,

    // Control
    BlockAckRequest,
    BlockAck,
    PsPoll,
    Rts,
    Cts,
    Ack,
    CfEnd,
    CfEndCfAck,

    // Data
    Data,

    Eapol,
    EapolKeyMessage1,
    EapolKeyMessage2,
    EapolKeyMessage3,
    EapolKeyMessage4,
}
#[derive(Debug, Clone, Default)]
pub struct RadioInfo {
    pub length: u16,

    pub tsft: Option<u64>,
    pub flags: Option<u8>,
    pub rate: Option<f32>,

    pub frequency: Option<u16>,
    pub channel_flags: Option<u16>,

    pub signal_power: Option<i8>,
    pub noise_power: Option<i8>,

    pub lock_quality: Option<u16>,

    pub tx_attenuation: Option<u16>,
    pub tx_attenuation_db: Option<u16>,
    pub tx_power: Option<i8>,

    pub antenna: Option<u8>,

    pub signal_quality: Option<u8>,
    pub noise_quality: Option<u8>,

    pub rx_flags: Option<u16>,
    pub tx_flags: Option<u16>,

    pub mcs: Option<[u8; 3]>,
    pub vht: Option<[u8; 12]>,
    pub he: Option<[u8; 12]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Security {
    #[default]
    Unknown,
    Open,
    Wpa1,
    Wpa2,
    Wpa3,
    Wpa2Wpa3,
}

impl fmt::Display for Security {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Security::Unknown => "Unknown",
            Security::Open => "Open",
            Security::Wpa1 => "WPA",
            Security::Wpa2 => "WPA2",
            Security::Wpa3 => "WPA3",
            Security::Wpa2Wpa3 => "WPA2/WPA3",
        };

        write!(f, "{name}")
    }
}

#[derive(Debug, Clone, Default)]
pub struct PacketInfo {
    // Radiotap
    pub radiotap_length: usize,
    pub capture_frequency: Option<u16>,

    // 802.11 Frame Control
    pub version: u8,
    pub frame_type: u8,
    pub subtype: u8,

    pub security: Security,

    pub to_ds: bool,
    pub from_ds: bool,
    pub more_frag: bool,
    pub retry: bool,
    pub power_mgmt: bool,
    pub more_data: bool,
    pub protected: bool,
    pub order: bool,

    // Frame
    pub frame_name: FrameName,

    // MAC Header
    pub duration: Option<u16>,
    pub receiver: String,
    pub transmitter: String,
    pub bssid: String,

    pub sequence: Option<u16>,
    pub fragment: Option<u16>,

    // Beacon / Probe Response
    pub ssid: Option<String>,

    /// Supported rates in Mbps
    pub supported_rates: Vec<f32>,

    /// Beacon timestamp
    pub timestamp: Option<u64>,

    /// Beacon interval in TU
    pub beacon_interval: Option<u16>,

    /// Beacon capability information
    pub capability: Option<u16>,

    /// Channel advertised by the Beacon
    pub beacon_channel: Option<u8>,

    pub country: Option<String>,

    /// TIM information
    pub dtim_count: Option<u8>,
    pub dtim_period: Option<u8>,
    pub tim_bitmap_control: Option<u8>,
    pub tim_partial_bitmap: Option<Vec<u8>>,

    /// ERP information
    pub erp_information: Option<u8>,

    /// RSN security information
    pub rsn: Option<Vec<u8>>,
}

impl PartialEq for PacketInfo {
    fn eq(&self, other: &Self) -> bool {
        self.bssid == other.bssid
    }
}

impl Eq for PacketInfo {}

impl std::hash::Hash for PacketInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bssid.hash(state);
    }
}

#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct RadioTab {
    pub version: u8,
    pub pad: u8,
    pub len: u16,
    pub present: u32,

    pub tsft: Option<u64>,
    pub flags: Option<u8>,
    pub rate: u8,
    pub channel_freq: u16,
    pub channel_flags: u16,
}

impl RadioTab {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.version);
        bytes.push(self.pad);
        bytes.extend_from_slice(&self.len.to_le_bytes());
        bytes.extend_from_slice(&self.present.to_le_bytes());
        if self.present != 0 {
            if let Some(tsft) = self.tsft {
                bytes.extend_from_slice(&tsft.to_le_bytes());
            }

            if let Some(flags) = self.flags {
                bytes.extend_from_slice(&flags.to_le_bytes());
            }
            bytes.push(self.rate);

            while bytes.len() % 2 != 0 {
                bytes.push(0);
            }

            bytes.extend_from_slice(&self.channel_freq.to_le_bytes());
            bytes.extend_from_slice(&self.channel_flags.to_le_bytes());
        }
        bytes
    }
}

#[derive(Default)]
#[repr(C, packed)]
pub struct ControlFrame {}

#[derive(Default)]
#[repr(C, packed)]
pub struct Dauth {
    pub frame_control: u16,
    pub duration_id: u16,
    pub addr1: [u8; 6],
    pub addr2: [u8; 6],
    pub addr3: [u8; 6],
    pub sequence_ctrl: u16,
    pub reason_code: u16,
}

impl Dauth {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(26);

        bytes.extend_from_slice(&self.frame_control.to_le_bytes());
        bytes.extend_from_slice(&self.duration_id.to_le_bytes());
        bytes.extend_from_slice(&self.addr1);
        bytes.extend_from_slice(&self.addr2);
        bytes.extend_from_slice(&self.addr3);
        bytes.extend_from_slice(&self.sequence_ctrl.to_le_bytes());
        bytes.extend_from_slice(&self.reason_code.to_le_bytes());

        bytes
    }
}

#[derive(Default)]
#[repr(C, packed)]
pub struct Bcon {
    pub frame_control: u16,
    pub duration_id: u16,
    pub addr1: [u8; 6],
    pub addr2: [u8; 6],
    pub addr3: [u8; 6],
    pub sequence_ctrl: u16,

    pub timestamp: u64,
    pub beacon_interval: u16,
    pub capabilities: u16,
    // SSID
    pub ssid_pad: u8,
    pub ssid_len: u8,
    pub ssid: [u8; 32],

    // Supported Rates Element
    pub supported_rates_id: u8,
    pub supported_rates_len: u8,
    pub supported_rates: [u8; 8],

    // DS Parameter Set / Channel Element
    pub ds_parameter_set_id: u8,
    pub ds_parameter_set_len: u8,
    pub channel: u8,

    // Country Information Element
    pub country_id: u8,
    pub country_len: u8,
    pub country_code: [u8; 3], // Country code
    pub first_channel: u8,
    pub channel_count: u8, // Number of channels
    pub max_tx_power: u8,  // Maximum transmit power (dBm)

    // TIM Element
    pub tim_id: u8,
    pub tim_len: u8,
    pub dtim_count: u8,
    pub dtim_period: u8,
    pub bitmap_control: u8,
    pub partial_virtual_bitmap: u8,

    // RSN Element
    pub rsn_id: u8,
    pub rsn_len: u8,
    pub rsn_version: u16,
    pub group_cipher: [u8; 4],
    pub pairwise_cipher_count: u16,
    pub pairwise_cipher: [u8; 4],
    pub akm_count: u16,
    pub akm_suite: [u8; 4],
    pub rsn_capabilities: u16,

    // Extended Supported Rates Element
    pub extended_supported_rates_id: u8,
    pub extended_supported_rates_len: u8,
    pub extended_supported_rates: [u8; 4],
}

impl Bcon {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.frame_control.to_le_bytes());
        bytes.extend_from_slice(&self.duration_id.to_le_bytes());
        bytes.extend_from_slice(&self.addr1);
        bytes.extend_from_slice(&self.addr2);
        bytes.extend_from_slice(&self.addr3);
        bytes.extend_from_slice(&self.sequence_ctrl.to_le_bytes());

        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.beacon_interval.to_le_bytes());
        bytes.extend_from_slice(&self.capabilities.to_le_bytes());

        bytes.push(self.ssid_pad);
        bytes.push(self.ssid_len);
        bytes.extend_from_slice(&self.ssid[..self.ssid_len as usize]);

        bytes.push(self.supported_rates_id);
        bytes.push(self.supported_rates_len);
        bytes.extend_from_slice(&self.supported_rates);

        bytes.push(self.ds_parameter_set_id);
        bytes.push(self.ds_parameter_set_len);
        bytes.push(self.channel);

        bytes.push(self.country_id);
        bytes.push(self.country_len);
        bytes.extend_from_slice(&self.country_code);
        bytes.push(self.first_channel);
        bytes.push(self.channel_count);
        bytes.push(self.max_tx_power);

        bytes.push(self.tim_id);
        bytes.push(self.tim_len);
        bytes.push(self.dtim_count);
        bytes.push(self.dtim_period);
        bytes.push(self.bitmap_control);
        bytes.push(self.partial_virtual_bitmap);

        // RSN Element
        bytes.push(self.rsn_id);
        bytes.push(self.rsn_len);
        bytes.extend_from_slice(&self.rsn_version.to_le_bytes());
        bytes.extend_from_slice(&self.group_cipher);
        bytes.extend_from_slice(&self.pairwise_cipher_count.to_le_bytes());
        bytes.extend_from_slice(&self.pairwise_cipher);
        bytes.extend_from_slice(&self.akm_count.to_le_bytes());
        bytes.extend_from_slice(&self.akm_suite);
        bytes.extend_from_slice(&self.rsn_capabilities.to_le_bytes());

        // Extended Supported Rates Element
        bytes.push(self.extended_supported_rates_id);
        bytes.push(self.extended_supported_rates_len);
        bytes.extend_from_slice(&self.extended_supported_rates);

        bytes
    }
}

#[derive(Default)]
#[repr(C, packed)]
pub struct PropRes {
    pub frame_control: u16,
    pub duration_id: u16,
    pub addr1: [u8; 6],
    pub addr2: [u8; 6],
    pub addr3: [u8; 6],
    pub sequence_ctrl: u16,

    pub timestamp: u64,
    pub beacon_interval: u16,
    pub capabilities: u16,
    // SSID
    pub ssid_pad: u8,
    pub ssid_len: u8,
    pub ssid: [u8; 32],

    // Supported Rates Element
    pub supported_rates_id: u8,
    pub supported_rates_len: u8,
    pub supported_rates: [u8; 8],

    // DS Parameter Set / Channel Element
    pub ds_parameter_set_id: u8,
    pub ds_parameter_set_len: u8,
    pub channel: u8,

    // RSN Element
    pub rsn_id: u8,
    pub rsn_len: u8,
    pub rsn_version: u16,
    pub group_cipher: [u8; 4],
    pub pairwise_cipher_count: u16,
    pub pairwise_cipher: [u8; 4],
    pub akm_count: u16,
    pub akm_suite: [u8; 4],
    pub rsn_capabilities: u16,
}

impl PropRes {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(26);

        bytes.extend_from_slice(&self.frame_control.to_le_bytes());
        bytes.extend_from_slice(&self.duration_id.to_le_bytes());
        bytes.extend_from_slice(&self.addr1);
        bytes.extend_from_slice(&self.addr2);
        bytes.extend_from_slice(&self.addr3);
        bytes.extend_from_slice(&self.sequence_ctrl.to_le_bytes());

        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.beacon_interval.to_le_bytes());
        bytes.extend_from_slice(&self.capabilities.to_le_bytes());

        bytes.push(self.ssid_pad);
        bytes.push(self.ssid_len);
        bytes.extend_from_slice(&self.ssid[..self.ssid_len as usize]);

        bytes.push(self.supported_rates_id);
        bytes.push(self.supported_rates_len);
        bytes.extend_from_slice(&self.supported_rates);

        bytes.push(self.ds_parameter_set_id);
        bytes.push(self.ds_parameter_set_len);
        bytes.push(self.channel);

        // RSN Element
        bytes.push(self.rsn_id);
        bytes.push(self.rsn_len);
        bytes.extend_from_slice(&self.rsn_version.to_le_bytes());
        bytes.extend_from_slice(&self.group_cipher);
        bytes.extend_from_slice(&self.pairwise_cipher_count.to_le_bytes());
        bytes.extend_from_slice(&self.pairwise_cipher);
        bytes.extend_from_slice(&self.akm_count.to_le_bytes());
        bytes.extend_from_slice(&self.akm_suite);
        bytes.extend_from_slice(&self.rsn_capabilities.to_le_bytes());

        bytes
    }
}

impl ControlFrame {
    pub fn dauth() -> Dauth {
        Dauth::default()
    }

    pub fn bcon() -> Bcon {
        Bcon::default()
    }

    pub fn prop_res() -> PropRes {
        PropRes::default()
    }
}
