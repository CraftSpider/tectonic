use std::ffi::CString;

pub unsafe fn get_tex_str(s: i32) -> CString {
    CString::from_raw(gettexstring(s))
}

extern "C" {
    fn gettexstring(s: i32) -> *mut libc::c_char;
    pub fn maketexstring(s: *const libc::c_char) -> i32;
}
