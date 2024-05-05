use crate::c_api::core::{
    scaled_t, UTF16Code, AUTO, FONT_FLAGS_COLORED, FONT_FLAGS_VERTICAL, ICUMAPPING, RAW,
    US_NATIVE_UTF16, UTF16BE, UTF16LE, UTF16_NATIVE, UTF8,
};
use crate::c_api::engine::{
    begin_diagnostic, end_diagnostic, file_name, font_area, font_feature_warning,
    font_layout_engine, font_mapping_warning, get_tracing_fonts_state, loaded_font_flags,
    loaded_font_letter_space, loaded_font_mapping, memory_word, name_of_file,
    native_font_type_flag, print_char, print_int, print_nl, print_raw_char, print_str,
};
use crate::c_api::mfmp::{get_tex_str, maketexstring};
use crate::teckit::{
    kForm_Bytes, TECkit_ConvertBuffer, TECkit_CreateConverter, TECkit_ResetConverter, UniChar,
};
use memchr::memmem;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString};
use std::{mem, ptr, slice};
use tectonic_bridge_core::FileFormat;
use tectonic_bridge_harfbuzz as hb;
use tectonic_bridge_icu as icu;
use tectonic_io_base::InputHandle;
use tectonic_xetex_layout::engine::LayoutEngine;
use tectonic_xetex_layout::manager::{Engine, FontManager};
use tectonic_xetex_layout::{Fixed, GlyphID, RawPlatformFontRef, XeTeXFont, XeTeXLayoutEngine};

pub const NATIVE_INFO_OFFSET: usize = 4;
pub const OTGR_FONT_FLAG: u32 = 0xFFFE;

thread_local! {
    static BRK_ITER: RefCell<Option<icu::BreakIterator>> = const { RefCell::new(None) };
    static BRK_LOCALE_STR_NUM: Cell<i32> = const { Cell::new(0) };
    static SAVED_MAPPING_NAME: Cell<*mut libc::c_char> = const { Cell::new(ptr::null_mut()) };
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
pub unsafe extern "C" fn print_utf8_str(str: *const u8, len: libc::c_int) {
    for i in 0..len as usize {
        /* bypass utf-8 encoding done in print_char() */
        print_raw_char(*str.add(i) as UTF16Code, true)
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_chars(str: *const libc::c_ushort, len: libc::c_int) {
    for i in 0..len as usize {
        /* bypass utf-8 encoding done in print_char() */
        print_char(*str.add(i) as libc::c_int)
    }
}

#[no_mangle]
pub unsafe extern "C" fn check_for_tfm_font_mapping() {
    let ptr = *name_of_file.get();
    let len = CStr::from_ptr(ptr).to_bytes().len();
    let cp = slice::from_raw_parts_mut(ptr.cast::<u8>(), len);
    if let Some(mut pos) = memmem::find(cp, b":mapping=") {
        cp[pos] = 0;
        pos += 9;
        while pos < cp.len() && cp[pos] <= b' ' {
            pos += 1;
        }
        if pos < cp.len() {
            SAVED_MAPPING_NAME.set(libc::strdup(cp.as_ptr().cast()))
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn linebreak_start(
    f: libc::c_int,
    locale_str_num: i32,
    text: *mut u16,
    text_len: i32,
) {
    let locale = get_tex_str(locale_str_num);
    let text = slice::from_raw_parts(text, text_len as usize);

    if *(*font_area.get()).add(f as usize) as u32 == OTGR_FONT_FLAG && locale.to_bytes() == b"G" {
        let engine = (*font_layout_engine.get())
            .add(f as usize)
            .cast::<LayoutEngine>();
        if (*engine).init_graphite_break(text) {
            return;
        }
    }

    if locale_str_num != BRK_LOCALE_STR_NUM.get() && BRK_ITER.with_borrow(|b| b.is_some()) {
        BRK_ITER.with_borrow_mut(|b| *b = None);
    }

    if BRK_ITER.with_borrow(|b| b.is_none()) {
        match icu::BreakIterator::new(&locale) {
            Ok(bi) => BRK_ITER.with_borrow_mut(|b| *b = Some(bi)),
            Err(err) => {
                begin_diagnostic();
                print_nl(b'E' as i32);
                print_str(b"rror ");
                print_int(err.into_raw());
                print_str(b" creating linebreak iterator for locale `");
                print_str(locale.to_bytes());
                print_str(b"'; trying default locale `en_us'.");
                end_diagnostic(true);
                match icu::BreakIterator::new(CStr::from_bytes_with_nul(b"en_us\0").unwrap()) {
                    Ok(bi) => BRK_ITER.with_borrow_mut(|b| *b = Some(bi)),
                    Err(err) => panic!(
                        "failed to create linebreak iterator, status={}",
                        err.into_raw()
                    ),
                }
            }
        }
        BRK_LOCALE_STR_NUM.set(locale_str_num);
    }

    let _ = BRK_ITER.with_borrow_mut(|b| b.as_mut().unwrap().set_text(text));
}

#[no_mangle]
pub unsafe extern "C" fn linebreak_next(f: libc::c_int) -> libc::c_int {
    let engine = &mut *(*font_layout_engine.get())
        .add(f as usize)
        .cast::<LayoutEngine>();
    BRK_ITER.with_borrow_mut(|b| {
        if let Some(iter) = b {
            iter.next()
        } else {
            engine.find_next_graphite_break() as libc::c_int
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn get_encoding_mode_and_info(info: *mut i32) -> libc::c_int {
    /* \XeTeXinputencoding "enc-name"
     *   -> name is packed in |nameoffile| as a C string, starting at [1]
     * Check if it's a built-in name; if not, try to open an ICU converter by that name
     */
    *info = 0;
    let file_name = file_name();
    let file_name_b = file_name.to_bytes();
    if file_name_b.eq_ignore_ascii_case(b"auto") {
        AUTO
    } else if file_name_b.eq_ignore_ascii_case(b"utf8") {
        UTF8
    } else if file_name_b.eq_ignore_ascii_case(b"utf16") {
        US_NATIVE_UTF16
    } else if file_name_b.eq_ignore_ascii_case(b"utf16be") {
        UTF16BE
    } else if file_name_b.eq_ignore_ascii_case(b"utf16le") {
        UTF16LE
    } else if file_name_b.eq_ignore_ascii_case(b"bytes") {
        RAW
    } else {
        let cnv = icu::Converter::new(file_name);
        if cnv.is_err() {
            begin_diagnostic();
            print_nl(b'U' as i32); /* ensure message starts on a new line */
            print_str(b"nknown encoding `");
            print_str(file_name_b);
            print_str(b"'; reading as raw bytes");
            end_diagnostic(true);
            RAW
        } else {
            *info = maketexstring(file_name_b.as_ptr().cast());
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

    while cp[0].is_ascii_digit() {
        val = val * 10.0 + (cp[0] - b'0') as f64;
        cp = &cp[1..];
    }

    if cp[0] == b'.' {
        let mut dec = 10.0;
        cp = &cp[1..];
        while cp[0].is_ascii_digit() {
            val += (cp[0] - b'0') as f64 / dec;
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
    for _ in 0..6 {
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
            alpha = (alpha << 4) + (cp[0] - b'0');
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

    while (*cp).is_ascii_digit() {
        val = val * 10.0 + (*cp - b'0') as f64;
        cp = cp.add(1);
    }

    if *cp == b'.' {
        let mut dec = 10.0;
        cp = cp.add(1);
        while (*cp).is_ascii_digit() {
            val += (*cp - b'0') as f64 / dec;
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

pub fn read_feature_number(mut str: &[u8], f: &mut hb::Tag, v: &mut u32) -> bool {
    let mut tag = (*f).to_raw();

    if str[0] < b'0' || str[0] > b'9' {
        return false;
    }

    while str[0].is_ascii_digit() {
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
    while str[0].is_ascii_digit() {
        *v = *v * 10 + (str[0] - b'0') as u32;
        str = &str[1..];
    }
    while str[0] == b' ' || str[0] == b'\t' {
        str = &str[1..];
    }

    str.is_empty()
}

pub unsafe fn read_common_features(
    feat: &[u8],
    extend: &mut f32,
    slant: &mut f32,
    embolden: &mut f32,
    letterspace: &mut f32,
    rgb_value: &mut u32,
) -> i8 {
    let features: &mut [(_, &mut dyn FnMut(_) -> i8)] = &mut [
        (b"mapping" as &[_], &mut |feat: &[u8]| {
            *loaded_font_mapping.get() = load_mapping_file(feat, 0)
                .cast::<libc::c_void>()
                .cast_const();
            1
        }),
        (b"extend", &mut |mut feat| {
            *extend = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"slant", &mut |mut feat| {
            *slant = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"embolden", &mut |mut feat| {
            *embolden = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"letterspace", &mut |mut feat| {
            *letterspace = rs_read_double(&mut feat) as f32;
            1
        }),
        (b"color", &mut |mut feat| {
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

    read_common_features(
        feat,
        &mut *extend,
        &mut *slant,
        &mut *embolden,
        &mut *letterspace,
        &mut *rgb_value,
    ) as libc::c_int
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
    engine: *mut libc::c_void,
    ascent: *mut scaled_t,
    descent: *mut scaled_t,
    xheight: *mut scaled_t,
    capheight: *mut scaled_t,
    slant: *mut scaled_t,
) {
    let engine = &mut *engine.cast::<LayoutEngine>();

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

pub fn read_tag_with_param(mut cp: &[u8], param: &mut i32) -> hb::Tag {
    let mut tag_end = 0;
    while tag_end < cp.len() && ![b':', b';', b',', b'='].contains(&cp[tag_end]) {
        tag_end += 1;
    }

    let tag = hb::Tag::from_bytes(&cp[..tag_end]);

    cp = &cp[tag_end..];

    if cp[0] == b'=' {
        let mut neg = false;
        cp = &cp[1..];
        if cp[0] == b'-' {
            neg = true;
            cp = &cp[1..];
        }

        while cp[0].is_ascii_digit() {
            *param = *param * 10 + (cp[0] - b'0') as i32;
            cp = &cp[1..];
        }
        if neg {
            *param = -*param;
        }
    }

    tag
}

#[no_mangle]
pub unsafe extern "C" fn loadOTfont(
    _: RawPlatformFontRef,
    font: XeTeXFont,
    scaled_size: Fixed,
    cp1: *const libc::c_char,
) -> XeTeXLayoutEngine {
    let cp1 = if cp1.is_null() {
        None
    } else {
        Some(CStr::from_ptr(cp1).to_bytes())
    };

    let mut font = Box::from_raw(font);
    let mut shapers = Vec::new();
    let mut rgb_value = 0x000000FFu32;
    let mut extend = 1.0;
    let mut slant = 0.0;
    let mut embolden = 0.0;
    let mut letterspace = 0.0;

    let req_engine = FontManager::with_font_manager(|m| m.get_req_engine());

    let engine = match req_engine {
        Engine::OpenType => {
            shapers.push(c"ot".as_ptr());
            None
        }
        Engine::Graphite => {
            shapers.push(c"graphite2".as_ptr());
            Some(LayoutEngine::new(
                &mut *font,
                hb::Tag::new(0),
                None,
                Box::new([]),
                vec![c"graphite2".as_ptr(), ptr::null_mut()],
                rgb_value,
                extend,
                slant,
                embolden,
            ))
        }
        Engine::Default | Engine::Apple => None,
    };

    let mut script = hb::Tag::new(0);
    let mut language = None;
    let mut features = Vec::new();
    /* scan the feature string (if any) */
    if let Some(mut cp1) = cp1 {
        while !cp1.is_empty() {
            if cp1[0] == b':' || cp1[0] == b';' || cp1[0] == b',' {
                cp1 = &cp1[1..];
            }
            while cp1[0] == b' ' || cp1[0] == b'\t' {
                cp1 = &cp1[1..];
            }
            if cp1.is_empty() {
                break;
            }
            let mut opt_end = 0;
            while opt_end < cp1.len()
                && cp1[opt_end] != b':'
                && cp1[opt_end] != b';'
                && cp1[opt_end] != b','
            {
                opt_end += 1;
            }

            let feature_handlers: &mut [(&[_], &mut dyn FnMut(&[u8]))] = &mut [
                (b"script", &mut |val| script = hb::Tag::from_bytes(val)),
                (b"language", &mut |val| {
                    language = Some(Cow::Owned(CString::new(val).unwrap()))
                }),
                (b"shaper", &mut |val| {
                    shapers.push(CString::new(val).unwrap().into_raw())
                }),
            ];

            for (feat, f) in feature_handlers {
                if cp1.starts_with(feat) {
                    if cp1[feat.len()] != b'=' {
                        font_feature_warning(cp1.as_ptr().cast(), opt_end as i32, ptr::null(), 0);
                        cp1 = &cp1[opt_end..];
                        continue;
                    }
                    f(&cp1[feat.len() + 1..opt_end]);
                    cp1 = &cp1[opt_end..];
                    continue;
                }
            }

            let i = read_common_features(
                &cp1[..opt_end],
                &mut extend,
                &mut slant,
                &mut embolden,
                &mut letterspace,
                &mut rgb_value,
            );
            if i == 1 {
                cp1 = &cp1[opt_end..];
                continue;
            } else if i == -1 {
                font_feature_warning(cp1.as_ptr().cast(), opt_end as i32, ptr::null(), 0);
                cp1 = &cp1[opt_end..];
                continue;
            }

            if let Engine::Graphite = req_engine {
                let mut tag = hb::Tag::new(0);
                let mut value = 0;
                if read_feature_number(&cp1[..opt_end], &mut tag, &mut value)
                    || engine.as_ref().unwrap().find_graphite_feature(
                        &cp1[..opt_end],
                        &mut tag,
                        &mut value,
                    )
                {
                    features.push(hb::Feature {
                        tag,
                        value,
                        start: 0,
                        end: u32::MAX,
                    });
                    cp1 = &cp1[opt_end..];
                    continue;
                }
            }

            if cp1[0] == b'+' {
                let mut param = 0;
                let tag = read_tag_with_param(&cp1[1..], &mut param);
                if param >= 0 {
                    param += 1;
                }
                features.push(hb::Feature {
                    tag,
                    value: param as u32,
                    start: 0,
                    end: u32::MAX,
                });
                cp1 = &cp1[opt_end..];
                continue;
            }

            if cp1[1] == b'-' {
                let tag = hb::Tag::from_bytes(&cp1[..opt_end]);
                features.push(hb::Feature {
                    tag,
                    value: 0,
                    start: 0,
                    end: u32::MAX,
                });
                cp1 = &cp1[opt_end..];
                continue;
            }

            if cp1.starts_with(b"vertical") {
                let mut temp_end = opt_end;
                if [b';', b':', b','].contains(&cp1[temp_end]) {
                    temp_end -= 1;
                }
                while [b'\0', b' ', b'\t'].contains(&cp1[temp_end]) {
                    temp_end -= 1;
                }
                if cp1[temp_end] != b'\0' {
                    temp_end += 1;
                }
                if temp_end == 8 {
                    *loaded_font_flags.get() |= FONT_FLAGS_VERTICAL;
                    cp1 = &cp1[opt_end..];
                    continue;
                }
            }

            font_feature_warning(cp1.as_ptr().cast(), opt_end as i32, ptr::null(), 0);
            cp1 = &cp1[opt_end..];
        }
    }

    drop(engine);

    if !shapers.is_empty() {
        shapers.push(ptr::null());
    }

    if embolden != 0.0 {
        embolden *= (fix_to_d(scaled_size) / 100.0) as f32;
    }

    if letterspace != 0.0 {
        *loaded_font_letter_space.get() = (letterspace / 100.0 * scaled_size as f32) as scaled_t;
    }

    if *loaded_font_flags.get() & FONT_FLAGS_COLORED == 0 {
        rgb_value = 0x000000FF;
    }

    if *loaded_font_flags.get() & FONT_FLAGS_VERTICAL != 0 {
        font.set_layout_dir_vertical(true);
    }

    let engine = LayoutEngine::new(
        font,
        script,
        language,
        features.into_boxed_slice(),
        shapers,
        rgb_value,
        extend,
        slant,
        embolden,
    );

    *native_font_type_flag.get() = OTGR_FONT_FLAG as i32;
    Box::into_raw(Box::new(engine))
}

#[no_mangle]
pub unsafe fn load_mapping_file(str: &[u8], byte_mapping: u8) -> *mut () {
    let mut cnv = ptr::null_mut();
    let mut buffer = str.to_vec();
    buffer.extend(b".tec");
    let buffer = CString::new(buffer).unwrap();

    let map = ttstub_input_open(buffer.as_ptr(), FileFormat::MiscFonts, 0);
    if !map.is_null() {
        let mapping_size = ttstub_input_get_size(map);
        let mut mapping = vec![0u8; mapping_size];
        let r = ttstub_input_read(map, mapping.as_mut_ptr(), mapping_size);
        if r < 0 || r as usize != mapping_size {
            panic!("could not read mapping file \"{:?}\"", buffer);
        }

        ttstub_input_close(map);

        if byte_mapping != 0 {
            TECkit_CreateConverter(
                mapping.as_mut_ptr(),
                mapping_size as u32,
                false as u8,
                UTF16_NATIVE,
                kForm_Bytes,
                &mut cnv,
            );
        } else {
            TECkit_CreateConverter(
                mapping.as_mut_ptr(),
                mapping_size as u32,
                true as u8,
                UTF16_NATIVE,
                UTF16_NATIVE,
                &mut cnv,
            );
        }

        if cnv.is_null() {
            font_mapping_warning(buffer.as_ptr().cast(), buffer.to_bytes().len() as i32, 2);
        /* not loadable */
        } else if get_tracing_fonts_state() > 1 {
            font_mapping_warning(buffer.as_ptr().cast(), buffer.to_bytes().len() as i32, 0);
            /* tracing */
        }
    } else {
        font_mapping_warning(buffer.as_ptr().cast(), buffer.to_bytes().len() as i32, 1);
        /* not found */
    }

    cnv.cast()
}

#[no_mangle]
pub unsafe extern "C" fn load_tfm_font_mapping() -> *mut libc::c_void {
    if !SAVED_MAPPING_NAME.get().is_null() {
        let out = load_mapping_file(CStr::from_ptr(SAVED_MAPPING_NAME.get()).to_bytes(), 1);
        libc::free(SAVED_MAPPING_NAME.get().cast());
        SAVED_MAPPING_NAME.set(ptr::null_mut());
        out.cast()
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn apply_tfm_font_mapping(
    cnv: *mut libc::c_void,
    c: libc::c_int,
) -> libc::c_int {
    let input: UniChar = c as UniChar;
    let mut output = [0u8; 2];
    let mut in_used = 0;
    let mut out_used = 0;
    TECkit_ConvertBuffer(
        cnv.cast(),
        ptr::from_ref(&input).cast(),
        mem::size_of::<UniChar>() as u32,
        &mut in_used,
        output.as_mut_ptr(),
        2,
        &mut out_used,
        1,
    );
    TECkit_ResetConverter(cnv.cast());
    if out_used < 1 {
        0
    } else {
        output[0] as libc::c_int
    }
}

/// cbindgen:ignore
#[allow(nonstandard_style, improper_ctypes)]
extern "C" {
    pub fn ttstub_input_open(
        path: *const libc::c_char,
        format: FileFormat,
        is_gz: libc::c_int,
    ) -> *mut InputHandle;
    pub fn ttstub_input_get_size(inp: *mut InputHandle) -> usize;
    pub fn ttstub_input_read(inp: *mut InputHandle, arr: *mut u8, len: usize) -> isize;
    pub fn ttstub_input_close(inp: *mut InputHandle) -> libc::c_int;
}
