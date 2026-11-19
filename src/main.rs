//! # Batscript
//!
//! A scripting language written in Rust.
//!
//! This project is not, and will not ever be, written with the help of any form of generative AI.
//! I do not like generative AI. I do not support it. It is a net negative on society and harms learning.

#![feature(
    try_from_int_error_kind, // used in number literal error
    deque_extend_front,
    const_destruct,
    const_trait_impl,
    const_convert,
    const_cmp,
    const_format_args,
    adt_const_params,
    unboxed_closures,
    pattern,
    impl_trait_in_bindings,
    macro_derive,
    deref_patterns,
    extend_one,
    deref_pure_trait,
    const_ops,
    iterator_try_reduce,
    map_try_insert,
    formatting_options,
    default_ignorable,
    try_entry,
)]
#![forbid(
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    clippy::correctness, // actually mandates correctness
    reason = "write sound code"
)]
#![deny(
    clippy::multiple_unsafe_ops_per_block,
    unused_unsafe,
    clippy::unnecessary_safety_doc,
    clippy::unnecessary_safety_comment,
    clippy::same_length_and_capacity,
    reason = "could cause mistakes"
)]
#![warn(
    clippy::pedantic,
    clippy::missing_const_for_fn,
    // clippy::missing_docs_in_private_items,
    clippy::too_many_lines,
    clippy::unnecessary_trailing_comma,
    clippy::missing_panics_doc,
    reason = "yucky. clean that up."
)]
#![warn(
    clippy::todo,
    clippy::dbg_macro,
    // clippy::use_debug,
    reason = "finish your code"
)]
#![forbid(
    clippy::unwrap_used,
    clippy::missing_assert_message,
    reason = "never panic without a reason"
)]
#![warn(
    clippy::suspicious,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::indexing_slicing,
    clippy::string_slice,
    reason = "be careful about edge-cases"
)]
#![warn(clippy::disallowed_types)]
#![deny(
    clippy::allow_attributes,
    clippy::as_pointer_underscore,
    clippy::as_underscore,
    clippy::assertions_on_result_states,
    clippy::deref_by_slicing,
    clippy::doc_include_without_cfg,
    clippy::missing_asserts_for_indexing,
    clippy::non_zero_suggestions,
    clippy::pub_without_shorthand,
    clippy::precedence_bits,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::return_and_then,
    clippy::semicolon_inside_block,
    clippy::separated_literal_suffix,
    clippy::str_to_string,
    clippy::string_lit_chars_any,
    clippy::try_err,
    clippy::unused_trait_names,
    unused_imports,
    clippy::redundant_closure,
    clippy::redundant_closure_call,
    clippy::redundant_closure_for_method_calls,
    reason = "consistency or personal preference"
)]
#![allow(
    clippy::struct_excessive_bools,
    clippy::unnecessary_debug_formatting,
    // usually, if I use `Default::default()` instead of the type name, it's because the typename is LESS readable
    clippy::default_trait_access,
    clippy::deprecated_attributes_without_since,
    // if im using a glob import, it's because im using all of them (or so many that it's pointless)
    clippy::enum_glob_use, clippy::wildcard_imports,
    // trying to avoid including crates
    clippy::naive_bytecount,
    reason = "these lints make sweeping assumptions that are often wrong"
)]
#![forbid(
    clippy::doc_broken_link,
    reason = "fix this before you forget what it means!!"
)]
#![allow(
    clippy::nonminimal_bool,
    clippy::overly_complex_bool_expr,
    reason = "expensive, only enable sometimes"
)]

use crate::{
    analyzer::static_env::scope::fmt::DisplayHir,
    error::Diagnostic,
    highlight::{
        TokenHighlight, highlight,
        style::{Color, Style, StyleWrapper},
        syntax::{Syntax, SyntaxStyle, syntax_style},
    },
    parse::ast::{ProgramUnit, VisitSpans as _},
    src_code::{
        code::{CreateSourceCodeError, SourceCode, SourceLoc, eq_module_name},
        map::SourceMap,
        span::RelSpan,
    },
    todo_types::Dummy,
    util::OrAllErrsExt as _,
};
use analyzer::static_env::scope::env::ScopeEnvironment;
use error::{CheckDiagnostics, fail::Failure};
use src_code::{
    code::ExpansionData,
    span::{FileId, usize_to_uspan},
};
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
};

mod analyzer;
mod arrayvec;
mod error;
mod highlight;
mod lex;
mod parse;
mod parser;
mod pre;
mod regex;
mod spec_ops;
mod src_code;
mod todo_types;
mod types;
mod util;

#[cfg(test)] // only include testing module in test builds
mod tests;

/// The style table currently being used
// TODO: make this configurable by file(?)
pub const SYNTAX_STYLE_ANSI: SyntaxStyle<Style, [Style; 3]> = syntax_style! {
    normal: {},
    comment: {
        foreground: Rgb(0x6a, 0x99, 0x55)
    },
    dimmed: {
        foreground: BrightBlack
    },
    number_literal: {
        foreground: Rgb(0xb5, 0xce, 0xa8)
    },
    char_literal: {
        foreground: Rgb(0xce, 0x91, 0x78)
    },
    string_literal: {
        foreground: Rgb(0xce, 0x91, 0x78)
    },
    escape_seq: {
        foreground: Rgb(0xd7, 0xba, 0x7d)
    },
    language_defined: {
        foreground: Rgb(0x56, 0x9c, 0xd6)
    },
    language_variable: {
        foreground: Rgb(0x56, 0x9c, 0xd6),
        underline: true
    },
    label: {
        foreground: Rgb(0x56, 0x9c, 0xd6)
    },
    variable: {
        foreground: Rgb(0x9c, 0xdc, 0xfe),
        underline: true
    },
    field: {
        foreground: Rgb(0x9c, 0xdc, 0xfe),
        underline: true
    },
    param: {
        foreground: Rgb(0x9c, 0xdc, 0xfe),
        underline: true
    },
    constant: {
        foreground: Rgb(0x4f, 0xc1, 0xff),
        underline: true
    },
    discriminant: {
        foreground: Rgb(0x4f, 0xc1, 0xff),
        underline: true
    },
    variant: {
        foreground: Rgb(0x4f, 0xc1, 0xff),
        underline: true
    },
    callable: {
        foreground: Rgb(0xdc, 0xdc, 0xaa),
        underline: true
    },
    keyword: {
        foreground: Rgb(0x56, 0x9c, 0xd6)
    },
    flow_keyword: {
        foreground: Rgb(0xc5, 0x86, 0xc0)
    },
    primitive: {
        foreground: Rgb(0x4e, 0xc9, 0xb0)
    },
    typename: {
        foreground: Rgb(0x4e, 0xc9, 0xb0),
        underline: true
    },
    modname: {
        foreground: Rgb(0x4e, 0xc9, 0xb0)
    },
    macro_name: {
        foreground: Rgb(0x56, 0x9c, 0xd6),
        underline: true
    },
    macro_param: {
        foreground: Rgb(0x4f, 0xc1, 0xff),
        underline: true
    },
    bracket: [{
        foreground: Rgb(0xff, 0xd7, 0x00)
    }, {
        foreground: Rgb(0xda, 0x70, 0xd6)
    }, {
        foreground: Rgb(0x17, 0x9f, 0xff)
    }],
    invalid: {
        foreground: BrightRed
    }
};

/// # Panics
/// This function may panic if `pos` is not a valid index in `src`
#[must_use]
pub fn last_ansi_seq(src: &str, pos: usize) -> &str {
    let pre = src
        .get(..pos)
        .expect("pos should not be within a UTF-8 character");
    pre
        .rfind("\x1b[")
        .and_then(|pos| {
            #[expect(clippy::string_slice, reason = "rfind should return a valid position within src. \
                pre only shortens the end, not the start, so pos should still be a valid start position.")]
            let s = &src[pos..];
            s.split_inclusive('m').next()
        })
        .unwrap_or("\x1b[0m")
}

fn print_highlighted<'src, I, T, A>(
    source: &SourceMap,
    code: &SourceCode,
    tokens: I,
    expansions: &[ExpansionData],
    syntax_style: &SyntaxStyle<T, A>,
) where
    I: IntoIterator<Item: TokenHighlight<'src>>,
    T: StyleWrapper,
    A: AsRef<[T]>,
{
    use std::fmt::Write as _;
    let mut buf = String::new();
    let tokens: Vec<_> = tokens.into_iter().map(|x| x.get_syntax()).collect();
    for (lexeme, syntax) in highlight(tokens.iter().copied()) {
        _ = write!(buf, "{}", syntax_style[syntax].style(lexeme));
    }
    _ = write!(buf, "\x1b[0m");
    let line_num_width = buf
        .split('\n')
        .count()
        .strict_add(' '.len_utf8())
        .to_string()
        .len();
    for (i, (highlighted_line, line_span)) in buf.lines().zip(code.line_content_spans()).enumerate()
    {
        let line_range = buf
            .substr_range(highlighted_line)
            .expect("lines should be substrings of buf");
        let last_ansi_seq = last_ansi_seq(&buf, line_range.start);
        println!(
            " \x1b[90m{:>line_num_width$}{last_ansi_seq}   {highlighted_line}",
            i.strict_add(1)
        );
        let mut has_annotation = false;
        for x in expansions {
            const MSG_STYLE: Style = Style::new().foreground(Color::BrightCyan).bold(true);
            if let Some(span) = line_span.intersection(x.exp) {
                has_annotation = true;
                print!(" {}{:>line_num_width$}   ", MSG_STYLE.begin(), "");
                for _ in line_span.start..span.0.start {
                    print!(" ");
                }
                if span.is_empty() {
                    print!("↖ (empty)");
                } else {
                    for _ in span.0 {
                        print!("~");
                    }
                }
                print!(" expanded from `\x1b[0m");
                let call_code = &source[x.call.file];
                for (lexeme, syntax) in highlight(
                    call_code
                        .span_tokens(x.call.rel)
                        .map(|(span, kind)| (&call_code[span], kind)),
                ) {
                    print!("{}", syntax_style[syntax].style(lexeme));
                }
                print!(
                    "\x1b[0m{}` <--\x1b[0m {}",
                    MSG_STYLE.begin(),
                    x.call.display_loc(source)
                );
                print!(" {}as defined in `\x1b[0m", MSG_STYLE.begin());
                let def_code = &source[x.def.file];
                for (lexeme, syntax) in highlight(
                    def_code
                        .span_tokens(x.def.rel)
                        .map(|(span, kind)| (&def_code[span], kind)),
                ) {
                    print!("{}", syntax_style[syntax].style(lexeme));
                }
                println!(
                    "\x1b[0m{}` <--\x1b[0m {}",
                    MSG_STYLE.begin(),
                    x.def.display_loc(source)
                );
            }
        }
        if has_annotation {
            println!(); // breathing room
        }
    }
    print!("\x1b[0m");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    UnknownOption {
        prgm: PathBuf,
        option: std::ffi::OsString,
    },
    TooManyArgs {
        prgm: PathBuf,
        unexpected: std::ffi::OsString,
    },
    MissingValue {
        option: std::ffi::OsString,
    },
    ParseError(std::str::ParseBoolError),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOption { prgm, option } => {
                writeln!(f, "unknown option {:?}\n", option.display())?;
                Cli::usage(&prgm.display(), f)
            }
            Self::TooManyArgs { prgm, unexpected } => {
                writeln!(f, "unexpected argument {:?}\n", unexpected.display())?;
                Cli::usage(&prgm.display(), f)
            }
            Self::MissingValue { option } => {
                writeln!(f, "option {:?} is missing a value", option.display())
            }
            Self::ParseError(e) => writeln!(f, "could not parse argument: {e}"),
        }
    }
}

impl std::error::Error for CliError {}

macro_rules! CliParser {
    (
        $(#[$sm:meta])*
        $vis:vis struct $Struct:ident {$(
            $(#[doc = $fdoc:expr])* // fun fact: comments CAN affect execution!
            $(#[option($name:expr)])?
            $fvis:vis $field:ident: $Type:ty
        ),* $(,)?}
    ) => {
        $(#[$sm])*
        $vis struct $Struct {$(
            $(#[doc = $fdoc])*
            $fvis $field: $Type
        ),*}

        static HELP: std::sync::LazyLock<[(&str, &str); [$(concat!($($fdoc),*)),*].len()]> = std::sync::LazyLock::new(|| [$(
            (stringify!($field), concat!($($fdoc, " "),*).trim())
        ),*]);

        static OPTIONS: std::sync::LazyLock<[(&str, &str); [$($($name,)?)*].len()]> = std::sync::LazyLock::new(|| [$($(
            (concat!("--", $name), stringify!($field)),
        )?)*]);

        impl $Struct {
            const fn get_option_mut(&mut self, name: &str) -> Option<&mut bool> {
                match name {
                    $($(concat!("--", $name) => Some(&mut self.$field),)?)*
                    _ => None,
                }
            }
        }
    };
}

CliParser! {
    // #[expect(
    //     clippy::struct_excessive_bools,
    //     reason = "no, you're wrong. they're flags."
    // )]
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Cli {
        prgm: PathBuf,

        /// Echo the source code
        #[option("echo-src")]
        echo_src: bool,

        /// Display a list of initial scanner tokens and their ranges
        #[option("scanner-tokens")]
        scanner_tokens: bool,
        /// Echo the source code with basic syntax highlighting
        #[option("scanner-highlight")]
        scanner_highlight: bool,

        /// Display a list of preprocessed tokens and their ranges
        #[option("preproc-tokens")]
        preproc_tokens: bool,
        /// Echo the preprocessed source code with basic syntax highlighting
        #[option("preproc-highlight")]
        preproc_highlight: bool,

        /// Display the abstract syntax tree
        #[option("dbg-ast")]
        dbg_ast: bool,
        /// Echo the original source code with semantic highlighting from the AST
        #[option("semantic-highlight")]
        semantic_highlight: bool,

        /// Perform evaluation of the code
        #[option("do-eval")]
        do_eval: bool,
        /// Echo the expression being evaluated before outputting its result (requires `do_eval` to have any effect)
        #[option("echo-exprs")]
        echo_exprs: bool,

        /// Print errors instead of just ending
        #[option("print-errors")]
        print_errors: bool,

        /// The source files used as the roots of each library.
        /// If there is a folder with the same name as one of these files,
        /// all files within that folder will also be included.
        source_paths: Vec<PathBuf>,
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            prgm: PathBuf::default(),
            source_paths: Vec::new(),
            echo_src: false,
            scanner_tokens: false,
            scanner_highlight: true,
            preproc_tokens: false,
            preproc_highlight: true, // TODO: should not be default!
            dbg_ast: false,
            semantic_highlight: true,
            do_eval: true,
            echo_exprs: false,
            print_errors: true,
        }
    }
}

impl Cli {
    pub fn usage<S>(prgm: &S, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        S: ?Sized + std::fmt::Display,
    {
        writeln!(f, "usage: {prgm} [OPTIONS ...] [PATH]\n\noptions:")?;
        let widest = OPTIONS
            .iter()
            .map(|item| item.0.len())
            .max()
            .expect("should have at least one option");
        for (opt, field) in &*OPTIONS {
            let help = HELP
                .iter()
                .find(|(hfield, _)| hfield == field)
                .map_or_default(|(_, help)| *help);
            writeln!(f, "  {opt:<widest$}  {help}")?;
        }
        Ok(())
    }

    pub fn parse_from_env() -> Result<Self, CliError> {
        let mut args = std::env::args_os().peekable();
        let mut config = Self {
            prgm: PathBuf::from(
                args.next()
                    .expect("must have a program argument to be running"),
            ),
            ..Default::default()
        };
        while let Some(arg) = args.next_if(|arg| arg.to_str().is_some_and(|s| s.starts_with("--")))
        {
            let s = arg.to_str().expect("guarded by next_if");
            let value: bool = match s.split_once('=') {
                Some(s) => s.1.parse().map_err(CliError::ParseError)?,
                None => match args.next() {
                    Some(s) => match s.to_str() {
                        Some(s) => s.parse().map_err(CliError::ParseError)?,
                        None => return Err(CliError::MissingValue { option: arg }),
                    },
                    None => return Err(CliError::MissingValue { option: arg }),
                },
            };
            if let Some(option) = config.get_option_mut(s) {
                *option = value;
            } else {
                return Err(CliError::UnknownOption {
                    prgm: config.prgm,
                    option: arg,
                });
            }
        }
        config.source_paths.extend(args.map(PathBuf::from));
        Ok(config)
    }
}

trait ContainsError {
    fn contains_err(&self) -> bool;

    #[inline]
    fn ok_or_err(&self) -> Result<(), CheckDiagnostics> {
        if self.contains_err() {
            Err(CheckDiagnostics)
        } else {
            Ok(())
        }
    }
}

impl ContainsError for [Diagnostic] {
    fn contains_err(&self) -> bool {
        self.iter().any(|e| e.kind.severity().is_err())
    }
}

fn comp_file(
    config: &Cli,
    source: &mut SourceMap,
    id: FileId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<ProgramUnit, CheckDiagnostics> {
    // scanner
    println!("scanning...");
    {
        let code = &mut source[id];
        code.set_tokens(lex::lex(id, code.content(), diagnostics));
    }

    let code = &source[id];

    // syntax highlighted
    if config.scanner_highlight {
        println!("\n\x1b[0m{}:", code.loc());
        print_highlighted(
            source,
            code,
            code.tokens().iter().map(|(rel, kind)| (&code[rel], kind)),
            code.expansions(),
            &SYNTAX_STYLE_ANSI,
        );
        println!();
    }

    // lex errors
    diagnostics.ok_or_err()?;

    // TODO: how to prevent REPL from generating preprocessed copies every time?

    // preprocessing
    println!("preprocessing...");
    let preprocessed = pre::preprocess(id, code, source, diagnostics);

    if let Some(pp) = preprocessed {
        let og_id = pp
            .loc()
            .original()
            .expect("preprocessor should only output preprocessed files");
        let pp_id = source.push(pp);
        source[og_id].set_preproc(pp_id);

        // preprocessor syntax highlighted
        if config.preproc_highlight {
            let code = &source[pp_id];
            println!("\n\x1b[0m{}:", code.loc());
            print_highlighted(
                source,
                code,
                code.tokens().iter().map(|(rel, kind)| (&code[rel], kind)),
                code.expansions(),
                &SYNTAX_STYLE_ANSI,
            );
            println!();
        }
    } else {
        println!("no preprocessor output");
    }

    // preprocesor errors
    diagnostics.ok_or_err()?;

    // use the preprocessed version from now on
    let id = source[id].preproc().copied().unwrap_or(id);

    // parse
    println!("parsing...");
    let code = &source[id];
    let Ok(unit) = parse::parse_unit(
        id,
        usize_to_uspan(code.content().len()),
        code.tokens().iter(),
        diagnostics,
    ) else {
        debug_assert!(
            diagnostics.contains_err(),
            "should have an error on failure"
        );
        return Err(CheckDiagnostics);
    };

    // parse errors
    diagnostics.ok_or_err()?;

    // // lisp debug
    // println!(
    //     "lisp for {}:\n{:#}",
    //     code.loc(),
    //     parse::fmt::Lisp {
    //         source: code,
    //         node: &unit
    //     }
    // );

    // semantic highlighted
    if config.scanner_highlight {
        println!();
        let id = unit.file;
        let code = &source[id];
        println!("\x1b[0m{}:", code.loc());
        let mut tokens = Vec::with_capacity(code.tokens().tokens.len());
        let mut skipped_tokens = code.tokens().iter().peekable();
        let mut f = {
            let tokens = &mut tokens;
            let mut prev_end = 0;
            move |span: RelSpan, syn: Syntax| {
                #[cfg(debug_assertions)]
                let mut curr_end = span.start; // if no tokens are visited, default to the value that is legitimate
                while skipped_tokens
                    .next_if(|(x, _)| x.end <= prev_end)
                    .inspect(|(x, _)| {
                        #[cfg(debug_assertions)]
                        (curr_end = x.end);
                    })
                    .is_some()
                {
                    // ignore; these were already handled by visit_spans
                }
                tokens.extend(
                    std::iter::from_fn(|| skipped_tokens.next_if(|(x, _)| x.end <= span.start))
                        .inspect(|(x, _)| {
                            #[cfg(debug_assertions)]
                            (curr_end = x.end);
                        })
                        .map(|(span, kind)| (&code[span], kind).get_syntax()),
                );
                #[cfg(debug_assertions)]
                debug_assert_eq!(curr_end, span.start, "missed some token(s)");
                prev_end = span.end;
                tokens.push((&code[span], syn));
            }
        };
        for decl in &unit.decls {
            decl.visit_spans(0, &mut f);
        }
        print_highlighted(source, code, tokens, code.expansions(), &SYNTAX_STYLE_ANSI);
        println!();
    }

    Ok(unit)
}

fn compile(
    config: &Cli,
    source: &mut SourceMap,
    diagnostics: &mut Vec<Diagnostic>,
    scope_env: &mut ScopeEnvironment,
) -> Result<Dummy, CheckDiagnostics> {
    println!("--- per-file ---");

    let mut ids = source.iter().map(|(id, _)| id).collect::<VecDeque<_>>();
    let mut file_mod_ids = BTreeMap::new();
    let mut units = Vec::new();
    while let Some(id) = ids.pop_front() {
        let loc = source[id].loc();
        println!(
            "processing file {loc} (mod {})",
            loc.name().unwrap_or("<REPL>")
        );
        if let Ok(unit) = comp_file(config, source, id, diagnostics) {
            let code = &source[unit.file];
            let unit = units.push_mut(unit);
            println!("searching for dependencies...");
            match file_mod_ids.entry(id).or_try_insert_with(|| {
                scope_env.decl_library(
                    code.loc()
                        .name()
                        .unwrap_or_else(|| todo!("REPL mode library name"))
                        .to_owned(),
                )
            }) {
                Ok(&mut mod_id) => {
                    let submods = scope_env.declared_submodules(code, mod_id, unit, diagnostics);
                    if !submods.is_empty() {
                        if let Some(parent) = code.loc().path() {
                            let dir = parent.with_extension("");
                            for (name, _id) in submods {
                                println!("loading submodule {name}...");
                                match load_module(dir.as_path(), name.as_str()) {
                                    Ok(submod) => {
                                        ids.push_back(source.push(submod));
                                    }

                                    Err(e) => todo!("error loading module: {e}"),
                                }
                            }
                        } else {
                            todo!("error: REPL cannot have modules");
                        }
                    }
                }

                Err(e) => todo!("error: {e:?}"),
            }
        } else {
            // accumulate errors in other units
        }
    }
    diagnostics.ok_or_err()?;

    println!("--- linking ---");

    // declaration analysis
    println!("analyzing declarations...");
    let units = units
        .into_iter()
        .map(|unit| {
            println!("analyzing {:?}", unit.file);
            let &mod_id = file_mod_ids
                .get(&unit.file)
                .expect("should have been defined by `analyze_module_tree`");
            let mut env = scope_env
                .define_module(mod_id)
                .expect("should have been defined by `analyze_module_tree`");
            let res = env.analyze(unit, source, diagnostics);
            if let Ok(hir) = &res {
                println!(
                    "hir:\n{:#}",
                    DisplayHir {
                        source,
                        env: scope_env,
                        hir,
                    }
                );
            } else {
                debug_assert!(
                    diagnostics.contains_err(),
                    "should have an error if analysis failed"
                );
            }
            res
        })
        .collect_or_drop_all_errs::<Vec<_>>();

    // declaration errors
    diagnostics.ok_or_err()?;

    let units = units.expect("should have an output if there are no errors");

    println!("analyzing scope...");
    let units = units
        .into_iter()
        .map(|hir| {
            println!("linking {:?}", hir.file);
            let &mod_id = file_mod_ids
                .get(&hir.file)
                .expect("should have been defined by `analyze_module_tree`");
            let mut env = scope_env
                .define_module(mod_id)
                .expect("should have been defined by `analyze_module_tree`");
            let res = env.link(hir, source, diagnostics);
            if let Ok(hir) = &res {
                println!(
                    "linked hir:\n{:#}",
                    DisplayHir {
                        source,
                        env: scope_env,
                        hir,
                    }
                );
            } else {
                debug_assert!(
                    diagnostics.contains_err(),
                    "should have an error if analysis failed"
                );
            }
            res
        })
        .collect_or_drop_all_errs::<Vec<_>>();

    // link errors
    diagnostics.ok_or_err()?;

    let units = units.expect("should have an output if there are no errors");

    // -------------------------------------------------------------------------

    // let (program, env) = match analyze(source, &prgm) {
    //     Ok(prgm) => prgm,
    //     Err(errs) => {
    //         // static analysis errors
    //         let any_err = if config.print_errors {
    //             println!();
    //             list_errors("static analysis", errs)
    //         } else {
    //             errs.into_iter().next().is_some()
    //         };
    //         debug_assert!(
    //             any_err,
    //             "static analysis should only error if there is an error (duh)"
    //         );
    //         return;
    //     }
    // };

    // // println!("static analysis: {program:#?}");

    // // execution
    // if config.do_eval {
    //     let outer_style = Style::new().foreground(highlight::style::Color::BrightBlack);
    //     println!(
    //         "\n{}running...\n----------{}",
    //         outer_style.begin(),
    //         outer_style.end()
    //     );
    //     let res = interpret(&program, env);
    //     if let Err(e) = res
    //         && config.print_errors
    //     {
    //         let style = SYNTAX_STYLE_ANSI.invalid;
    //         println!(
    //             "{}----------{}\n{}unhandled runtime error: {e}{}",
    //             outer_style.begin(),
    //             outer_style.end(),
    //             style.begin(),
    //             style.end()
    //         );
    //     } else {
    //         println!(
    //             "{}----------\nfinished{}\n",
    //             outer_style.begin(),
    //             outer_style.end()
    //         );
    //     }
    // }

    // any remaining errors
    diagnostics.ok_or_err()?;
    println!("--- compilation successful ---");
    Ok(Dummy)
}

fn run_program(_prgm: &Dummy) -> Result<(), Failure> {
    todo!("runtime")
}

fn pop_trailing_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn compile_and_run(source_map: &mut SourceMap, config: &Cli, diagnostics: &mut Vec<Diagnostic>) {
    let mut scope_env = ScopeEnvironment::new();
    let res = compile(config, source_map, diagnostics, &mut scope_env);

    for e in diagnostics.iter() {
        println!("{}\n", e.contextualize(source_map));
    }
    diagnostics.clear();

    if let Ok(prgm) = res {
        println!();
        if let Err(e) = run_program(&prgm) {
            eprintln!("runtime error: {e}");
        }
    }
}

#[derive(Debug)]
enum LoadFileError {
    InvalidModName,
    CreateCode(CreateSourceCodeError),
    Io(std::io::Error),
}

impl std::fmt::Display for LoadFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidModName => write!(
                f,
                "invalid module name: must be a valid identifier after sanitizing"
            ),
            Self::CreateCode(e) => write!(f, "failed to read source code: {e}"),
            Self::Io(e) => write!(f, "failed to read source code file: {e}"),
        }
    }
}

fn load_file(path: PathBuf) -> Result<SourceCode, LoadFileError> {
    let mut source = std::fs::read_to_string(&path).map_err(LoadFileError::Io)?;
    let loc = SourceLoc::from_file(path).ok_or(LoadFileError::InvalidModName)?;
    pop_trailing_newline(&mut source);
    SourceCode::new(loc, source).map_err(LoadFileError::CreateCode)
}

#[derive(Debug)]
enum LoadModError<'name> {
    NoMatch(&'name str),
    MultipleMatches(&'name str, [PathBuf; 2]),
    Fs(LoadFileError),
}

impl std::fmt::Display for LoadModError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMatch(name) => write!(f, "no file matching the name {name:?} exists"),
            Self::MultipleMatches(name, [a, b]) => write!(
                f,
                "multiple files matching the name {name:?} exist, including {} and {}",
                a.display(),
                b.display()
            ),
            Self::Fs(e) => e.fmt(f),
        }
    }
}

fn load_module<'name>(dir: &Path, name: &'name str) -> Result<SourceCode, LoadModError<'name>> {
    debug_assert!(dir.is_dir(), "dir should be a directory");
    let dir = dir
        .read_dir()
        .map_err(LoadFileError::Io)
        .map_err(LoadModError::Fs)?;
    let mut it = dir
        .flatten()
        .map(|ent| ent.path())
        .filter(|path| eq_module_name(path, name))
        .inspect(|path| {
            debug_assert!(path.is_file(), "expected file_stem to ensure only files");
            debug_assert!(path.exists(), "expected read_dir to ensure only existing");
        });
    let path = it.next().ok_or(LoadModError::NoMatch(name))?;
    if let Some(dupe) = it.next() {
        return Err(LoadModError::MultipleMatches(name, [path, dupe]));
    }
    load_file(path).map_err(LoadModError::Fs)
}

fn main() {
    let config = match Cli::parse_from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    let mut source_map = SourceMap::new();
    // TODO: what if `builtin` is an actual directory?
    source_map.push(
        SourceCode::new(
            SourceLoc::from_file(PathBuf::from("builtin").join("std").with_extension("bats"))
                .expect("'std' should be a valid module name"),
            include_str!("../builtin/std.bats").to_owned(),
        )
        .expect("standard library should not be valid"),
    );

    let mut diagnostics = Vec::new();
    if config.source_paths.is_empty() {
        // interactive
        loop {
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("failed to obtain input");
            pop_trailing_newline(&mut input);
            if matches!(input.trim(), "exit" | "quit") {
                break; // finish
            }
            match SourceCode::new(SourceLoc::Repl, input) {
                Ok(code) => {
                    source_map.push(code);
                }
                Err(e) => eprintln!("failed to read source code: {e}"),
            }

            compile_and_run(&mut source_map, &config, &mut diagnostics);
        }
    } else {
        // from file
        for source_path in &config.source_paths {
            match load_file(source_path.clone()) {
                Ok(code) => {
                    source_map.push(code);
                }
                Err(e) => eprintln!("error: {e}"),
            }
        }
        compile_and_run(&mut source_map, &config, &mut diagnostics);
    }
}
