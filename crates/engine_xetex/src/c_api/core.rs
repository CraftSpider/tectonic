use crate::teckit::{kForm_UTF16BE, kForm_UTF16LE};

pub const FONT_FLAGS_COLORED: libc::c_char = 0x01;
pub const FONT_FLAGS_VERTICAL: libc::c_char = 0x02;

pub const AUTO: libc::c_int = 0;
pub const UTF8: libc::c_int = 1;
pub const UTF16BE: libc::c_int = 2;
pub const UTF16LE: libc::c_int = 3;
pub const RAW: libc::c_int = 4;
pub const ICUMAPPING: libc::c_int = 5;
pub const US_NATIVE_UTF16: libc::c_int = if cfg!(target_endian = "big") {
    UTF16BE
} else {
    UTF16LE
};
pub const UTF16_NATIVE: u16 = if cfg!(target_endian = "big") {
    kForm_UTF16BE
} else {
    kForm_UTF16LE
};

#[allow(nonstandard_style)]
pub type scaled_t = i32;

pub type UTF16Code = libc::c_ushort;
