/*
int
linebreak_next(void)
{
    if (brkIter != NULL)
        return ubrk_next((UBreakIterator*)brkIter);
    else
        return findNextGraphiteBreak();
}
 */

use crate::c_api::core::{scaled_t, FONT_FLAGS_COLORED};
use crate::c_api::engine::{loaded_font_flags, loaded_font_mapping, memory_word};
use std::cell::UnsafeCell;
use std::ffi::CStr;
use std::{ptr, slice};
use tectonic_bridge_harfbuzz as hb;
use tectonic_bridge_icu::{ubrk_next, UBreakIterator};
use tectonic_xetex_layout::c_api::engine::findNextGraphiteBreak;
use tectonic_xetex_layout::c_api::{Fixed, GlyphID, XeTeXLayoutEngine};

pub const NATIVE_INFO_OFFSET: usize = 4;

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
pub unsafe extern "C" fn linebreak_next() -> libc::c_int {
    if !(*brkIter.get()).is_null() {
        ubrk_next(*brkIter.get())
    } else {
        findNextGraphiteBreak()
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

    static brkIter: UnsafeCell<*mut UBreakIterator>;
    static brkLocaleStrNum: UnsafeCell<libc::c_int>;
}
