use crate::{
    bibs::BibData,
    buffer::BufTy,
    cite::CiteInfo,
    exec::print_bst_name,
    hash::HashExtra,
    log::{
        aux_end1_err_print, aux_end2_err_print, aux_err_illegal_another_print,
        aux_err_no_right_brace_print, aux_err_print, aux_err_stuff_after_right_brace_print,
        aux_err_white_space_in_argument_print, hash_cite_confusion, log_pr_aux_name,
        log_pr_bst_name, print_a_pool_str, print_a_token, print_aux_name, print_bib_name,
        print_confusion, print_overflow, AuxTy,
    },
    peekable::PeekableInput,
    pool::StringPool,
    Bibtex, BibtexError, File, GlobalItems, StrIlk, StrNumber,
};
use chumsky::Parser;
use std::ffi::CString;
use tectonic_bridge_core::FileFormat;

const AUX_STACK_SIZE: usize = 20;

#[derive(Debug, PartialEq)]
enum CmdArgs<T> {
    Args(T),
    NoRightBrace(usize),
    WhitespaceInArg(usize),
    StuffAfterRightBrace(usize),
}

#[derive(Debug, PartialEq)]
enum AuxKind {
    Data(CmdArgs<Vec<Vec<u8>>>),
    Style(CmdArgs<Vec<u8>>),
    Citation(CmdArgs<Vec<Vec<u8>>>),
    Input(CmdArgs<Vec<u8>>),
}

#[derive(Debug, PartialEq)]
struct AuxCmd {
    offset: usize,
    kind: AuxKind,
}

type Error = chumsky::error::Cheap<u8>;

fn aux_parser() -> impl Parser<u8, Option<AuxCmd>, Error = Error> {
    use chumsky::prelude::*;

    fn sarb<T>((cur, end): (CmdArgs<T>, Option<()>), span: core::ops::Range<usize>) -> CmdArgs<T> {
        if let Some(_) = end {
            cur
        } else {
            CmdArgs::StuffAfterRightBrace(span.end)
        }
    }

    fn nrb<T>(_: Vec<u8>, span: core::ops::Range<usize>) -> CmdArgs<T> {
        CmdArgs::NoRightBrace(span.end)
    }

    let arg = none_of::<_, _, Error>([b'}'])
        .repeated()
        .map_with_span(move |str, span| {
            if str.iter().copied().any(|c: u8| c.is_ascii_whitespace()) {
                CmdArgs::WhitespaceInArg(span.start)
            } else {
                CmdArgs::Args(str)
            }
        })
        .then_ignore(just(b'}'))
        .then(end().or_not())
        .map_with_span(sarb)
        .or(any().repeated().map_with_span(nrb));

    let args = none_of::<_, _, Error>([b'}', b','])
        .repeated()
        .map(move |str| {
            if str.iter().copied().any(|c: u8| c.is_ascii_whitespace()) {
                None
            } else {
                Some(str)
            }
        })
        .separated_by(just(b','))
        .map_with_span(move |strs, span| {
            let (ws, strs) =
                strs.into_iter()
                    .fold((false, Vec::new()), |(is_ws, mut strs), str| match str {
                        Some(str) => {
                            strs.push(str);
                            (is_ws, strs)
                        }
                        None => (true, strs),
                    });
            if ws {
                CmdArgs::WhitespaceInArg(span.start)
            } else {
                CmdArgs::Args(strs)
            }
        })
        .then_ignore(just(b'}'))
        .then(end().or_not())
        .map_with_span(sarb)
        .or(any().repeated().map_with_span(nrb));

    let cmd = |cmd: &'static [u8]| {
        just::<_, _, Error>(cmd)
            .map_with_span(|_, span| span.end)
            .then_ignore(just(b'{'))
    };

    let bibdata = cmd(b"\\bibdata").then(args.clone().map(AuxKind::Data));
    let bibstyle = cmd(b"\\bibstyle").then(arg.clone().map(AuxKind::Style));
    let citation = cmd(b"\\citation").then(args.map(AuxKind::Citation));
    let input = cmd(b"\\@input").then(arg.map(AuxKind::Input));

    choice((bibdata, bibstyle, citation, input))
        .map(|(offset, kind)| AuxCmd { offset, kind })
        .or_not()
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum AuxCommand {
    Data,
    Style,
    Citation,
    Input,
}

pub(crate) struct AuxData {
    aux: Vec<File>,
}

impl AuxData {
    pub fn new() -> AuxData {
        AuxData { aux: Vec::new() }
    }

    pub fn push_file(&mut self, file: File) {
        self.aux.push(file);
    }

    pub fn pop_file(&mut self) -> (File, bool) {
        let out = self.aux.pop().unwrap();
        (out, self.aux.is_empty())
    }

    pub fn top_file(&self) -> &File {
        self.aux.last().unwrap()
    }

    pub fn top_file_mut(&mut self) -> &mut File {
        self.aux.last_mut().unwrap()
    }

    pub fn ptr(&self) -> usize {
        self.aux.len()
    }
}

macro_rules! unwrap_args {
    ($ctx:ident, $globals:ident, $args:ident) => {
        match $args {
            CmdArgs::Args(arg) => arg,
            CmdArgs::NoRightBrace(offset) => {
                $globals.buffers.set_offset(BufTy::Base, 2, offset);
                aux_err_no_right_brace_print($ctx);
                aux_err_print($ctx, $globals.buffers, $globals.aux, $globals.pool)?;
                return Ok(());
            }
            CmdArgs::WhitespaceInArg(offset) => {
                $globals.buffers.set_offset(BufTy::Base, 2, offset);
                aux_err_white_space_in_argument_print($ctx);
                aux_err_print($ctx, $globals.buffers, $globals.aux, $globals.pool)?;
                return Ok(());
            }
            CmdArgs::StuffAfterRightBrace(offset) => {
                $globals.buffers.set_offset(BufTy::Base, 2, offset);
                aux_err_stuff_after_right_brace_print($ctx);
                aux_err_print($ctx, $globals.buffers, $globals.aux, $globals.pool)?;
                return Ok(());
            }
        }
    };
}

pub(crate) fn get_aux_command_and_process(
    ctx: &mut Bibtex<'_, '_>,
    globals: &mut GlobalItems<'_>,
) -> Result<(), BibtexError> {
    let range = 0..globals.buffers.init(BufTy::Base);
    let line = &globals.buffers.buffer(BufTy::Base)[range];

    let cmd = aux_parser().parse(line).unwrap();

    let cmd = match cmd {
        Some(cmd) => cmd,
        None => return Ok(()),
    };

    match cmd.kind {
        AuxKind::Citation(cites) => {
            ctx.citation_seen = true;

            let cites = unwrap_args!(ctx, globals, cites);
            globals.buffers.set_offset(BufTy::Base, 2, line.len());

            for cite in cites {
                if cite == b"*" {
                    if ctx.all_entries {
                        ctx.write_logs("Multiple inclusions of entire database\n");
                        aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                        return Ok(());
                    } else {
                        ctx.all_entries = true;
                        globals.cites.set_all_marker(globals.cites.ptr());
                        continue;
                    }
                }

                let lc_cite = cite.to_ascii_lowercase();

                let lc_res = globals.pool.lookup_str_insert(
                    ctx,
                    globals.hash,
                    &lc_cite,
                    HashExtra::LcCite(0),
                )?;
                if lc_res.exists {
                    let HashExtra::LcCite(cite_loc) = globals.hash.node(lc_res.loc).extra else {
                        panic!("LcCite lookup didn't have LcCite extra");
                    };
                    let uc_res = globals.pool.lookup_str(globals.hash, &cite, StrIlk::Cite);
                    if !uc_res.exists {
                        let HashExtra::Cite(cite) = globals.hash.node(cite_loc).extra else {
                            panic!("LcCite location didn't have a Cite extra");
                        };

                        ctx.write_logs("Case mismatch error between cite keys ");
                        print_a_token(ctx, globals.buffers);
                        ctx.write_logs(" and ");
                        print_a_pool_str(ctx, globals.cites.get_cite(cite), globals.pool)?;
                        ctx.write_logs("\n");
                        aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                        return Ok(());
                    }
                } else {
                    let uc_res = globals.pool.lookup_str_insert(
                        ctx,
                        globals.hash,
                        &cite,
                        HashExtra::Cite(0),
                    )?;
                    if uc_res.exists {
                        hash_cite_confusion(ctx);
                        return Err(BibtexError::Fatal);
                    }

                    if globals.cites.ptr() == globals.cites.len() {
                        globals.cites.grow();
                    }

                    globals
                        .cites
                        .set_cite(globals.cites.ptr(), globals.hash.text(uc_res.loc));
                    globals.hash.node_mut(uc_res.loc).extra = HashExtra::Cite(globals.cites.ptr());
                    globals.hash.node_mut(lc_res.loc).extra = HashExtra::LcCite(uc_res.loc);
                    globals.cites.set_ptr(globals.cites.ptr() + 1);
                }
            }
        }
        AuxKind::Data(files) => {
            if ctx.bib_seen {
                globals.buffers.set_offset(BufTy::Base, 2, cmd.offset);
                aux_err_illegal_another_print(ctx, AuxTy::Data)?;
                aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                return Ok(());
            }
            ctx.bib_seen = true;

            let files = unwrap_args!(ctx, globals, files);
            globals.buffers.set_offset(BufTy::Base, 2, line.len());

            for file in files {
                let res =
                    globals
                        .pool
                        .lookup_str_insert(ctx, globals.hash, &file, HashExtra::BibFile)?;
                if res.exists {
                    ctx.write_logs("This database file appears more than once: ");
                    print_bib_name(ctx, globals.pool, globals.hash.text(res.loc))?;
                    aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                    return Ok(());
                }

                let name = globals.pool.get_str(globals.hash.text(res.loc));
                let fname = CString::new(name).unwrap();
                let bib_in = PeekableInput::open(ctx, &fname, FileFormat::Bib);
                match bib_in {
                    Err(_) => {
                        ctx.write_logs("I couldn't open database file ");
                        print_bib_name(ctx, globals.pool, globals.hash.text(res.loc))?;
                        aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                        return Ok(());
                    }
                    Ok(file) => {
                        globals.bibs.push_file(File {
                            name: globals.hash.text(res.loc),
                            file,
                            line: 0,
                        });
                    }
                }
            }
        }
        AuxKind::Input(file) => {
            let file = unwrap_args!(ctx, globals, file);
            globals.buffers.set_offset(BufTy::Base, 2, line.len());

            if globals.aux.ptr() == AUX_STACK_SIZE {
                print_a_token(ctx, globals.buffers);
                ctx.write_logs(": ");
                print_overflow(ctx);
                ctx.write_logs(&format!("auxiliary file depth {}\n", AUX_STACK_SIZE));
                return Err(BibtexError::Fatal);
            }

            let aux_ext = globals.pool.get_str(ctx.s_aux_extension);
            let aux_extension_ok =
                file.len() >= aux_ext.len() || *aux_ext != file[file.len() - aux_ext.len()..];

            if !aux_extension_ok {
                print_a_token(ctx, globals.buffers);
                ctx.write_logs(" has a wrong extension");
                aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                return Ok(());
            }

            let res =
                globals
                    .pool
                    .lookup_str_insert(ctx, globals.hash, &file, HashExtra::AuxFile)?;
            if res.exists {
                ctx.write_logs("Already encountered file ");
                print_aux_name(ctx, globals.pool, globals.hash.text(res.loc))?;
                aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                return Ok(());
            }

            let name = globals.pool.get_str(globals.hash.text(res.loc));
            let fname = CString::new(name).unwrap();
            let file = PeekableInput::open(ctx, &fname, FileFormat::Tex);
            match file {
                Err(_) => {
                    ctx.write_logs("I couldn't open auxiliary file ");
                    print_aux_name(ctx, globals.pool, globals.hash.text(res.loc))?;
                    aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                    return Ok(());
                }
                Ok(file) => {
                    globals.aux.push_file(File {
                        name: globals.hash.text(res.loc),
                        file,
                        line: 0,
                    });
                }
            }

            ctx.write_logs(&format!(
                "A level-{} auxiliary file: ",
                globals.aux.ptr() - 1
            ));
            log_pr_aux_name(ctx, globals.aux, globals.pool)?;
        }
        AuxKind::Style(file) => {
            if ctx.bst_seen {
                globals.buffers.set_offset(BufTy::Base, 2, cmd.offset);
                aux_err_illegal_another_print(ctx, AuxTy::Style)?;
                aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                return Ok(());
            }
            ctx.bst_seen = true;

            let file = unwrap_args!(ctx, globals, file);
            globals.buffers.set_offset(BufTy::Base, 2, line.len());

            let res =
                globals
                    .pool
                    .lookup_str_insert(ctx, globals.hash, &file, HashExtra::BstFile)?;
            if res.exists {
                ctx.write_logs("Already encountered style file");
                print_confusion(ctx);
                return Err(BibtexError::Fatal);
            }

            let name = globals.pool.get_str(globals.hash.text(res.loc));
            let fname = CString::new(name).unwrap();
            let bst_file = PeekableInput::open(ctx, &fname, FileFormat::Bst);
            match bst_file {
                Err(_) => {
                    ctx.write_logs("I couldn't open style file ");
                    print_bst_name(ctx, globals.pool, globals.hash.text(res.loc))?;
                    ctx.bst = None;
                    aux_err_print(ctx, globals.buffers, globals.aux, globals.pool)?;
                    return Ok(());
                }
                Ok(file) => {
                    ctx.bst = Some(File {
                        name: globals.hash.text(res.loc),
                        file,
                        line: 0,
                    });
                }
            }

            if ctx.config.verbose {
                ctx.write_logs("The style file: ");
                print_bst_name(ctx, globals.pool, ctx.bst.as_ref().unwrap().name)?;
            } else {
                ctx.write_log_file("The style file: ");
                log_pr_bst_name(ctx, globals.pool)?;
            }
        }
    }

    Ok(())
}

pub(crate) fn pop_the_aux_stack(ctx: &mut Bibtex<'_, '_>, aux: &mut AuxData) -> Option<StrNumber> {
    let (file, last) = aux.pop_file();
    file.file.close(ctx).unwrap();
    if last {
        Some(file.name)
    } else {
        None
    }
}

pub(crate) fn last_check_for_aux_errors(
    ctx: &mut Bibtex<'_, '_>,
    pool: &StringPool,
    cites: &mut CiteInfo,
    bibs: &BibData,
    last_aux: StrNumber,
) -> Result<(), BibtexError> {
    cites.set_num_cites(cites.ptr());
    if !ctx.citation_seen {
        aux_end1_err_print(ctx);
        ctx.write_logs("\\citation commands");
        aux_end2_err_print(ctx, pool, last_aux)?;
    } else if cites.num_cites() == 0 && !ctx.all_entries {
        aux_end1_err_print(ctx);
        ctx.write_logs("cite keys");
        aux_end2_err_print(ctx, pool, last_aux)?;
    }

    if !ctx.bib_seen {
        aux_end1_err_print(ctx);
        ctx.write_logs("\\bibdata command");
        aux_end2_err_print(ctx, pool, last_aux)?;
    } else if bibs.len() == 0 {
        aux_end1_err_print(ctx);
        ctx.write_logs("database files");
        aux_end2_err_print(ctx, pool, last_aux)?;
    }

    if !ctx.bst_seen {
        aux_end1_err_print(ctx);
        ctx.write_logs("\\bibstyle command");
        aux_end2_err_print(ctx, pool, last_aux)?;
    } else if ctx.bst.is_none() {
        aux_end1_err_print(ctx);
        ctx.write_logs("style file");
        aux_end2_err_print(ctx, pool, last_aux)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;

    #[test]
    fn test_parser() {
        let parser = aux_parser();

        let cmd = parser.parse(b"\\citation{a,b}").unwrap().unwrap();

        assert_eq!(
            cmd.kind,
            AuxKind::Citation(CmdArgs::Args(vec![b"a".to_vec(), b"b".to_vec()])),
        );

        let cmd = parser.parse(b"\\bibdata{a b}").unwrap().unwrap();

        assert!(matches!(
            cmd.kind,
            AuxKind::Data(CmdArgs::WhitespaceInArg(_))
        ));

        let cmd = parser.parse(b"\\bibstyle{abc}  ").unwrap().unwrap();

        assert!(matches!(
            cmd.kind,
            AuxKind::Style(CmdArgs::StuffAfterRightBrace(_))
        ));

        let cmd = parser.parse(b"\\@input{foo").unwrap().unwrap();

        assert!(matches!(cmd.kind, AuxKind::Input(CmdArgs::NoRightBrace(_))));
    }
}
