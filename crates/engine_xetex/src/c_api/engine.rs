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

pub use defs::*;

#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: f64,
    pub ptr: *mut (),
}

pub fn print_str(s: &[u8]) {
    for c in s {
        unsafe { print_char(*c as i32) };
    }
}

pub unsafe fn file_name() -> &'static CStr {
    let ptr = *name_of_file.get();
    CStr::from_ptr(ptr)
}

#[allow(nonstandard_style)]
extern "C" {
    pub fn font_mapping_warning(
        mappingNameP: *const libc::c_void,
        mappingNameLen: i32,
        warningType: i32,
    );
    pub fn begin_diagnostic();
    pub fn print_nl(s: i32);
    pub fn print_char(c: i32);
    pub fn print_int(n: i32);
    pub fn end_diagnostic(blank_line: bool);

    pub static loaded_font_flags: UnsafeCell<libc::c_char>;
    pub static loaded_font_mapping: UnsafeCell<*const libc::c_void>;
    pub static font_area: UnsafeCell<*mut i32>;
    pub static font_layout_engine: UnsafeCell<*mut *mut ()>;
    pub static name_of_file: UnsafeCell<*mut libc::c_char>;
}
