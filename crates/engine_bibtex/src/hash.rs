use crate::{
    auxi::AuxCommand, bibs::BibCommand, bst::BstCommand, exec::ControlSeq, pool, CiteNumber,
    FnDefLoc, HashPointer, StrNumber,
};

pub(crate) const HASH_BASE: usize = 1;
pub(crate) const HASH_SIZE: usize = if pool::MAX_STRINGS > 5000 {
    pool::MAX_STRINGS
} else {
    5000
};
const HASH_MAX: usize = HASH_SIZE + HASH_BASE - 1;
pub(crate) const HASH_PRIME: usize = compute_hash_prime();

/// Calculate a prime number for use in hashing that's at least 17/20 of `HASH_SIZE`
const fn compute_hash_prime() -> usize {
    const HASH_WANT: usize = HASH_SIZE / 20 * 17;

    let mut primes = [0; HASH_SIZE];
    let mut sieve = [0; HASH_SIZE];

    let mut j = 1;
    let mut k = 1;
    let mut hash_prime = 2;
    primes[k] = hash_prime;
    let mut o = 2;
    let mut square = 9;

    while hash_prime < HASH_WANT {
        loop {
            j += 2;
            if j == square {
                sieve[o] = j;
                j += 2;
                o += 1;
                square = primes[o] * primes[o];
            }

            let mut n = 2;
            let mut j_prime = true;
            while n < o && j_prime {
                while sieve[n] < j {
                    sieve[n] += 2 * primes[n];
                }
                if sieve[n] == j {
                    j_prime = false;
                }
                n += 1;
            }

            if j_prime {
                break;
            }
        }

        k += 1;
        hash_prime = j;
        primes[k] = hash_prime;
    }

    hash_prime
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StrIlk {
    Text,
    Integer,
    AuxCommand,
    AuxFile,
    BstCommand,
    BstFile,
    BibFile,
    FileExt,
    Cite,
    LcCite,
    BstFn,
    BibCommand,
    Macro,
    ControlSeq,
}

pub(crate) trait HashTy {
    type Extra: Clone;

    fn ilk() -> StrIlk;
    fn extra(extra: &HashExtra) -> Self::Extra;
    fn wrap(extra: Self::Extra) -> HashExtra;
}

pub(crate) struct Text;

impl HashTy for Text {
    type Extra = ();

    fn ilk() -> StrIlk {
        StrIlk::Text
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::Text = extra {
            ()
        } else {
            unreachable!()
        }
    }

    fn wrap(_: Self::Extra) -> HashExtra {
        HashExtra::Text
    }
}

pub(crate) struct Int;

impl HashTy for Int {
    type Extra = i64;

    fn ilk() -> StrIlk {
        StrIlk::Integer
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::Integer(i) = extra {
            *i
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::Integer(extra)
    }
}

pub(crate) struct CtrlSeq;

impl HashTy for CtrlSeq {
    type Extra = ControlSeq;

    fn ilk() -> StrIlk {
        StrIlk::ControlSeq
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::ControlSeq(cs) = extra {
            *cs
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::ControlSeq(extra)
    }
}

pub(crate) struct AuxCmd;

impl HashTy for AuxCmd {
    type Extra = AuxCommand;

    fn ilk() -> StrIlk {
        StrIlk::AuxCommand
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::AuxCommand(cmd) = extra {
            *cmd
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::AuxCommand(extra)
    }
}

pub(crate) struct Cite;

impl HashTy for Cite {
    type Extra = CiteNumber;

    fn ilk() -> StrIlk {
        StrIlk::Cite
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::Cite(num) = extra {
            *num
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::Cite(extra)
    }
}

pub(crate) struct LcCite;

impl HashTy for LcCite {
    type Extra = HashPointer;

    fn ilk() -> StrIlk {
        StrIlk::LcCite
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::LcCite(num) = extra {
            *num
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::LcCite(extra)
    }
}

pub(crate) struct Bst;

impl HashTy for Bst {
    type Extra = BstFn;

    fn ilk() -> StrIlk {
        StrIlk::BstFn
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::BstFn(bst) = extra {
            *bst
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::BstFn(extra)
    }
}

pub(crate) struct Macro;

impl HashTy for Macro {
    type Extra = StrNumber;

    fn ilk() -> StrIlk {
        StrIlk::Macro
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::Macro(num) = extra {
            *num
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::Macro(extra)
    }
}

pub(crate) struct BibCmd;

impl HashTy for BibCmd {
    type Extra = BibCommand;

    fn ilk() -> StrIlk {
        StrIlk::BibCommand
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::BibCommand(cmd) = extra {
            *cmd
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::BibCommand(extra)
    }
}

pub(crate) struct BstCmd;

impl HashTy for BstCmd {
    type Extra = BstCommand;

    fn ilk() -> StrIlk {
        StrIlk::BstCommand
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::BstCommand(cmd) = extra {
            *cmd
        } else {
            unreachable!()
        }
    }

    fn wrap(extra: Self::Extra) -> HashExtra {
        HashExtra::BstCommand(extra)
    }
}

pub(crate) struct BibFile;

impl HashTy for BibFile {
    type Extra = ();

    fn ilk() -> StrIlk {
        StrIlk::BibFile
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::BibFile = extra {
        } else {
            unreachable!()
        }
    }

    fn wrap(_: Self::Extra) -> HashExtra {
        HashExtra::BibFile
    }
}

pub(crate) struct FileExt;

impl HashTy for FileExt {
    type Extra = ();

    fn ilk() -> StrIlk {
        StrIlk::FileExt
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::FileExt = extra {
        } else {
            unreachable!()
        }
    }

    fn wrap(_: Self::Extra) -> HashExtra {
        HashExtra::FileExt
    }
}

pub(crate) struct BstFile;

impl HashTy for BstFile {
    type Extra = ();

    fn ilk() -> StrIlk {
        StrIlk::BstFile
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::BstFile = extra {
        } else {
            unreachable!()
        }
    }

    fn wrap(_: Self::Extra) -> HashExtra {
        HashExtra::BstFile
    }
}

pub(crate) struct AuxFile;

impl HashTy for AuxFile {
    type Extra = ();

    fn ilk() -> StrIlk {
        StrIlk::AuxFile
    }

    fn extra(extra: &HashExtra) -> Self::Extra {
        if let HashExtra::AuxFile = extra {
        } else {
            unreachable!()
        }
    }

    fn wrap(_: Self::Extra) -> HashExtra {
        HashExtra::AuxFile
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum BstBuiltin {
    Eq,
    Gt,
    Lt,
    Plus,
    Minus,
    Concat,
    Set,
    AddPeriod,
    CallType,
    ChangeCase,
    ChrToInt,
    Cite,
    Duplicate,
    Empty,
    FormatName,
    If,
    IntToChr,
    IntToStr,
    Missing,
    Newline,
    NumNames,
    Pop,
    Preamble,
    Purify,
    Quote,
    Skip,
    Stack,
    Substring,
    Swap,
    TextLength,
    TextPrefix,
    Top,
    Type,
    Warning,
    While,
    Width,
    Write,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum BstFn {
    Builtin(BstBuiltin),
    Wizard(FnDefLoc),
    Field(usize),
    IntEntry(usize),
    StrEntry(usize),
    IntGlbl(i64),
    StrGlbl(usize),
}

#[derive(Clone, Default, Debug)]
pub enum HashExtra {
    #[default]
    Text,
    Integer(i64),
    AuxCommand(AuxCommand),
    AuxFile,
    BstCommand(BstCommand),
    BstFile,
    BibFile,
    FileExt,
    Cite(CiteNumber),
    LcCite(HashPointer),
    BstFn(BstFn),
    BibCommand(BibCommand),
    Macro(StrNumber),
    ControlSeq(ControlSeq),
}

impl HashExtra {
    pub(crate) fn kind(&self) -> StrIlk {
        match self {
            HashExtra::Text => StrIlk::Text,
            HashExtra::Integer(_) => StrIlk::Integer,
            HashExtra::AuxCommand(_) => StrIlk::AuxCommand,
            HashExtra::AuxFile => StrIlk::AuxFile,
            HashExtra::BstCommand(_) => StrIlk::BstCommand,
            HashExtra::BstFile => StrIlk::BstFile,
            HashExtra::BibFile => StrIlk::BibFile,
            HashExtra::FileExt => StrIlk::FileExt,
            HashExtra::Cite(_) => StrIlk::Cite,
            HashExtra::LcCite(_) => StrIlk::LcCite,
            HashExtra::BstFn(_) => StrIlk::BstFn,
            HashExtra::BibCommand(_) => StrIlk::BibCommand,
            HashExtra::Macro(_) => StrIlk::Macro,
            HashExtra::ControlSeq(_) => StrIlk::ControlSeq,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct HashNode {
    next: HashPointer,
    text: StrNumber,
    pub(crate) extra: HashExtra,
}

impl HashNode {
    pub(crate) fn kind(&self) -> StrIlk {
        self.extra.kind()
    }
}

// TODO: Split string-pool stuff into string pool, executor stuff into execution context
pub(crate) struct HashData {
    hash_data: Vec<HashNode>,
    len: usize,
}

impl HashData {
    pub(crate) fn new() -> HashData {
        HashData {
            hash_data: vec![HashNode::default(); HASH_MAX + 1],
            len: HASH_MAX + 1,
        }
    }

    pub fn undefined() -> usize {
        HASH_MAX + 1
    }

    pub fn end_of_def() -> usize {
        HASH_MAX + 1
    }

    pub fn node(&self, pos: usize) -> &HashNode {
        &self.hash_data[pos]
    }

    pub fn node_mut(&mut self, pos: usize) -> &mut HashNode {
        &mut self.hash_data[pos]
    }

    pub fn text(&self, pos: usize) -> StrNumber {
        self.hash_data[pos].text
    }

    pub fn set_text(&mut self, pos: usize, val: StrNumber) {
        self.hash_data[pos].text = val;
    }

    pub fn next(&self, pos: usize) -> HashPointer {
        self.hash_data[pos].next
    }

    pub fn set_next(&mut self, pos: usize, val: HashPointer) {
        self.hash_data[pos].next = val
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn set_len(&mut self, val: usize) {
        self.len = val;
    }

    pub fn prime(&self) -> usize {
        HASH_PRIME
    }
}
