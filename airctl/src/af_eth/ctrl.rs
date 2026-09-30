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
}
