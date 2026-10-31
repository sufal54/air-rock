use std::ffi::c_int;

pub const GENL_ID_CTRL: c_int = 0x10;
pub const CTRL_CMD_GETFAMILY: c_int = 0x03;
pub const CTRL_ATTR_FAMILY_ID: c_int = 0x01;
pub const CTRL_ATTR_FAMILY_NAME: c_int = 0x02;
pub const NLM_F_ACK: c_int = 0x04;
pub const NLMSG_DONE: c_int = 0x03;
pub const NLMSG_ERROR: c_int = 0x02;
