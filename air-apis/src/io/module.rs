#[repr(C)]
pub struct pollfd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}
