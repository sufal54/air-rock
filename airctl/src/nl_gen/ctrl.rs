use crate::{
    helper::CHANNEL_FREQ_MAP,
    nl_gen::service::{InterfaceInfo, Service},
};

pub struct Ctrl {
    service: Service,
}

impl Ctrl {
    pub fn new(interface: String) -> Self {
        let service = Service::new(interface);

        return Self { service };
    }

    pub fn bind(&mut self) {
        self.service.sock_bind();
    }

    pub fn interface_info(&mut self) -> Option<InterfaceInfo> {
        self.service.interface_info()
    }

    pub fn set_frequency(&mut self, freq: u32) -> bool {
        self.service.set_frequency(freq)
    }

    pub fn start_monitor(&mut self) -> bool {
        self.service.start_monitor()
    }
    pub fn stop_monitor(&mut self) -> bool {
        self.service.stop_monitor()
    }
}
