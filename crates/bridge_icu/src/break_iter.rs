use crate::{sys, BreakIteratorType, IcuErr};
use std::ffi::CStr;
use std::ptr;
use std::ptr::NonNull;

pub struct BreakIterator(NonNull<sys::UBreakIterator>);

impl BreakIterator {
    pub fn new(locale: &CStr) -> Result<BreakIterator, IcuErr> {
        let mut err = sys::U_ZERO_ERROR;
        let ptr = unsafe {
            sys::ubrk_open(
                BreakIteratorType::Line,
                locale.as_ptr(),
                ptr::null(),
                0,
                &mut err,
            )
        };
        if sys::U_SUCCESS(err) {
            Ok(BreakIterator(NonNull::new(ptr).unwrap()))
        } else {
            Err(IcuErr::from_raw(err))
        }
    }

    pub fn set_text(&mut self, text: &[u16]) -> Result<(), IcuErr> {
        let mut err = sys::U_ZERO_ERROR;
        unsafe {
            sys::ubrk_setText(
                self.0.as_ptr(),
                text.as_ptr(),
                text.len() as libc::c_int,
                &mut err,
            )
        };
        if sys::U_SUCCESS(err) {
            Ok(())
        } else {
            Err(IcuErr::from_raw(err))
        }
    }

    pub fn next(&mut self) -> i32 {
        unsafe { sys::ubrk_next(self.0.as_ptr()) }
    }
}

impl Drop for BreakIterator {
    fn drop(&mut self) {
        unsafe { sys::ubrk_close(self.0.as_ptr()) }
    }
}
