#![allow(non_upper_case_globals)]

use std::cell::UnsafeCell;
use std::ffi::CStr;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr;

pub const INT_BASE: usize = 0x776d29;

pub const INT_PAR__new_line_char: usize = 0x31;
pub const INT_PAR__escape_char: usize = 0x2d;

#[cfg(target_endian = "big")]
mod defs {
    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b32x2 {
        pub s1: i32,
        pub s0: i32,
    }

    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b16x4 {
        pub s3: u16,
        pub s2: u16,
        pub s1: u16,
        pub s0: u16,
    }
}

#[cfg(not(target_endian = "big"))]
mod defs {
    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b32x2 {
        pub s0: i32,
        pub s1: i32,
    }

    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    pub struct b16x4 {
        pub s0: u16,
        pub s1: u16,
        pub s2: u16,
        pub s3: u16,
    }
}

use crate::c_api::core::{scaled_t, Selector, UTF16Code, NATIVE_NODE_SIZE};
use crate::c_api::format::{CAT_CODE_BASE, PRIM_SIZE};
pub use defs::*;
use tectonic_io_base::OutputHandle;

/* ## THE ORIGINAL SITUATION (archived for posterity)
 *
 * In XeTeX, a "quarterword" is 16 bits. Who knows why. A "halfword" is,
 * sensibly, 32 bits. A "memory word" is a full word: either four quarters or
 * two halves: i.e., 64 bits. The memory word union also has options for
 * doubles (called `gr`), `integer` which is an int32_t (called `cint`), and a
 * pointer (`ptr`).
 *
 * Original struct definition, LITTLE ENDIAN (condensed):
 *
 *   typedef union {
 *       struct { int32_t LH, RH; } v;
 *       struct { short B1, B0; } u;
 *   } two_halves;
 *
 *   typedef struct {
 *       struct { uint16_t B3, B2, B1, B0; } u;
 *   } four_quarters;
 *
 *   typedef union {
 *       two_halves hh;
 *
 *       struct {
 *           int32_t junk;
 *           int32_t CINT;
 *       } u;
 *
 *       struct {
 *           four_quarters QQQQ;
 *       } v;
 *   } memory_word;
 *
 *   #  define cint u.CINT
 *   #  define qqqq v.QQQQ
 *
 * Original memory layout, LITTLE ENDIAN:
 *
 *   bytes:    --0-- --1-- --2-- --3-- --4-- --5-- --6-- --7--
 *   cint:                             [lsb...............msb]
 *   hh.u:     [l..B1...m] [l..B0...m]
 *   hh.v:     [lsb......LH.......msb] [lsb......RH.......msb]
 *   quarters: [l..B3...m] [l..B2...m] [l..B1...m] [l..B0...m]
 *
 * Original struct definition, BIG ENDIAN (condensed):
 *
 *   typedef union {
 *       struct { int32_t RH, LH; } v;
 *       struct {
 *           int32_t junk;
 *           short B0, B1;
 *       } u;
 *   } two_halves;
 *
 *   typedef struct {
 *       struct { uint16_t B0, B1, B2, B3; } u;
 *   } four_quarters;
 *
 *   typedef union {
 *       two_halves hh;
 *       four_quarters qqqq;
 *   } memory_word;
 *
 * Original memory layout, BIG ENDIAN:
 *
 *   bytes:    --0-- --1-- --2-- --3-- --4-- --5-- --6-- --7--
 *   cint:     [msb...............lsb]
 *   hh.u:                             [m..B0...l] [m..B1...l]
 *   hh.v:     [msb......RH.......lsb] [msb......LH.......lsb]
 *   quarters: [m..B0...l] [m..B1...l] [m..B2...l] [m...B3..l]
 *
 * Several things to note that apply to both endiannesses:
 *
 *   1. The different B0 and B1 instances do not line up.
 *   2. `cint` is isomorphic to `hh.v.RH`
 *   3. `hh.u.B0` is isomorphic to `qqqq.u.B2`
 *   4. `hh.u.B1` is isomorphic to `qqqq.u.B3`.
 *   5. The `four_quarters` field `u` serves no discernable purpose.
 *
 * CONVERTING TO THE NEW SYSTEM
 *
 * - `w.cint` => `w.b32.s1`
 * - `w.qqqq.u.B<n>` => `w.b16.s{{3 - <n>}}` !!!!!!!!!!!
 * - similar for `<quarterword_variable>.u.B<n>` => `<quarterword_variable>.s{{3 - <n>}}` !!!
 * - `w.hh.u.B0` => `w.b16.s1`
 * - `w.hh.u.B1` => `w.b16.s0`
 * - `w.hh.v.RH` => `w.b32.s1`
 * - `w.hh.v.LH` => `w.b32.s0`
 * - `four_quarters` => `b16x4`
 * - `two_halves` => `b32x2`
 *
 */

/* The annoying `memory_word` type. We have to make sure the byte-swapping
 * that the (un)dumping routines do suffices to put things in the right place
 * in memory.
 *
 * This set of data used to be a huge mess (see comment after the
 * definitions). It is now (IMO) a lot more reasonable, but there will no
 * doubt be carryover weird terminology around the code.
 *
 * ## ENDIANNESS (cheat sheet because I'm lame)
 *
 * Intel is little-endian. Say that we have a 32-bit integer stored in memory
 * with `p` being a `uint8` pointer to its location. In little-endian land,
 * `p[0]` is least significant byte and `p[3]` is its most significant byte.
 *
 * Conversely, in big-endian land, `p[0]` is its most significant byte and
 * `p[3]` is its least significant byte.
 *
 * ## MEMORY_WORD LAYOUT
 *
 * Little endian:
 *
 *   bytes: --0-- --1-- --2-- --3-- --4-- --5-- --6-- --7--
 *   b32:   [lsb......s0.......msb] [lsb......s1.......msb]
 *   b16:   [l..s0...m] [l..s1...m] [l..s2...m] [l..s3...m]
 *
 * Big endian:
 *
 *   bytes: --0-- --1-- --2-- --3-- --4-- --5-- --6-- --7--
 *   b32:   [msb......s1.......lsb] [msb......s0.......lsb]
 *   b16:   [m..s3...l] [m..s2...l] [m..s1...l] [m...s0..l]
 *
 */
#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: f64,
    pub ptr: *mut (),
}

pub unsafe fn file_name() -> &'static CStr {
    let ptr = *name_of_file;
    CStr::from_ptr(ptr)
}

fn intpar_offset(name: &str) -> usize {
    match name {
        "new_line_char" => INT_PAR__new_line_char,
        "escape_char" => INT_PAR__escape_char,
        _ => unreachable!(),
    }
}

pub unsafe fn intpar(name: &str) -> i32 {
    let offset = intpar_offset(name);
    eqtb[INT_BASE + offset].b32.s1
}

pub unsafe fn set_intpar(name: &str, val: i32) {
    let offset = intpar_offset(name);
    eqtb[INT_BASE + offset].b32.s1 = val;
}

pub unsafe fn cat_code(n: usize) -> i32 {
    eqtb[CAT_CODE_BASE + n].b32.s1
}

/* Symbolic accessors for various TeX data structures. I would loooove to turn these
 * into actual structs, but the path to doing that is not currently clear. Making
 * field references symbolic seems like a decent start. Sadly I don't see how to do
 * this conversion besides painstakingly annotating things.
 */

pub unsafe fn llist_link(p: usize) -> i32 {
    mem[p].b32.s1
}

pub unsafe fn llist_info(p: usize) -> i32 {
    mem[p].b32.s0
}

pub unsafe fn native_node_text<'a>(p: usize) -> CArr<u16> {
    let ptr = ptr::from_mut(&mut mem[p + NATIVE_NODE_SIZE]).cast();
    CArr(ptr)
}

#[repr(transparent)]
pub struct CArr<T>(*mut T);

impl<T> Index<usize> for CArr<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { self.0.add(index).as_ref() }.unwrap()
    }
}

impl<T> IndexMut<usize> for CArr<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { self.0.add(index).as_mut() }.unwrap()
    }
}

#[repr(transparent)]
pub struct DangerCell<T>(UnsafeCell<T>);

impl<T> Deref for DangerCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.get().as_ref() }.unwrap()
    }
}

impl<T> DerefMut for DangerCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.get().as_mut() }.unwrap()
    }
}

/// cbindgen:ignore
#[allow(nonstandard_style)]
extern "C" {
    pub fn font_mapping_warning(
        mappingNameP: *const libc::c_void,
        mappingNameLen: i32,
        warningType: i32,
    );
    pub fn begin_diagnostic();
    pub fn end_diagnostic(blank_line: bool);
    pub fn font_feature_warning(
        featureNameP: *const libc::c_void,
        featLen: i32,
        settingNameP: *const libc::c_void,
        setLen: i32,
    );
    pub fn get_tracing_fonts_state() -> i32;
    pub fn print_raw_char(s: UTF16Code, incr_offset: bool);

    pub static mut loaded_font_flags: DangerCell<libc::c_char>;
    pub static mut loaded_font_mapping: DangerCell<*const libc::c_void>;
    pub static mut loaded_font_letter_space: DangerCell<scaled_t>;
    pub static font_area: DangerCell<CArr<i32>>;
    pub static font_layout_engine: DangerCell<CArr<*mut ()>>;
    pub static mut native_font_type_flag: DangerCell<i32>;
    pub static name_of_file: DangerCell<*mut libc::c_char>;
    pub static mut arith_error: DangerCell<bool>;
    pub static mut tex_remainder: DangerCell<scaled_t>;
    pub static mut help_ptr: DangerCell<libc::c_char>;
    pub static mut help_line: DangerCell<[*const libc::c_char; 6]>;
    pub static in_open: DangerCell<i32>;
    pub static line: DangerCell<i32>;
    pub static full_source_filename_stack: DangerCell<CArr<i32>>;
    pub static line_stack: DangerCell<CArr<i32>>;
    pub static file_line_error_style_p: DangerCell<libc::c_int>;
    pub static selector: DangerCell<Selector>;
    pub static rust_stdout: DangerCell<*mut OutputHandle>;
    pub static log_file: DangerCell<*mut OutputHandle>;
    pub static mut term_offset: DangerCell<i32>;
    pub static mut file_offset: DangerCell<i32>;
    pub static write_file: DangerCell<[*mut OutputHandle; 16]>;
    pub static mut tally: DangerCell<i32>;
    pub static max_print_line: DangerCell<i32>;
    pub static trick_count: DangerCell<i32>;
    pub static mut trick_buf: DangerCell<[UTF16Code; 256]>;
    pub static error_line: DangerCell<i32>;
    pub static mut pool_ptr: DangerCell<i32>;
    pub static pool_size: DangerCell<i32>;
    pub static mut str_pool: DangerCell<CArr<libc::c_ushort>>;
    pub static str_start: DangerCell<CArr<i32>>;
    pub static str_ptr: DangerCell<i32>;
    pub static doing_special: DangerCell<bool>;
    pub static mut eqtb: DangerCell<CArr<memory_word>>;
    pub static mut dig: DangerCell<[u8; 23]>;
    pub static eqtb_top: DangerCell<i32>;
    pub static hash: DangerCell<CArr<b32x2>>;
    pub static prim: DangerCell<[b32x2; PRIM_SIZE + 1]>;
    pub static mut mem: DangerCell<CArr<memory_word>>;
}
