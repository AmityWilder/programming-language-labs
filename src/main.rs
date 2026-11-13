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
    impl_trait_in_fn_trait_return,
    macro_derive,
    deref_patterns,
    char_to_u32,
    extend_one,
    deref_pure_trait,
    const_ops,
    iterator_try_reduce,
    map_try_insert,
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
    reason = "could cause mistakes"
)]
#![warn(
    clippy::pedantic,
    clippy::missing_const_for_fn,
    // clippy::missing_docs_in_private_items,
    clippy::too_many_lines,
    reason = "yucky. clean that up."
)]
#![warn(clippy::todo, reason = "finish your code")]
#![warn(
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    clippy::missing_assert_message,
    reason = "avoid panics, or at least give a reason for them"
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
// #![warn(
//     clippy::expect_used,
//     clippy::panic,
//     reason = "avoid using `expect` or `panic` where things ought to be guaranteed \
//         (i.e. std library promises it). prefer builtin operators since their panics \
//         can be optimized away in release builds. use expect/panic when correctness \
//         depends on MYSELF writing it correctly."
// )]
// #![warn(unsafe_code)] // not actually a problem, just be very careful

use crate::{
    analyzer::static_env::scope::fmt::DisplayHir,
    error::Diagnostic,
    highlight::{
        TokenHighlight, highlight,
        style::{Color, Style, StyleWrapper},
        syntax::{SyntaxStyle, syntax_style},
    },
    old_error::ContextError,
    scanner::token::Token,
    src_code::{
        code::{SourceCode, SourceLoc},
        map::SourceMap,
    },
};
use analyzer::static_env::scope::env::ScopeEnvironment;
use error::fail::Failure;
use src_code::{code::ExpansionData, span::usize_to_uspan};
use std::path::PathBuf;

/// Indicates unfinished code.
///
/// This can be useful if you are prototyping and just
/// want a placeholder to let your code pass type analysis.
pub type Todo = !;

/// Indicates unfinished code.
///
/// This can be useful if you are prototyping and just
/// want a placeholder to let your code pass type analysis.
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Default)]
pub struct Dummy;

macro_rules! debug_info {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)] {
            println!("\x1b[90mINFO: {}\x1b[0m", format_args!($($args)*));
        }
    };
}
pub(crate) use debug_info;

macro_rules! debug_warn {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)] {
            println!("\x1b[93mWARNING: {}\x1b[0m", format_args!($($args)*));
        }
    };
}
pub(crate) use debug_warn;

macro_rules! debug_unsafe {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)] {
            println!("\x1b[95mUNSAFE: {}\x1b[0m", format_args!($($args)*));
        }
    };
}
pub(crate) use debug_unsafe;

mod analyzer;
mod arrayvec;
mod error;
// mod exec;
mod grammar;
mod highlight;
mod interp;
mod lex;
mod old_error;
mod parse;
mod parser;
mod pre;
mod preproc;
mod regex;
mod scanner;
mod spec_ops;
mod src_code;
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
    typename: {
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

trait ContextErrorOrRef<'src> {
    fn as_ref(&self) -> &ContextError<'src>;
}

impl<'src> ContextErrorOrRef<'src> for ContextError<'src> {
    fn as_ref(&self) -> &ContextError<'src> {
        self
    }
}

impl<'src> ContextErrorOrRef<'src> for &ContextError<'src> {
    fn as_ref(&self) -> &ContextError<'src> {
        self
    }
}

/// Print a list of all errors with clean formatting
///
/// Returns true if the iterator was not empty
#[must_use = "indicates that an error occurred"]
fn list_errors<'src, 'err, I>(title: &str, errs: I) -> bool
where
    'src: 'err,
    I: IntoIterator<IntoIter: 'err, Item: ContextErrorOrRef<'src>>,
{
    println!("{title} errors:");
    let mut any_errors = false;
    for e in errs {
        const INDENT: &str = "          ";
        let e = e.as_ref();
        print!(
            "  \x1b[91m{}:\x1b[0m {}\n{}    \x1b[92mhelp:\x1b[0m ",
            e.code(),
            e.err,
            e.render(),
        );
        let mut has_prev = false;
        for line in e.help().to_string().lines() {
            if has_prev {
                print!("{INDENT}");
            }
            println!("{line}");
            has_prev = true;
        }
        println!();
        any_errors = true;
    }
    if !any_errors {
        println!("  \x1b[92mnone\x1b[0m");
    }
    any_errors
}

/// Display the debug of tokens in a stream
fn print_tokens<'src: 'arr, 'arr, I>(source: &str, tokens: I)
where
    I: IntoIterator<Item = &'arr Result<Token<'src>, ContextError<'src>>>,
{
    let max_cols = source.lines().map(str::len).max().unwrap_or(0);
    let max_range_digits = max_cols.to_string().len().strict_mul(2);
    for item in tokens {
        let (_, syn, _) = item.get_syntax();
        let style = SYNTAX_STYLE_ANSI[syn];
        let range = match item {
            Ok(token) => token.lex_range(source),
            Err(e) => e.range,
        };
        print!("\x1b[90m{range:>max_range_digits$?}:\x1b[0m ");
        match item {
            Ok(token) => {
                println!("{}", style.style_dbg(token));
            }
            Err(e) => {
                let style = &SYNTAX_STYLE_ANSI.invalid;
                println!("{}", style.style_dbg(e));
            }
        }
    }
}

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
    use std::fmt::Write;
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
                let call_code = source.source_of(x.call);
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
                let def_code = source.source_of(x.def);
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

        source_path: Vec<PathBuf>,
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            prgm: PathBuf::default(),
            source_path: Vec::new(),
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
        config.source_path.extend(args.map(PathBuf::from));
        Ok(config)
    }
}

fn compile(source: &mut SourceMap, config: &Cli) -> (Result<Dummy, ()>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    // scanner
    for (id, file) in source.iter_mut() {
        file.set_tokens(lex::lex(id, file.content(), &mut diagnostics));
    }

    // syntax highlighted
    if config.scanner_highlight {
        println!();
        for (id, file) in source.iter() {
            println!("\x1b[0m{}:", source[id].loc());
            print_highlighted(
                source,
                file,
                file.tokens()
                    .iter()
                    .map(|(rel, kind)| (source.span_str(rel.within_file(id)), kind)),
                file.expansions(),
                &SYNTAX_STYLE_ANSI,
            );
            println!();
        }
    }

    // lex errors
    if diagnostics.iter().any(|e| e.kind.severity().is_err()) {
        return (Err(()), diagnostics);
    }

    // TODO: how to prevent REPL from generating preprocessed copies every time?

    // preprocessing
    let preprocessed = source
        .iter()
        .filter_map(|(id, file)| pre::preprocess(id, file, source, &mut diagnostics))
        .collect::<Vec<_>>();

    for code in preprocessed {
        let &(SourceLoc::PreprocRepl(og_id) | SourceLoc::PreprocFile(og_id, _)) = code.loc() else {
            unimplemented!("preprocessor should only output preprocessed files")
        };
        let pp_id = source.push(code);
        source
            .get_mut(og_id)
            .expect("preprocessor should reference legitimate, existing files")
            .set_preproc(pp_id);
    }
    // preprocessor syntax highlighted
    if config.preproc_highlight {
        println!();
        for (id, file) in source.iter() {
            println!("\x1b[0m{}:", source[id].loc());
            print_highlighted(
                source,
                file,
                file.tokens()
                    .iter()
                    .map(|(rel, kind)| (source.span_str(rel.within_file(id)), kind)),
                file.expansions(),
                &SYNTAX_STYLE_ANSI,
            );
            println!();
        }
    }

    // preprocesor errors
    if diagnostics.iter().any(|e| e.kind.severity().is_err()) {
        return (Err(()), diagnostics);
    }

    let prgm_units: Vec<_> = source
        .iter_mut()
        .filter(|(_, code)| {
            // only parse post-processed files, or files that have no preprocessor directives
            code.preproc().is_none()
                || matches!(
                    code.loc(),
                    SourceLoc::PreprocRepl(_) | SourceLoc::PreprocFile(_, _)
                )
        })
        .filter_map(|(id, code)| {
            parse::parse(
                id,
                usize_to_uspan(code.content().len()),
                code.tokens().iter(),
                &mut diagnostics,
            )
            .ok()
        })
        .collect();

    // parse errors
    if diagnostics.iter().any(|e| e.kind.severity().is_err()) {
        return (Err(()), diagnostics);
    }

    // lisp debug
    for unit in &prgm_units {
        let source = source
            .get(unit.file)
            .expect("program unit should have valid file ID");

        println!(
            "lisp for {}:\n{:#}",
            source.loc(),
            parse::fmt::Lisp { source, node: unit }
        );
    }

    // scope analysis
    let mut scope_env = ScopeEnvironment::new();
    for unit in prgm_units {
        println!("analyzing {:?}", unit.file);
        let code = source
            .get(unit.file)
            .expect("all program units should be valid");
        // TODO: submodules
        let name = code.loc().path().map_or_default(|path| {
            let name = path.file_stem().expect("must be a file");
            name.to_str()
                .unwrap_or_else(|| todo!("invalid characters in library name {name:?}"))
        });
        match scope_env.decl_library(name.to_string()) {
            Ok(id) => {
                let mut env = scope_env.define_library(id).expect("just declared");
                if let Ok(hir) = env.analyze(&source[unit.file], unit, &mut diagnostics) {
                    println!(
                        "hir:\n{:#}",
                        DisplayHir {
                            code,
                            env: &scope_env,
                            hir: &hir
                        }
                    );
                    // TODO
                } else {
                    debug_assert!(
                        diagnostics.iter().any(|e| e.kind.severity().is_err()),
                        "should have an error if analysis failed"
                    );
                }
            }
            Err(e) => {
                todo!("error declaring library {name}: {e:?}");
            }
        }
    }

    // scope errors
    if diagnostics.iter().any(|e| e.kind.severity().is_err()) {
        return (Err(()), diagnostics);
    }

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

    (Ok(Dummy), diagnostics)
}

fn run_program(prgm: &Dummy) -> Result<(), Failure> {
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

fn main() {
    let config = match Cli::parse_from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    let mut source_map = SourceMap::new();
    source_map.push(
        SourceCode::new(
            SourceLoc::File(PathBuf::from("builtin").join("std").with_extension("bats")),
            include_str!("batstd/std.bats").to_string(),
        )
        .expect("standard library should not be valid"),
    );
    loop {
        if config.source_path.is_empty() {
            // interactive
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("failed to obtain input");
            pop_trailing_newline(&mut input);
            if matches!(input.trim(), "exit" | "quit") {
                break; // finish
            }
            match SourceCode::new(SourceLoc::Repl, input) {
                Err(e) => eprintln!("failed to read source code: {e}"),

                Ok(code) => _ = source_map.push(code),
            }
        } else {
            // from file
            for source_path in &config.source_path {
                match std::fs::read_to_string(source_path) {
                    Err(e) => eprintln!("failed to read source code file: {e}"),

                    Ok(mut source) => {
                        pop_trailing_newline(&mut source);
                        match SourceCode::new(SourceLoc::File(source_path.clone()), source) {
                            Err(e) => eprintln!("failed to read source code file: {e}"),

                            Ok(code) => _ = source_map.push(code),
                        }
                    }
                }
            }
        }

        let (res, diagnostics) = compile(&mut source_map, &config);

        for e in diagnostics {
            println!("{}\n", e.contextualize(&source_map));
        }

        if let Ok(prgm) = res {
            println!();
            if let Err(e) = run_program(&prgm) {
                eprintln!("runtime error: {e}");
            }
        }

        if !config.source_path.is_empty() {
            break;
        }
    }
}
