use std::time::Instant;

pub struct RadioInfo {
    pub signal_dbm: Option<i8>,
    pub noise_dbm: Option<i8>,
    pub rate: Option<f32>,
    pub channel_freq: Option<u16>,
    pub channel_flags: Option<u16>,
    pub antenna: Option<u8>,
    pub flags: Option<u8>,
}

pub struct WifiFrame {
    pub frame_type: u8,
    pub subtype: u8,

    pub to_ds: bool,
    pub from_ds: bool,
    pub protected: bool,

    pub duration: u16,

    pub receiver: [u8; 6],
    pub transmitter: [u8; 6],
    pub bssid: [u8; 6],

    pub sequence: u16,
    pub fragment: u8,
}

pub struct BeaconInfo {
    pub bssid: [u8; 6],
    pub ssid: String,
    pub channel: u8,
    pub signal: Option<i8>,
    pub encryption: String,
}

pub struct Device {
    pub mac: [u8; 6],
    pub signal: i8,
    pub last_seen: Instant,
}
