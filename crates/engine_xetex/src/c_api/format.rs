/* Fixed array sizes. */

pub const PRIM_SIZE: usize = 2100;

/* Offsets in the equivalents table. */

pub const ACTIVE_BASE: usize = 0x1;
pub const SINGLE_BASE: usize = 0x110001;
pub const NULL_CS: usize = 0x220001;
pub const HASH_BASE: usize = 0x220002;
pub const PRIM_EQTB_BASE: usize = 0x226603;
pub const FROZEN_NULL_FONT: usize = 0x223aa6;
pub const UNDEFINED_CONTROL_SEQUENCE: usize = 0x226603;
pub const CAT_CODE_BASE: usize = 0x226d29;
pub const EQTB_SIZE: usize = 0x886f92;

/* Codes for core engine commands. */

pub const LETTER: i32 = 0xb;

/* Math font sizes. */

pub const TEXT_SIZE: i32 = 0;
pub const SCRIPT_SIZE: i32 = 0x100;
pub const SCRIPT_SCRIPT_SIZE: i32 = 0x200;
