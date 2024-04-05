use crate::c_api::core::{
    scaled_t, AUTO, FONT_FLAGS_COLORED, ICUMAPPING, RAW, US_NATIVE_UTF16, UTF16BE, UTF16LE, UTF8,
};
use crate::c_api::engine::{
    begin_diagnostic, end_diagnostic, file_name, font_area, font_layout_engine, loaded_font_flags,
    loaded_font_mapping, memory_word, print_int, print_nl, print_str,
};
use crate::c_api::mfmp::{get_tex_str, maketexstring};
use std::cell::{Cell, UnsafeCell};
use std::ffi::CStr;
use std::{ptr, slice};
use tectonic_bridge_harfbuzz as hb;
use tectonic_bridge_icu::{
    ubrk_close, ubrk_next, ubrk_open, ubrk_setText, ucnv_close, ucnv_open, UBreakIterator,
    UBreakIteratorType, U_FAILURE, U_ZERO_ERROR,
};
use tectonic_xetex_layout::c_api::engine::{
    findNextGraphiteBreak, initGraphiteBreaking, XeTeXLayoutEngineBase,
};
use tectonic_xetex_layout::c_api::{Fixed, GlyphID, XeTeXLayoutEngine};

pub const NATIVE_INFO_OFFSET: usize = 4;
pub const OTGR_FONT_FLAG: u32 = 0xFFFE;

thread_local! {
    static BRK_ITER: Cell<*mut UBreakIterator> = Cell::new(ptr::null_mut());
    static BRK_LOCALE_STR_NUM: Cell<i32> = Cell::new(0);
}

unsafe fn native_glyph_count(node: *mut memory_word) -> u16 {
    (*node.add(NATIVE_INFO_OFFSET)).b16.s0
}

fn d_to_fix(d: f64) -> Fixed {
    (d * 65536.0 + 0.5) as Fixed
}

fn fix_to_d(f: Fixed) -> f64 {
    f as f64 / 65536.0
}

#[no_mangle]
pub unsafe extern "C" fn linebreak_start(
    f: libc::c_int,
    locale_str_num: i32,
    text: *mut u16,
    text_len: i32,
) {
    let locale = get_tex_str(locale_str_num);

    if *(*font_area.get()).add(f as usize) as u32 == OTGR_FONT_FLAG && locale.to_bytes() == b"G" {
        let engine = (*font_layout_engine.get())
            .add(f as usize)
            .cast::<XeTeXLayoutEngineBase>();
        if initGraphiteBreaking(engine, text, text_len as libc::c_uint) {
            return;
        }
    }

    if locale_str_num != BRK_LOCALE_STR_NUM.get() && !BRK_ITER.get().is_null() {
        ubrk_close(BRK_ITER.get());
        BRK_ITER.set(ptr::null_mut());
    }

    let mut status = U_ZERO_ERROR;
    if BRK_ITER.get().is_null() {
        BRK_ITER.set(ubrk_open(
            UBreakIteratorType::Line,
            locale.as_ptr(),
            ptr::null(),
            0,
            &mut status,
        ));
        if U_FAILURE(status) {
            begin_diagnostic();
            print_nl(b'E' as i32);
            print_str(b"rror ");
            print_int(status);
            print_str(b" creating linebreak iterator for locale `");
            print_str(locale.to_bytes());
            print_str(b"'; trying default locale `en_us'.");
            end_diagnostic(true);
            if !BRK_ITER.get().is_null() {
                ubrk_close(BRK_ITER.get());
            }
            status = U_ZERO_ERROR;
            BRK_ITER.set(ubrk_open(
                UBreakIteratorType::Line,
                b"en_us\0".as_ptr().cast(),
                ptr::null(),
                0,
                &mut status,
            ));
        }
        BRK_LOCALE_STR_NUM.set(locale_str_num);
    }

    if BRK_ITER.get().is_null() {
        panic!("failed to create linebreak iterator, status={}", status);
    }

    ubrk_setText(BRK_ITER.get(), text, text_len, &mut status);
}

#[no_mangle]
pub unsafe extern "C" fn linebreak_next() -> libc::c_int {
    if !BRK_ITER.get().is_null() {
        ubrk_next(BRK_ITER.get())
    } else {
        findNextGraphiteBreak()
    }
}

#[no_mangle]
pub unsafe extern "C" fn get_encoding_mode_and_info(info: *mut i32) -> libc::c_int {
    /* \XeTeXinputencoding "enc-name"
     *   -> name is packed in |nameoffile| as a C string, starting at [1]
     * Check if it's a built-in name; if not, try to open an ICU converter by that name
     */
    *info = 0;
    let file_name = file_name().to_bytes();
    if file_name.eq_ignore_ascii_case(b"auto") {
        AUTO
    } else if file_name.eq_ignore_ascii_case(b"utf8") {
        UTF8
    } else if file_name.eq_ignore_ascii_case(b"utf16") {
        US_NATIVE_UTF16
    } else if file_name.eq_ignore_ascii_case(b"utf16be") {
        UTF16BE
    } else if file_name.eq_ignore_ascii_case(b"utf16le") {
        UTF16LE
    } else if file_name.eq_ignore_ascii_case(b"bytes") {
        RAW
    } else {
        let mut err = U_ZERO_ERROR;
        let cnv = ucnv_open(file_name.as_ptr().cast(), &mut err);
        if cnv.is_null() {
            begin_diagnostic();
            print_nl(b'U' as i32); /* ensure message starts on a new line */
            print_str(b"nknown encoding `");
            print_str(file_name);
            print_str(b"'; reading as raw bytes");
            end_diagnostic(true);
            RAW
        } else {
            ucnv_close(cnv);
            *info = maketexstring(file_name.as_ptr().cast());
            ICUMAPPING
        }
    }
}

fn rs_read_double(s: &mut &[u8]) -> f64 {
    let mut neg = false;
    let mut val = 0.0;
    let mut cp = *s;

    while cp[0] == b' ' || cp[0] == b'\t' {
        cp = &cp[1..];
    }

    if cp[0] == b'-' {
        neg = true;
        cp = &cp[1..];
    } else if cp[0] == b'*' {
        cp = &cp[1..];
    }

    while (b'0'..=b'9').contains(&cp[0]) {
        val = val * 10.0 + (cp[0] - b'0') as f64;
        cp = &cp[1..];
    }

    if cp[0] == b'.' {
        let mut dec = 10.0;
        cp = &cp[1..];
        while (b'0'..=b'9').contains(&cp[0]) {
            val = val + (cp[0] - b'0') as f64 / dec;
            cp = &cp[1..];
            dec *= 10.0;
        }
    }

    *s = cp;

    if neg {
        -val
    } else {
        val
    }
}

pub fn read_rgb_a(cp: &mut &[u8]) -> u32 {
    let mut rgb_value: u32 = 0;
    let mut alpha = 0;
    for i in 0..6 {
        if cp[0].is_ascii_digit() {
            rgb_value = (rgb_value << 4) + (cp[0] - b'0') as u32;
        } else if (b'A'..=b'F').contains(&cp[0]) {
            rgb_value = (rgb_value << 4) + (cp[0] - b'A' + 10) as u32;
        } else if (b'a'..=b'f').contains(&cp[0]) {
            rgb_value = (rgb_value << 4) + (cp[0] - b'a' + 10) as u32;
        } else {
            return 0x000000FF;
        }
        *cp = &cp[1..];
    }
    rgb_value <<= 8;
    let mut broken = false;
    for _ in 0..2 {
        if cp[0].is_ascii_digit() {
            alpha = alpha << 4 + (cp[0] - b'0');
        } else if (b'A'..=b'F').contains(&cp[0]) {
            alpha = (alpha << 4) + (cp[0] - b'A' + 10);
        } else if (b'a'..=b'f').contains(&cp[0]) {
            alpha = (alpha << 4) + (cp[0] - b'a' + 10);
        } else {
            broken = true;
            break;
        }
        *cp = &cp[1..];
    }

    if !broken {
        rgb_value += alpha as u32;
    } else {
        rgb_value += 0xFF;
    }

    rgb_value
}

#[no_mangle]
pub unsafe extern "C" fn read_double(s: *mut *const libc::c_char) -> f64 {
    let mut neg = false;
    let mut val = 0.0;
    let mut cp = (*s).cast::<u8>();

    while *cp == b' ' || *cp == b'\t' {
        cp = cp.add(1);
    }

    if *cp == b'-' {
        neg = true;
        cp = cp.add(1);
    } else if *cp == b'*' {
        cp = cp.add(1);
    }

    while (b'0'..=b'9').contains(&*cp) {
        val = val * 10.0 + (*cp - b'0') as f64;
        cp = cp.add(1);
    }

    if *cp == b'.' {
        let mut dec = 10.0;
        cp = cp.add(1);
        while (b'0'..=b'9').contains(&*cp) {
            val = val + (*cp - b'0') as f64 / dec;
            cp = cp.add(1);
            dec *= 10.0;
        }
    }
    *s = cp.cast();

    if neg {
        -val
    } else {
        val
    }
}

#[no_mangle]
pub unsafe extern "C" fn readFeatureNumber(
    s: *const libc::c_char,
    e: *const libc::c_char,
    f: *mut hb::Tag,
    v: *mut libc::c_int,
) -> bool {
    let len = e as usize - s as usize;
    let mut str = slice::from_raw_parts(s.cast(), len);
    let mut tag = (*f).to_raw();

    if str[0] < b'0' || str[0] > b'9' {
        return false;
    }

    while (b'0'..=b'9').contains(&str[0]) {
        tag = tag * 10 + (str[0] - b'0') as u32;
        str = &str[1..];
    }
    *f = hb::Tag::new(tag);

    while str[0] == b' ' || str[0] == b'\t' {
        str = &str[1..];
    }
    if str[0] != b'=' {
        /* no setting was specified */
        return false;
    }
    str = &str[1..];

    if str[0] < b'0' || str[0] >= b'9' {
        return false;
    }
    while (b'0'..=b'9').contains(&str[0]) {
        *v = *v * 10 + (str[0] - b'0') as i32;
        str = &str[1..];
    }
    while str[0] == b' ' || str[0] == b'\t' {
        str = &str[1..];
    }

    str.is_empty()
}

/// returns 1 to go to next_option, -1 for bad_option, 0 to continue
#[no_mangle]
pub unsafe extern "C" fn readCommonFeatures(
    feat: *const libc::c_char,
    end: *const libc::c_char,
    extend: *mut f32,
    slant: *mut f32,
    embolden: *mut f32,
    letterspace: *mut f32,
    rgb_value: *mut u32,
) -> libc::c_int {
    let len = end as usize - feat as usize;
    let feat = slice::from_raw_parts(feat.cast::<u8>(), len);

    let features: &[(&[_], &dyn Fn(&[u8]) -> libc::c_int)] = &[
        (b"mapping", &|feat| {
            *loaded_font_mapping.get() = load_mapping_file(feat.as_ptr().cast(), end, 0);
            1
        }),
        (b"extend", &|mut feat| {
            *extend = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"slant", &|mut feat| {
            *slant = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"embolden", &|mut feat| {
            *embolden = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"letterspace", &|mut feat| {
            *letterspace = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"color", &|mut feat| {
            let s = feat;
            *rgb_value = read_rgb_a(&mut feat);
            if ptr::addr_eq(feat, &s[6..]) || ptr::addr_eq(feat, &s[8..]) {
                *loaded_font_flags.get() |= FONT_FLAGS_COLORED;
                1
            } else {
                -1
            }
        }),
    ];

    for feature in features {
        if feat.starts_with(feature.0) {
            let feat = &feat[feature.0.len()..];
            return if feat[0] != b'=' {
                -1
            } else {
                (feature.1)(&feat[1..])
            };
        }
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn splitFontName(
    name: *const libc::c_char,
    var: *mut *const libc::c_char,
    feat: *mut *const libc::c_char,
    end: *mut *const libc::c_char,
    index: *mut libc::c_int,
) {
    let mut name = CStr::from_ptr(name).to_bytes();

    *var = ptr::null();
    *feat = ptr::null();
    *index = 0;

    if name[0] == b'[' {
        let mut within_file_name = true;
        name = &name[1..];
        while !name.is_empty() {
            if within_file_name && name[0] == b']' {
                within_file_name = false;
                if (*var).is_null() {
                    *var = name.as_ptr().cast();
                }
                name = &name[1..];
            } else if name[0] == b':' {
                if within_file_name && (*var).is_null() {
                    *var = name.as_ptr().cast();
                    name = &name[1..];
                    while name[0].is_ascii_digit() {
                        *index = *index * 10 + (name[0] - b'0') as libc::c_int;

                        name = &name[1..];
                    }
                } else if !within_file_name && (*feat).is_null() {
                    *feat = name.as_ptr().cast();
                    name = &name[1..];
                } else {
                    name = &name[1..];
                }
            } else {
                name = &name[1..];
            }
        }
        *end = name.as_ptr().cast();
    } else {
        while !name.is_empty() {
            if name[0] == b'/' && (*var).is_null() && (*feat).is_null() {
                *var = name.as_ptr().cast();
            } else if name[0] == b':' && (*feat).is_null() {
                *feat = name.as_ptr().cast();
            }
            name = &name[1..];
        }
        *end = name.as_ptr().cast();
    }

    if (*feat).is_null() {
        *feat = name.as_ptr().cast();
    }
    if (*var).is_null() {
        *var = *feat;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ot_get_font_metrics(
    engine: XeTeXLayoutEngine,
    ascent: *mut scaled_t,
    descent: *mut scaled_t,
    xheight: *mut scaled_t,
    capheight: *mut scaled_t,
    slant: *mut scaled_t,
) {
    let engine = &mut *engine;

    *ascent = d_to_fix(engine.font().ascent() as f64);
    *descent = d_to_fix(engine.font().descent() as f64);

    *slant =
        d_to_fix(fix_to_d(engine.font().slant()) * engine.extend() as f64 + engine.slant() as f64);

    *capheight = d_to_fix(engine.font().cap_height() as f64);
    *xheight = d_to_fix(engine.font().x_height() as f64);

    if *xheight == 0 {
        let glyph_id = engine.font().map_char_to_glyph('x');
        if glyph_id == 0 {
            let (height, _) = engine
                .font_mut()
                .get_glyph_height_depth(glyph_id as GlyphID);
            *xheight = d_to_fix(height as f64);
        } else {
            *xheight = *ascent / 2;
        }
    }

    if *capheight == 0 {
        let glyph_id = engine.font().map_char_to_glyph('X');
        if glyph_id == 0 {
            let (height, _) = engine
                .font_mut()
                .get_glyph_height_depth(glyph_id as GlyphID);
            *capheight = d_to_fix(height as f64);
        } else {
            *capheight = *ascent;
        }
    }
}

#[allow(nonstandard_style)]
extern "C" {
    pub fn load_mapping_file(
        s: *const libc::c_char,
        e: *const libc::c_char,
        byteMapping: libc::c_char,
    ) -> *const libc::c_void;
}
