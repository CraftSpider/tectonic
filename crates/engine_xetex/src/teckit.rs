#![allow(nonstandard_style)]

// TODO: Move this to bridge_teckit or such

pub const kForm_Bytes: u16 = 1;
pub const kForm_UTF16BE: u16 = 3;
pub const kForm_UTF16LE: u16 = 4;

#[repr(C)]
pub struct Opaque_TECkit_Converter(());

pub type TECkit_Converter = *mut Opaque_TECkit_Converter;
pub type TECkit_Status = libc::c_long;
pub type UniChar = u16;

extern "C" {
    pub fn TECkit_CreateConverter(
        mapping: *mut u8,
        mappingSize: u32,
        mapForward: u8,
        inputForm: u16,
        outputForm: u16,
        converter: *mut TECkit_Converter,
    ) -> TECkit_Status;
    pub fn TECkit_ConvertBuffer(
        converter: TECkit_Converter,
        in_buffer: *const u8,
        in_len: u32,
        in_used: *mut u32,
        out_buffer: *mut u8,
        out_len: u32,
        out_used: *mut u32,
        input_is_complete: u8,
    ) -> TECkit_Status;
    pub fn TECkit_ResetConverter(converter: TECkit_Converter) -> TECkit_Status;
}
