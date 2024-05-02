macro_rules! versioned_names {
    (
        $(
        pub fn $name:ident($($argname:ident: $argty:ty),* $(,)?) $(-> $output:ty)?;
        )+
    ) => {
        $(
        #[link_name = concat!(stringify!($name), "_", env!("ICU_MAJOR_VERSION"))]
        pub fn $name($($argname: $argty),*) $(-> $output)?;
        )*
    };
}

pub const UBIDI_DEFAULT_LTR: u8 = 0xFE;
pub const UBIDI_DEFAULT_RTL: u8 = 0xFF;
pub const U_ZERO_ERROR: UErrorCode = 0;

pub type UErrorCode = libc::c_int;
pub type UChar = u16;
pub type UChar32 = i32;

pub fn U_SUCCESS(code: UErrorCode) -> bool {
    code <= U_ZERO_ERROR
}

#[repr(C)]
pub enum UBreakIteratorType {
    Character,
    Word,
    Line,
    Sentence,
    Title,
    Count,
}

#[repr(C)]
pub struct UBreakIterator(());

#[repr(C)]
pub struct UConverter(());

extern "C" {
    versioned_names! {
        pub fn ucnv_open(name: *const libc::c_char, err: *mut UErrorCode) -> *mut UConverter;
        pub fn ucnv_close(conv: *mut UConverter);
        pub fn ucnv_toUChars(
            conv: *mut UConverter,
            dest: *mut UChar,
            dest_capacity: i32,
            src: *const libc::c_char,
            src_len: i32,
            p_error_code: *mut UErrorCode,
        ) -> i32;
        pub fn ucnv_fromUChars(
            conv: *mut UConverter,
            dest: *mut libc::c_char,
            dest_capacity: i32,
            src: *const UChar,
            src_len: i32,
            p_error_code: *mut UErrorCode,
        ) -> i32;
        pub fn ucnv_getMaxCharSize(
            conv: *mut UConverter,
        ) -> i8;
        pub fn ubrk_open(ty: UBreakIteratorType, locale: *const libc::c_char, text: *const UChar, textLength: i32, status: *mut UErrorCode) -> *mut UBreakIterator;
        pub fn ubrk_next(bi: *mut UBreakIterator) -> i32;
        pub fn ubrk_setText(bi: *mut UBreakIterator, text: *const UChar, textLength: i32, status: *mut UErrorCode);
        pub fn ubrk_close(bi: *mut UBreakIterator);
    }
}
