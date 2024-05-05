use std::cell::UnsafeCell;
use std::ffi::CStr;

#[cfg(target_endian = "big")]
mod defs {
    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b32x2 {
        pub s1: i32,
        pub s0: i32,
    }

    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b16x4 {
        pub s3: u16,
        pub s2: u16,
        pub s1: u16,
        pub s0: u16,
    }
}

#[cfg(target_endian = "little")]
mod defs {
    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b32x2 {
        pub s0: i32,
        pub s1: i32,
    }

    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b16x4 {
        pub s0: u16,
        pub s1: u16,
        pub s2: u16,
        pub s3: u16,
    }
}

use crate::c_api::core::{scaled_t, UTF16Code};
pub use defs::*;

#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: f64,
    pub ptr: *mut (),
}

pub unsafe fn file_name() -> &'static CStr {
    let ptr = *name_of_file.get();
    CStr::from_ptr(ptr)
}

/// cbindgen:ignore
#[allow(nonstandard_style)]
extern "C" {
    pub fn font_mapping_warning(
        mappingNameP: *const libc::c_void,
        mappingNameLen: i32,
        warningType: i32,
    );
    pub fn begin_diagnostic();
    pub fn end_diagnostic(blank_line: bool);
    pub fn font_feature_warning(
        featureNameP: *const libc::c_void,
        featLen: i32,
        settingNameP: *const libc::c_void,
        setLen: i32,
    );
    pub fn get_tracing_fonts_state() -> i32;
    pub fn print_raw_char(s: UTF16Code, incr_offset: bool);

    pub static loaded_font_flags: UnsafeCell<libc::c_char>;
    pub static loaded_font_mapping: UnsafeCell<*const libc::c_void>;
    pub static loaded_font_letter_space: UnsafeCell<scaled_t>;
    pub static font_area: UnsafeCell<*mut i32>;
    pub static font_layout_engine: UnsafeCell<*mut *mut ()>;
    pub static native_font_type_flag: UnsafeCell<i32>;
    pub static name_of_file: UnsafeCell<*mut libc::c_char>;
    pub static arith_error: UnsafeCell<bool>;
    pub static tex_remainder: UnsafeCell<scaled_t>;
    pub static help_ptr: UnsafeCell<libc::c_char>;
    pub static help_line: UnsafeCell<[*const libc::c_char; 6]>;
    pub static randoms: UnsafeCell<[i32; 55]>;
    pub static j_random: UnsafeCell<libc::c_uchar>;
}
