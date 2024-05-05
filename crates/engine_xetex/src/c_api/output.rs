use crate::c_api::core::scaled_t;

pub fn print_str(s: &[u8]) {
    for c in s {
        unsafe { print_char(*c as i32) };
    }
}

/// cbindgen:ignore
extern "C" {
    pub fn error_here_with_diagnostic(message: *const libc::c_char) -> *mut ();
    pub fn print_scaled(s: scaled_t);
    pub fn print_char(c: i32);
    pub fn print_nl(s: i32);
    pub fn print_int(n: i32);
    pub fn print_cstr(str: *const libc::c_char);
    // TODO: ttbc_diagnostic_whatever
    pub fn capture_to_diagnostic(diag: *mut ());
}
