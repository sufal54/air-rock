use crate::af_eth::service::Service;

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

    pub fn accept(&self) {
        self.service.accept();
    }
    pub fn accept_timeout(&self, timeout_ms: i32) {
        self.service.accept_timeout(timeout_ms);
    }
}
