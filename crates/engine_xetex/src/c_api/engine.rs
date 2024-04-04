use std::cell::UnsafeCell;

#[cfg(target_endian = "big")]
mod defs {
    #[repr(C)]
    pub struct b32x2 {
        pub s1: i32,
        pub s0: i32,
    }

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
    #[repr(C)]
    pub struct b32x2 {
        pub s0: i32,
        pub s1: i32,
    }

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

#[allow(nonstandard_style)]
extern "C" {
    pub fn font_mapping_warning(
        mappingNameP: *const libc::c_void,
        mappingNameLen: i32,
        warningType: i32,
    );

    pub static loaded_font_flags: UnsafeCell<libc::c_char>;
    pub static loaded_font_mapping: UnsafeCell<*const libc::c_void>;
}
