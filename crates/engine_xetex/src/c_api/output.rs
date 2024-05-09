use crate::c_api::core::{scaled_t, Selector, UTF16Code, BIGGEST_CHAR, BIGGEST_USV};
use crate::c_api::engine::{
    cat_code, dig, doing_special, eqtb_top, error_line, file_line_error_style_p, file_offset,
    full_source_filename_stack, hash, in_open, intpar, line, line_stack, log_file, max_print_line,
    pool_ptr, pool_size, prim, rust_stdout, selector, set_intpar, str_pool, str_ptr, str_start,
    tally, term_offset, trick_buf, trick_count, write_file,
};
use crate::c_api::format::{
    ACTIVE_BASE, EQTB_SIZE, FROZEN_NULL_FONT, HASH_BASE, LETTER, NULL_CS, PRIM_EQTB_BASE,
    SCRIPT_SIZE, SINGLE_BASE, TEXT_SIZE, UNDEFINED_CONTROL_SEQUENCE,
};
use crate::c_api::mfmp::get_tex_str;
use std::cell::Cell;
use std::ffi::CStr;
use std::ptr;
use tectonic_bridge_core::{
    ttbc_diag_append, ttbc_diag_begin_error, ttbc_diag_begin_warning, Diagnostic,
};
use tectonic_io_base::OutputHandle;

thread_local! {
    static CURRENT_DIAGNOSTIC: Cell<*mut Diagnostic> = const { Cell::new(ptr::null_mut()) };
}

pub fn print_str(s: &[u8]) {
    for c in s {
        unsafe { print_char(*c as i32) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn capture_to_diagnostic(diagnostic: *mut Diagnostic) {
    if !CURRENT_DIAGNOSTIC.get().is_null() {
        ttstub_diag_finish(CURRENT_DIAGNOSTIC.get());
    }
    CURRENT_DIAGNOSTIC.set(diagnostic);
}

#[no_mangle]
pub unsafe extern "C" fn diagnostic_print_file_line(diagnostic: *mut Diagnostic) {
    // Add file/line number information
    // This duplicates logic from print_file_line

    let mut level = *in_open as usize;
    while level > 0 && full_source_filename_stack[level] == 0 {
        level -= 1;
    }

    if level == 0 {
        ttbc_diag_append(&mut *diagnostic, c!("!"));
    } else {
        let mut source_line = *line;
        if level != *in_open as usize {
            source_line = line_stack[level + 1];
        }

        let filename = get_tex_str(full_source_filename_stack[level]);
        ttstub_diag_printf(diagnostic, c!("%s:%d: "), filename.as_ptr(), source_line);
    }
}

#[no_mangle]
pub unsafe extern "C" fn diagnostic_begin_capture_warning_here() -> *mut Diagnostic {
    let warning = ttbc_diag_begin_warning();
    diagnostic_print_file_line(warning);
    capture_to_diagnostic(warning);
    warning
}

// This replaces the "print file+line number" block at the start of errors
#[no_mangle]
pub unsafe extern "C" fn error_here_with_diagnostic(
    message: *const libc::c_char,
) -> *mut Diagnostic {
    let error = ttbc_diag_begin_error();
    diagnostic_print_file_line(error);
    ttstub_diag_printf(error, c!("%s"), message);

    if *file_line_error_style_p != 0 {
        print_file_line();
    } else {
        print_nl_cstr(c!("! "));
    }
    print_cstr(message);

    capture_to_diagnostic(error);

    error
}

#[no_mangle]
pub unsafe extern "C" fn warn_char(c: libc::c_int) {
    let diag = CURRENT_DIAGNOSTIC.get();
    if !diag.is_null() {
        let bytes = [c as libc::c_char, 0];
        ttbc_diag_append(&mut *diag, bytes.as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_ln() {
    match *selector {
        Selector::TermAndLog => {
            warn_char('\n' as libc::c_int);
            ttstub_output_putc(*rust_stdout, '\n' as libc::c_int);
            ttstub_output_putc(*log_file, '\n' as libc::c_int);
            *term_offset = 0;
            *file_offset = 0;
        }
        Selector::LogOnly => {
            warn_char('\n' as libc::c_int);
            ttstub_output_putc(*log_file, '\n' as libc::c_int);
            *file_offset = 0;
        }
        Selector::TermOnly => {
            warn_char('\n' as libc::c_int);
            ttstub_output_putc(*rust_stdout, '\n' as libc::c_int);
            *term_offset = 0;
        }
        Selector::NoPrint | Selector::Pseudo | Selector::NewString => (),
        _ => {
            ttstub_output_putc(write_file[*selector as usize], '\n' as libc::c_int);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_raw_char(s: UTF16Code, incr_offset: bool) {
    match *selector {
        Selector::TermAndLog => {
            warn_char(s as libc::c_int);
            ttstub_output_putc(*rust_stdout, s as _);
            ttstub_output_putc(*log_file, s as _);
            if incr_offset {
                *term_offset += 1;
                *file_offset += 1;
            }

            if *term_offset == *max_print_line {
                ttstub_output_putc(*rust_stdout, '\n' as _);
                *term_offset = 0;
            }
            if *file_offset == *max_print_line {
                ttstub_output_putc(*log_file, '\n' as _);
                *file_offset = 0;
            }
        }
        Selector::LogOnly => {
            warn_char(s as libc::c_int);
            ttstub_output_putc(*log_file, s as _);
            if incr_offset {
                *file_offset += 1;
            }
            if *file_offset == *max_print_line {
                ttstub_output_putc(*log_file, '\n' as _);
                *file_offset = 0;
            }
        }
        Selector::TermOnly => {
            warn_char(s as libc::c_int);
            ttstub_output_putc(*rust_stdout, s as _);
            if incr_offset {
                *term_offset += 1;
            }
            if *term_offset == *max_print_line {
                ttstub_output_putc(*rust_stdout, '\n' as _);
                *term_offset = 0;
            }
        }
        Selector::NoPrint => (),
        Selector::Pseudo => {
            if *tally < *trick_count {
                trick_buf[(*tally % *error_line) as usize] = s;
            }
        }
        Selector::NewString => {
            if *pool_ptr < *pool_size {
                str_pool[*pool_ptr as usize] = s;
                *pool_ptr += 1;
            }
        }
        _ => {
            ttstub_output_putc(write_file[*selector as usize], s as _);
        }
    }
    *tally += 1;
}

#[no_mangle]
pub unsafe extern "C" fn print_char(s: i32) {
    if *selector > Selector::Pseudo && !*doing_special {
        if s >= 0x10000 {
            print_raw_char((0xD800 + (s - 0x10000) / 1024) as UTF16Code, true);
            print_raw_char((0xDC00 + (s - 0x10000) % 1024) as UTF16Code, true);
        } else {
            print_raw_char(s as UTF16Code, true);
        }
        return;
    }

    if s == intpar("new_line_char") && *selector < Selector::Pseudo {
        print_ln();
        return;
    }

    if s < 32 && !*doing_special {
        print_raw_char('^' as UTF16Code, true);
        print_raw_char('^' as UTF16Code, true);
        print_raw_char((s + 64) as UTF16Code, true);
    } else if s < 127 {
        print_raw_char(s as UTF16Code, true);
    } else if s == 127 {
        if !*doing_special {
            print_raw_char('^' as UTF16Code, true);
            print_raw_char('^' as UTF16Code, true);
            print_raw_char('?' as UTF16Code, true);
        } else {
            print_raw_char(s as UTF16Code, true);
        }
    } else if s < 160 && !*doing_special {
        print_raw_char('^' as UTF16Code, true);
        print_raw_char('^' as UTF16Code, true);

        let l = (s / 16) as u8;
        if l < 10 {
            print_raw_char((b'0' + l) as UTF16Code, true);
        } else {
            print_raw_char((b'a' + l - 10) as UTF16Code, true);
        }

        let l = (s % 16) as u8;
        if l < 10 {
            print_raw_char((b'0' + l) as UTF16Code, true);
        } else {
            print_raw_char((b'a' + l - 10) as UTF16Code, true);
        }
    } else if *selector == Selector::Pseudo {
        print_raw_char(s as UTF16Code, true);
    } else if s < 2048 {
        print_raw_char((192 + s / 64) as UTF16Code, false);
        print_raw_char((128 + s % 64) as UTF16Code, true);
    } else if s < 0x10000 {
        print_raw_char((224 + s / 4096) as UTF16Code, false);
        print_raw_char((128 + s % 4096 / 64) as UTF16Code, false);
        print_raw_char((128 + s % 64) as UTF16Code, true);
    } else {
        print_raw_char((240 + s / 0x40000) as UTF16Code, false);
        print_raw_char((128 + s % 0x40000 / 4096) as UTF16Code, false);
        print_raw_char((128 + s % 4096 / 64) as UTF16Code, false);
        print_raw_char((128 + s % 64) as UTF16Code, true);
    }
}

#[no_mangle]
pub unsafe extern "C" fn print(s: i32) {
    if s >= *str_ptr {
        print_str(b"???");
        return;
    } else if s <= BIGGEST_CHAR {
        if s < 0 {
            print_str(b"???");
            return;
        } else {
            if *selector > Selector::Pseudo {
                print_char(s);
                return;
            }

            if s == intpar("new_line_char") && *selector < Selector::Pseudo {
                print_ln();
                return;
            }

            let nl = intpar("new_line_char");
            set_intpar("new_line_char", intpar("new_line_char") - 1);
            print_char(s);
            set_intpar("new_line_char", nl);
            return;
        }
    }

    let pool_idx = (s - 0x10000) as usize;

    for i in str_start[pool_idx] as usize..str_start[pool_idx + 1] as usize {
        if str_pool[i] >= 0xD800
            && str_pool[i] < 0xDC00
            && (i + 1) < str_start[pool_idx + 1] as usize
            && str_pool[i + 1] >= 0xDC00
            && str_pool[i + 1] < 0xE000
        {
            print_char(
                0x10000 + str_pool[i] as i32 - 0xD800 * 1024 + str_pool[i + 1] as i32 - 0xDC00,
            );
        } else {
            print_char(str_pool[i] as i32)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_cstr(str: *const libc::c_char) {
    for i in CStr::from_ptr(str).to_bytes() {
        print_char(*i as i32);
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_nl(s: i32) {
    if (*term_offset > 0 && (*selector as usize) % 2 != 0)
        || (*file_offset > 0 && *selector >= Selector::LogOnly)
    {
        print_ln();
    }
    print(s);
}

#[no_mangle]
pub unsafe extern "C" fn print_nl_cstr(str: *const libc::c_char) {
    if (*term_offset > 0 && (*selector as usize) % 2 != 0)
        || (*file_offset > 0 && *selector >= Selector::LogOnly)
    {
        print_ln();
    }
    print_cstr(str);
}

#[no_mangle]
pub unsafe extern "C" fn print_esc(s: i32) {
    let c = intpar("escape_char");

    if c >= 0 && c <= BIGGEST_USV {
        print_char(c);
    }
    print(s);
}

#[no_mangle]
pub unsafe extern "C" fn print_esc_cstr(str: *const libc::c_char) {
    let c = intpar("escape_char");

    if c >= 0 && c <= BIGGEST_USV {
        print_char(c);
    }
    print_cstr(str);
}

#[no_mangle]
pub unsafe extern "C" fn print_the_digs(k: u8) {
    for k in (0..k).rev() {
        if dig[k as usize] < 10 {
            print_char((b'0' + dig[k as usize]) as i32);
        } else {
            print_char((b'A' - 10 + dig[k as usize]) as i32);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_int(mut n: i32) {
    let mut k = 0;

    if n < 0 {
        print_char('-' as i32);
        if n > -100000000 {
            n = -n;
        } else {
            let mut m = -1 - n;
            n = m / 10;
            m = (m % 10) + 1;
            k = 1;
            if m < 10 {
                dig[0] = m as u8;
            } else {
                dig[0] = 0;
                n += 1;
            }
        }
    }

    loop {
        dig[k as usize] = (n % 10) as u8;
        n = n / 10;
        k += 1;
        if n == 0 {
            break;
        }
    }

    print_the_digs(k);
}

#[no_mangle]
pub unsafe extern "C" fn print_cs(p: i32) {
    let p = p as usize;
    if p < HASH_BASE {
        if p >= SINGLE_BASE {
            if p == NULL_CS {
                print_esc_cstr(c!("csname"));
                print_esc_cstr(c!("endcsname"));
                print_char(' ' as i32);
            } else {
                print_esc((p - SINGLE_BASE) as i32);
                if cat_code(p - SINGLE_BASE) == LETTER {
                    print_char(' ' as i32);
                }
            }
        } else if p < ACTIVE_BASE {
            print_esc_cstr(c!("IMPOSSIBLE."));
        } else {
            print_char((p - 1) as i32);
        }
    } else if p >= UNDEFINED_CONTROL_SEQUENCE && p <= EQTB_SIZE || p > *eqtb_top as usize {
        print_esc_cstr(c!("IMPOSSIBLE."));
    } else if hash[p].s1 >= *str_ptr {
        print_esc_cstr(c!("NONEXISTENT."));
    } else {
        if p >= PRIM_EQTB_BASE && p < FROZEN_NULL_FONT {
            print_esc(prim[p - PRIM_EQTB_BASE].s1 - 1);
        } else {
            print_esc(hash[p].s1);
        }
        print_char(' ' as i32);
    }
}

#[no_mangle]
pub unsafe extern "C" fn sprint_cs(p: i32) {
    let p = p as usize;
    if p < HASH_BASE {
        if p < SINGLE_BASE {
            print_char((p - 1) as i32);
        } else if p < NULL_CS {
            print_esc((p - SINGLE_BASE) as i32);
        } else {
            print_esc_cstr(c!("csname"));
            print_esc_cstr(c!("endcsname"));
        }
    } else if p >= PRIM_EQTB_BASE && p < FROZEN_NULL_FONT {
        print_esc(prim[p - PRIM_EQTB_BASE].s1 - 1);
    } else {
        print_esc(hash[p].s1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_file_name(n: i32, a: i32, e: i32) {
    let a = a as usize;
    let n = n as usize;
    let e = e as usize;
    let mut must_quote = false;
    let mut quote_char = 0;

    for val in [a, n, e] {
        if val != 0 {
            let mut j = str_start[val - 0x10000] as usize;
            while (!must_quote || quote_char == 0) && (j < str_start[val + 1 - 0x10000] as usize) {
                if str_pool[j] == ' ' as u16 {
                    must_quote = true;
                } else if str_pool[j] == '"' as u16 || str_pool[j] == '\'' as u16 {
                    must_quote = true;
                    quote_char = '"' as u16 - str_pool[j];
                }
                j += 1;
            }
        }
    }

    if must_quote {
        if quote_char == 0 {
            quote_char = '"' as u16;
        }
        print_char(quote_char as i32);
    }

    for val in [a, n, e] {
        if val != 0 {
            let j = str_start[val - 0x10000] as usize;
            let for_end = (str_start[val + 1 - 0x10000] - 1) as usize;
            for j in j..=for_end {
                if str_pool[j] == quote_char {
                    print(quote_char as i32);
                    quote_char = '"' as u16 - quote_char;
                    print(quote_char as i32);
                }
                print(str_pool[j] as i32);
            }
        }
    }

    if quote_char != 0 {
        print_char(quote_char as i32);
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_size(s: i32) {
    if s == TEXT_SIZE {
        print_esc_cstr(c!("textfont"));
    } else if s == SCRIPT_SIZE {
        print_esc_cstr(c!("scriptfont"));
    } else {
        print_esc_cstr(c!("scriptscriptfont"));
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_write_whatsit(s: *const libc::c_char, p: i32) {
    print_esc_cstr(s);

    if mem[(p + 1) as usize].b32.s0 < 16 {
        print_int(mem[(p + 1)])
    }
}

/*
void
print_write_whatsit(const char* s, int32_t p)
{

    print_esc_cstr(s);

    if (mem[p + 1].b32.s0 < 16)
        print_int(mem[p + 1].b32.s0);
    else if (mem[p + 1].b32.s0 == 16)
        print_char('*');
    else
        print_char('-');
}
 */

/// cbindgen:ignore
extern "C" {
    fn ttstub_diag_finish(diag: *mut Diagnostic);
    fn ttstub_diag_printf(diag: *mut Diagnostic, format: *const libc::c_char, ...);
    fn ttstub_output_putc(output: *mut OutputHandle, c: libc::c_int) -> libc::c_int;

    pub fn print_scaled(s: scaled_t);
    pub fn print_file_line();
}
