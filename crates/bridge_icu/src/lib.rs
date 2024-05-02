// Copyright 2020 the Tectonic Project
// Licensed under the MIT License.

#![allow(nonstandard_style)]

//! This crate exists to export the ICU *C* API into the Cargo framework, as well as
//! provide bindings to other tectonic crates.

mod sys;

mod break_iter;
mod converter;

pub use sys::UBreakIteratorType as BreakIteratorType;
pub use sys::{UChar32, UBIDI_DEFAULT_LTR, UBIDI_DEFAULT_RTL};

pub use break_iter::BreakIterator;
pub use converter::Converter;

#[derive(PartialEq, Debug)]
pub struct IcuErr(sys::UErrorCode);

impl IcuErr {
    fn from_raw(err: sys::UErrorCode) -> IcuErr {
        IcuErr(err)
    }

    pub fn into_raw(self) -> sys::UErrorCode {
        self.0
    }
}

impl Default for IcuErr {
    fn default() -> Self {
        IcuErr(-1)
    }
}
