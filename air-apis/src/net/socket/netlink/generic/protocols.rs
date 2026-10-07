use std::ffi::c_int;

pub const GENL_ID_CTRL: c_int = 0x10;
pub const CTRL_CMD_GETFAMILY: c_int = 3;
pub const CTRL_ATTR_FAMILY_ID: c_int = 1;
pub const CTRL_ATTR_FAMILY_NAME: c_int = 2;
pub const NLM_F_ACK: c_int = 4;
pub const NLMSG_DONE: c_int = 3;
pub const NLMSG_ERROR: c_int = 2;
