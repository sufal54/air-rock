use crate::nl_route::service::Service;

pub struct Ctrl {
    service: Service,
}

impl Ctrl {
    pub fn new(interface: String) -> Self {
        Self {
            service: Service::new(interface),
        }
    }
    pub fn set_interface_power(&mut self, is_on: bool) {
        self.service.set_interface_power(is_on);
    }
}
