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
}

impl RadioTab {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);

        // Add the actual fields of your RadioTab here.
        // Example:
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.pad.to_le_bytes());
        bytes.extend_from_slice(&self.len.to_le_bytes());
        bytes.extend_from_slice(&self.present.to_le_bytes());

        bytes
    }
}

#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct ControlFrame {
    pub frame_control: u16,
    pub duration_id: u16,
    pub addr1: [u8; 6],
    pub addr2: [u8; 6],
    pub addr3: [u8; 6],
    pub sequence_ctrl: u16,
    pub reason_code: u16,
}

impl ControlFrame {
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
