use crate::{
    auxi::AuxCommand,
    bibs::BibCommand,
    bst::BstCommand,
    buffer::{BufTy, GlobalBuffer},
    char_info::LexClass,
    entries::ENT_STR_SIZE,
    exec::ControlSeq,
    global::GLOB_STR_SIZE,
    hash,
    hash::{BstBuiltin, BstFn, HashData, HashTy},
    log::{output_bbl_line, print_overflow},
    ASCIICode, Bibtex, BibtexError, GlobalItems, HashPointer, PoolPointer, StrNumber,
};
use std::ops::Range;

const POOL_SIZE: usize = 65000;
pub(crate) const MAX_PRINT_LINE: usize = 79;
pub(crate) const MIN_PRINT_LINE: usize = 3;
pub(crate) const MAX_STRINGS: usize = 35307;

pub(crate) struct LookupResTy<T: HashTy> {
    /// The location of the string - where it exists, was inserted, of if insert is false,
    /// where it *would* have been inserted
    pub loc: usize,
    /// Whether the string existed in the hash table already
    pub extra: Option<T::Extra>,
}

impl<T: HashTy> Clone for LookupResTy<T> {
    fn clone(&self) -> Self {
        LookupResTy {
            loc: self.loc,
            extra: self.extra.clone(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum LookupErr {
    Invalid,
    DoesntExist,
}

pub(crate) struct StringPool {
    strings: Vec<u8>,
    // Stores string starting locations in the string pool
    // length of string `s` is offsets[s + 1] - offsets[s]
    offsets: Vec<usize>,
    pool_ptr: PoolPointer,
    str_ptr: StrNumber,
}

impl StringPool {
    pub(crate) fn new() -> StringPool {
        StringPool {
            strings: vec![0; POOL_SIZE + 1],
            offsets: vec![0; MAX_STRINGS + 1],
            pool_ptr: 0,
            str_ptr: 1,
        }
    }

    pub fn try_get_str(&self, s: usize) -> Result<&[u8], LookupErr> {
        // TODO: Why plus three? Should probably find if somewhere relies on that
        if s == 0 || s >= self.str_ptr + 3 {
            Err(LookupErr::DoesntExist)
        } else if s >= MAX_STRINGS {
            Err(LookupErr::Invalid)
        } else {
            Ok(&self.strings[self.offsets[s]..self.offsets[s + 1]])
        }
    }

    pub fn get_str(&self, s: usize) -> &[u8] {
        self.try_get_str(s).unwrap_or_else(|e| match e {
            LookupErr::DoesntExist => panic!("String number {} doesn't exist", s),
            LookupErr::Invalid => panic!("Invalid string number {}", s),
        })
    }

    pub fn grow(&mut self) {
        self.strings.resize(self.strings.len() + POOL_SIZE, 0);
    }

    /// Used while defining strings - declare the current `pool_ptr` as the end of the current
    /// string, increment the `str_ptr`, and return the new string's `StrNumber`
    pub fn make_string(&mut self, ctx: &mut Bibtex<'_, '_>) -> Result<StrNumber, BibtexError> {
        if self.str_ptr == MAX_STRINGS {
            print_overflow(ctx);
            ctx.write_logs(&format!("number of strings {}\n", MAX_STRINGS));
            return Err(BibtexError::Fatal);
        }
        self.str_ptr += 1;
        self.offsets[self.str_ptr] = self.pool_ptr;
        Ok(self.str_ptr - 1)
    }

    fn hash_str(hash: &HashData, str: &[ASCIICode]) -> usize {
        let prime = hash.prime();
        str.iter()
            .fold(0, |acc, &c| ((2 * acc) + c as usize) % prime)
    }

    pub fn lookup<T: HashTy>(&self, hash: &HashData, str: &[ASCIICode]) -> LookupResTy<T> {
        let h = Self::hash_str(hash, str);
        let mut p = h as HashPointer + hash::HASH_BASE as HashPointer;
        let exists = loop {
            let existing = hash.text(p);

            if existing > 0 && self.get_str(existing) == str && hash.node(p).kind() == T::ilk() {
                break true;
            }

            if hash.next(p) == 0 {
                break false;
            }

            p = hash.next(p);
        };

        let extra = if exists {
            Some(T::extra(&hash.node(p).extra))
        } else {
            None
        };

        LookupResTy { loc: p, extra }
    }

    // TODO: Use a different type from LookupResTy to better represent inserted vs not
    /// Lookup a string, inserting it if it isn't found. Note that this returns `Ok` whether the
    /// string is found or not, only returning `Err` if a called function fails.
    pub fn lookup_insert<T: HashTy>(
        &mut self,
        ctx: &mut Bibtex<'_, '_>,
        hash: &mut HashData,
        str: &[ASCIICode],
        extra: T::Extra,
    ) -> Result<LookupResTy<T>, BibtexError> {
        // Hash string using simple hash function. This hash is capped to HASH_PRIME
        let h = Self::hash_str(hash, str);
        let mut str_num = 0;
        // Get position by adding HASH_BASE
        let mut p = (h + hash::HASH_BASE) as HashPointer;

        // Look for an existing match, or the last slot
        let existing = loop {
            // Get the current text at the position
            let existing = hash.text(p);
            // If the text exists and is the same as the text we're adding
            if self.try_get_str(existing) == Ok(str) {
                // If an existing hash entry exists for this type, return it
                if hash.node(p).kind() == T::ilk() {
                    return Ok(LookupResTy {
                        loc: p,
                        extra: Some(T::extra(&hash.node(p).extra)),
                    });
                } else {
                    str_num = existing;
                }
            }

            if hash.next(p) == 0 {
                break existing;
            }

            p = hash.next(p);
        };

        // If we hit the end and the slot is already in use
        if existing > 0 {
            // Walk backwards from our current len to our first empty slot.
            // If all slots are full, error
            loop {
                if hash.len() == hash::HASH_BASE {
                    print_overflow(ctx);
                    ctx.write_logs(&format!("hash size {}\n", hash::HASH_SIZE));
                    return Err(BibtexError::Fatal);
                }
                hash.set_len(hash.len() - 1);

                if hash.text(hash.len()) == 0 {
                    break;
                }
            }
            // Set the next item to our new lowest open slot
            hash.set_next(p, hash.len());
            // Operate on the new empty slot
            p = hash.len();
        }

        // We found the string in the string pool while hunting for a slot
        if str_num > 0 {
            hash.set_text(p, str_num);
        // The string isn't in the string pool - add it
        } else {
            while self.pool_ptr + str.len() > self.strings.len() {
                self.grow();
            }
            self.strings[self.pool_ptr..self.pool_ptr + str.len()].copy_from_slice(str);
            self.pool_ptr += str.len();

            match self.make_string(ctx) {
                Ok(str) => hash.set_text(p, str),
                Err(err) => return Err(err),
            }
        }

        // Set the type of this slot
        hash.node_mut(p).extra = T::wrap(extra);

        Ok(LookupResTy {
            loc: p,
            extra: None,
        })
    }

    pub fn str_ptr(&self) -> usize {
        self.str_ptr
    }

    pub fn set_str_ptr(&mut self, val: usize) {
        self.str_ptr = val;
    }

    pub fn pool_ptr(&self) -> usize {
        self.pool_ptr
    }

    pub fn set_pool_ptr(&mut self, val: usize) {
        self.pool_ptr = val;
    }

    pub fn str_start(&self, str: StrNumber) -> usize {
        self.offsets[str]
    }

    // TODO: Encapsulate better
    pub fn set_start(&mut self, str: StrNumber, start: usize) {
        self.offsets[str] = start;
    }

    pub fn copy_raw(&mut self, str: StrNumber, pos: usize) {
        let start = self.offsets[str];
        let end = self.offsets[str + 1];

        while pos + (end - start) > self.strings.len() {
            self.grow();
        }

        self.strings.copy_within(start..end, pos);
    }

    pub fn copy_range_raw(&mut self, range: Range<usize>, pos: usize) {
        while pos + (range.end - range.start) > self.strings.len() {
            self.grow();
        }
        self.strings.copy_within(range, pos)
    }

    pub fn append(&mut self, c: ASCIICode) {
        self.strings[self.pool_ptr] = c;
        self.pool_ptr += 1;
    }

    pub fn add_string_raw(
        &mut self,
        ctx: &mut Bibtex<'_, '_>,
        str: &[ASCIICode],
    ) -> Result<PoolPointer, BibtexError> {
        while self.pool_ptr + str.len() > self.strings.len() {
            self.grow();
        }
        self.strings[self.pool_ptr..self.pool_ptr + str.len()].copy_from_slice(str);
        self.pool_ptr += str.len();
        self.make_string(ctx)
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }
}

pub(crate) fn add_buf_pool(pool: &StringPool, buffers: &mut GlobalBuffer, str: StrNumber) {
    let str = pool.get_str(str);

    if buffers.init(BufTy::Ex) + str.len() > buffers.len() {
        buffers.grow_all();
    }

    let start = buffers.init(BufTy::Ex);
    buffers.copy_from(BufTy::Ex, start, str);
    buffers.set_offset(BufTy::Ex, 1, start + str.len());
    buffers.set_init(BufTy::Ex, start + str.len());
}

pub(crate) fn pre_def_certain_strings(
    ctx: &mut Bibtex<'_, '_>,
    GlobalItems {
        pool,
        hash,
        other,
        entries,
        ..
    }: &mut GlobalItems<'_>,
) -> Result<(), BibtexError> {
    let res = pool.lookup_insert::<hash::FileExt>(ctx, hash, b".aux", ())?;
    ctx.s_aux_extension = hash.text(res.loc);

    pool.lookup_insert::<hash::AuxCmd>(ctx, hash, b"\\bibdata", AuxCommand::Data)?;
    pool.lookup_insert::<hash::AuxCmd>(ctx, hash, b"\\bibstyle", AuxCommand::Style)?;
    pool.lookup_insert::<hash::AuxCmd>(ctx, hash, b"\\citation", AuxCommand::Citation)?;
    pool.lookup_insert::<hash::AuxCmd>(ctx, hash, b"\\@input", AuxCommand::Input)?;

    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"entry", BstCommand::Entry)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"execute", BstCommand::Execute)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"function", BstCommand::Function)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"integers", BstCommand::Integers)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"iterate", BstCommand::Iterate)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"macro", BstCommand::Macro)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"read", BstCommand::Read)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"reverse", BstCommand::Reverse)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"sort", BstCommand::Sort)?;
    pool.lookup_insert::<hash::BstCmd>(ctx, hash, b"strings", BstCommand::Strings)?;

    pool.lookup_insert::<hash::BibCmd>(ctx, hash, b"comment", BibCommand::Comment)?;
    pool.lookup_insert::<hash::BibCmd>(ctx, hash, b"preamble", BibCommand::Preamble)?;
    pool.lookup_insert::<hash::BibCmd>(ctx, hash, b"string", BibCommand::String)?;

    let mut build_in = |pds: &[ASCIICode], builtin| {
        let res = pool.lookup_insert::<hash::Bst>(ctx, hash, pds, BstFn::Builtin(builtin))?;
        Ok(res.loc)
    };

    build_in(b"=", BstBuiltin::Eq)?;
    build_in(b">", BstBuiltin::Gt)?;
    build_in(b"<", BstBuiltin::Lt)?;
    build_in(b"+", BstBuiltin::Plus)?;
    build_in(b"-", BstBuiltin::Minus)?;
    build_in(b"*", BstBuiltin::Concat)?;
    build_in(b":=", BstBuiltin::Set)?;
    build_in(b"add.period$", BstBuiltin::AddPeriod)?;
    build_in(b"call.type$", BstBuiltin::CallType)?;
    build_in(b"change.case$", BstBuiltin::ChangeCase)?;
    build_in(b"chr.to.int$", BstBuiltin::ChrToInt)?;
    build_in(b"cite$", BstBuiltin::Cite)?;
    build_in(b"duplicate$", BstBuiltin::Duplicate)?;
    build_in(b"empty$", BstBuiltin::Empty)?;
    build_in(b"format.name$", BstBuiltin::FormatName)?;
    build_in(b"if$", BstBuiltin::If)?;
    build_in(b"int.to.chr$", BstBuiltin::IntToChr)?;
    build_in(b"int.to.str$", BstBuiltin::IntToStr)?;
    build_in(b"missing$", BstBuiltin::Missing)?;
    build_in(b"newline$", BstBuiltin::Newline)?;
    build_in(b"num.names$", BstBuiltin::NumNames)?;
    build_in(b"pop$", BstBuiltin::Pop)?;
    build_in(b"preamble$", BstBuiltin::Preamble)?;
    build_in(b"purify$", BstBuiltin::Purify)?;
    build_in(b"quote$", BstBuiltin::Quote)?;
    let skip_loc = build_in(b"skip$", BstBuiltin::Skip)?;
    build_in(b"stack$", BstBuiltin::Stack)?;
    build_in(b"substring$", BstBuiltin::Substring)?;
    build_in(b"swap$", BstBuiltin::Swap)?;
    build_in(b"text.length$", BstBuiltin::TextLength)?;
    build_in(b"text.prefix$", BstBuiltin::TextPrefix)?;
    build_in(b"top$", BstBuiltin::Top)?;
    build_in(b"type$", BstBuiltin::Type)?;
    build_in(b"warning$", BstBuiltin::Warning)?;
    build_in(b"while$", BstBuiltin::While)?;
    build_in(b"width$", BstBuiltin::Width)?;
    build_in(b"write$", BstBuiltin::Write)?;

    let res = pool.lookup_insert::<hash::Text>(ctx, hash, b"", ())?;
    ctx.s_null = hash.text(res.loc);
    let res = pool.lookup_insert::<hash::Text>(ctx, hash, b"default.type", ())?;
    ctx.s_default = hash.text(res.loc);
    ctx.b_default = skip_loc;

    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"i", ControlSeq::LowerI)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"j", ControlSeq::LowerJ)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"oe", ControlSeq::LowerOE)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"OE", ControlSeq::UpperOE)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"ae", ControlSeq::LowerAE)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"AE", ControlSeq::UpperAE)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"aa", ControlSeq::LowerAA)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"AA", ControlSeq::UpperAA)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"o", ControlSeq::LowerO)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"O", ControlSeq::UpperO)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"l", ControlSeq::LowerL)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"L", ControlSeq::UpperL)?;
    pool.lookup_insert::<hash::CtrlSeq>(ctx, hash, b"ss", ControlSeq::LowerSS)?;

    let num_fields = other.num_fields();
    pool.lookup_insert::<hash::Bst>(ctx, hash, b"crossref", BstFn::Field(num_fields))?;
    other.set_crossref_num(num_fields);
    other.set_num_fields(num_fields + 1);
    other.set_pre_defined_fields(num_fields + 1);

    let num_ent_strs = entries.num_ent_strs();
    pool.lookup_insert::<hash::Bst>(ctx, hash, b"sort.key$", BstFn::StrEntry(num_ent_strs))?;
    entries.set_sort_key_num(num_ent_strs);
    entries.set_num_ent_strs(num_ent_strs + 1);

    pool.lookup_insert::<hash::Bst>(
        ctx,
        hash,
        b"entry.max$",
        BstFn::IntGlbl(ENT_STR_SIZE as i64),
    )?;

    pool.lookup_insert::<hash::Bst>(
        ctx,
        hash,
        b"global.max$",
        BstFn::IntGlbl(GLOB_STR_SIZE as i64),
    )?;

    Ok(())
}

pub(crate) fn add_out_pool(
    ctx: &mut Bibtex<'_, '_>,
    buffers: &mut GlobalBuffer,
    pool: &StringPool,
    str: StrNumber,
) {
    let str = pool.get_str(str);

    while buffers.init(BufTy::Out) + str.len() > buffers.len() {
        buffers.grow_all();
    }

    let out_offset = buffers.init(BufTy::Out);
    buffers.copy_from(BufTy::Out, out_offset, str);
    buffers.set_init(BufTy::Out, out_offset + str.len());

    let mut unbreakable_tail = false;
    while buffers.init(BufTy::Out) > MAX_PRINT_LINE && !unbreakable_tail {
        let end_ptr = buffers.init(BufTy::Out);
        let mut out_offset = MAX_PRINT_LINE;
        let mut break_pt_found = false;

        while LexClass::of(buffers.at(BufTy::Out, out_offset)) != LexClass::Whitespace
            && out_offset >= MIN_PRINT_LINE
        {
            out_offset -= 1;
        }

        if out_offset == MIN_PRINT_LINE - 1 {
            out_offset = MAX_PRINT_LINE + 1;
            while out_offset < end_ptr {
                if LexClass::of(buffers.at(BufTy::Out, out_offset)) != LexClass::Whitespace {
                    out_offset += 1;
                } else {
                    break;
                }
            }

            if out_offset == end_ptr {
                unbreakable_tail = true;
            } else {
                break_pt_found = true;
                while out_offset + 1 < end_ptr {
                    if LexClass::of(buffers.at(BufTy::Out, out_offset + 1)) == LexClass::Whitespace
                    {
                        out_offset += 1;
                    } else {
                        break;
                    }
                }
            }
        } else {
            break_pt_found = true;
        }

        if break_pt_found {
            buffers.set_init(BufTy::Out, out_offset);
            let break_ptr = buffers.init(BufTy::Out) + 1;
            output_bbl_line(ctx, buffers);
            buffers.set_at(BufTy::Out, 0, b' ');
            buffers.set_at(BufTy::Out, 1, b' ');
            let len = end_ptr - break_ptr;
            buffers.copy_within(BufTy::Out, BufTy::Out, break_ptr, 2, len);
            buffers.set_init(BufTy::Out, len + 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BibtexConfig;
    use tectonic_bridge_core::{CoreBridgeLauncher, CoreBridgeState, MinimalDriver};
    use tectonic_io_base::{stack::IoStack, IoProvider};
    use tectonic_status_base::NoopStatusBackend;

    fn with_cbs(f: impl FnOnce(&mut CoreBridgeState<'_>)) {
        let io_list: Vec<&mut dyn IoProvider> = vec![];
        let io = IoStack::new(io_list);
        let mut hooks = MinimalDriver::new(io);
        let mut status = NoopStatusBackend::default();
        let mut cbl = CoreBridgeLauncher::new(&mut hooks, &mut status);
        cbl.with_global_lock(|cbs| {
            f(cbs);
            Ok(())
        })
        .unwrap();
    }

    // TODO: Create context without backend? Use custom backend-like type?
    //       Implement the relevant interfaces ourself?
    #[test]
    fn test_pool() {
        with_cbs(|cbs| {
            let mut ctx = Bibtex::new(cbs, BibtexConfig::default());
            let mut hash = HashData::new();
            let mut new_pool = StringPool::new();
            let res = new_pool
                .lookup_insert::<hash::Text>(&mut ctx, &mut hash, b"a cool string", ())
                .unwrap();
            assert!(res.extra.is_none());
            assert_eq!(
                new_pool.try_get_str(hash.text(res.loc)),
                Ok(b"a cool string" as &[_])
            );

            let res2 = new_pool
                .lookup_insert::<hash::Text>(&mut ctx, &mut hash, b"a cool string", ())
                .unwrap();
            assert!(res2.extra.is_some());
            assert_eq!(
                new_pool.try_get_str(hash.text(res2.loc)),
                Ok(b"a cool string" as &[_])
            );

            let res3 = new_pool.lookup::<hash::Text>(&hash, b"a cool string");
            assert!(res3.extra.is_some());
            assert_eq!(
                new_pool.try_get_str(hash.text(res3.loc)),
                Ok(b"a cool string" as &[_])
            );

            let res4 = new_pool.lookup::<hash::Text>(&hash, b"a bad string");
            assert!(res4.extra.is_none());
            assert_eq!(
                new_pool.try_get_str(hash.text(res4.loc)),
                Err(LookupErr::DoesntExist)
            );
        })
    }
}
