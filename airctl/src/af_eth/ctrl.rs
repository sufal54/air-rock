use crate::af_eth::{PacketInfo, RadioInfo, service::Service};

pub struct Ctrl {
    service: Service,
}

impl Ctrl {
    pub fn new(interface: String) -> Self {
        Self {
            service: Service::new(interface),
        }
    }
    pub fn sock_bind(&self) {
        self.service.sock_bind();
    }

    pub fn accept(&self) -> Option<(RadioInfo, PacketInfo, Vec<u8>)> {
        self.service.accept()
    }
    pub fn accept_timeout(&mut self, timeout_ms: i32) -> Option<(RadioInfo, PacketInfo, Vec<u8>)> {
        self.service.accept_timeout(timeout_ms)
    }

    pub fn drain(&mut self) {
        self.service.drain();
    }
    pub fn deauth(&self, target: [u8; 6], bssid: [u8; 6]) {
        self.service.deauth(target, bssid);
    }

    pub fn beacons(&self, bssid: [u8; 6], ssid: &str, channel: u16, seq_counter: u16) {
        self.service.beacons(bssid, ssid, channel, seq_counter);
    }

    pub fn prope_responce(
        &self,
        ap_mac: [u8; 6],
        client_mac: [u8; 6],
        ssid: &str,
        channel: u16,
        seq_counter: u16,
    ) {
        self.service
            .probe_response(ap_mac, client_mac, ssid, channel, seq_counter);
    }
}
