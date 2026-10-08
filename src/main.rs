//! # Batscript
//!
//! This project is not, and will not ever be, written with the help of any form of generative AI.
//! I do not like generative AI. I do not support it. It is a net negative on society and harms learning.

#![allow(unused_features)]
#![feature(
    char_to_u32,
    ascii_char,
    try_from_int_error_kind, // used in number literal error
    iter_next_chunk,
    deque_extend_front,
    debug_closure_helpers,
    const_destruct,
    const_trait_impl,
    const_convert,
    const_array,
    const_bool,
    const_clone,
    const_cmp,
    const_control_flow,
    const_default,
    const_iter,
    const_index,
    const_for,
    const_format_args,
    const_ops,
    const_option_ops,
    const_range,
    const_range_bounds,
    derive_const,
    min_adt_const_params,
    more_float_constants,
    adt_const_params,
    const_closures,
    unboxed_closures,
    fn_traits,
    allocator_api,
    split_array,
    pattern,
    deref_patterns,
    slice_pattern,
    pattern_type_range_trait,
    deref_pure_trait,
    macro_derive,
    macro_attr,
    impl_trait_in_assoc_type,
    impl_trait_in_bindings,
    impl_trait_in_fn_trait_return,
    type_alias_impl_trait,
    anonymous_lifetime_in_impl_trait,
    associated_type_defaults,
)]
#![forbid(
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    clippy::correctness, // actually mandates correctness
    reason = "write sound code"
)]
#![deny(
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
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::indexing_slicing,
    clippy::string_slice,
    reason = "be careful about edge-cases"
)]
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
    error::ContextError,
    eval::{RunValue, evaluate},
    grammar::{
        ast::{Binary, Expr, Grouping, OrType, TypeExpr, Unary},
        ast_iter::semantic,
        fmt::Lisp,
        parse,
    },
    highlight::{
        GenericError, TokenHighlight, highlight,
        style::{Style, StyleWrapper},
        syntax::{Syntax, SyntaxStyle, syntax_style},
    },
    preproc::preprocess,
    scanner::{
        token::{
            Token,
            value::{CharLiteral, LexValue, StrLiteral},
        },
        tokenize,
    },
};
use std::{path::PathBuf, range::Range};

mod arrayvec;
mod error;
mod eval;
mod grammar;
mod highlight;
mod preproc;
mod regex;
mod scanner;

#[cfg(test)] // only include testing module in test builds
mod test;

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
    variable: {
        foreground: Rgb(0x9c, 0xdc, 0xfe),
        underline: true
    },
    constant: {
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
    ctrl_keyword: {
        foreground: Rgb(0xc5, 0x86, 0xc0)
    },
    typename: {
        foreground: Rgb(0x4e, 0xc9, 0xb0)
    },
    macro_name: {
        foreground: Rgb(0x56, 0x9c, 0xd6),
        underline: true
    },
    macro_arg: {
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

pub fn print_ast(node: &Expr<'_>, indent: usize, br_depth: usize) {
    fn header(name: &str) {
        println!("\x1b[94m{name}:\x1b[0m");
    }

    fn field(name: &str, indent: usize) -> usize {
        const INDENT_BY: &str = "  ";
        print!("{:>indent$}{INDENT_BY}\x1b[90m{name}:\x1b[0m ", "");
        indent.strict_add(INDENT_BY.len())
    }

    match node {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;

            header("Binary");

            let field_indent = field("lhs", indent);
            print_ast(lhs, field_indent, br_depth);

            field(" op", indent);
            println!("{}", SYNTAX_STYLE_ANSI[op.syntax()].style_dbg(op));

            let field_indent = field("rhs", indent);
            print_ast(rhs, field_indent, br_depth);
        }

        Expr::Unary(inner) => {
            let Unary { op, operand, side } = &**inner;

            header("Unary");

            field(" op", indent);
            println!("{}", SYNTAX_STYLE_ANSI[op.syntax()].style_dbg(op));

            let field_indent = field(side.as_str(), indent);
            print_ast(operand, field_indent, br_depth);
        }

        Expr::Literal(token) => {
            header("Literal");

            field("token", indent);
            println!("{}", SYNTAX_STYLE_ANSI[token.syntax()].style_dbg(token));
        }

        Expr::Grouping(inner) => {
            let Grouping { open, expr, close } = &**inner;
            let style = SYNTAX_STYLE_ANSI[Syntax::Bracket(br_depth)];

            header("Grouping");

            field(" open", indent);
            println!("{}", style.style_dbg(open));

            let field_indent = field(" expr", indent);
            print_ast(expr, field_indent, br_depth.strict_add(1));

            field("close", indent);
            println!("{}", style.style_dbg(close));
        }

        Expr::Type(inner) => {
            let TypeExpr { name, or_ty } = inner;
            header("Type");

            field("name", indent);
            if let Some(OrType { pipe, ty }) = or_ty {
                println!(
                    "{} {} {}",
                    SYNTAX_STYLE_ANSI[name.syntax()].style_dbg(name),
                    SYNTAX_STYLE_ANSI[pipe.syntax()].style_dbg(pipe),
                    SYNTAX_STYLE_ANSI[ty.syntax()].style_dbg(ty)
                );
            } else {
                println!("{}", SYNTAX_STYLE_ANSI[name.syntax()].style_dbg(name));
            }
        }
    }
}

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
fn list_errors<'src, 'err, I>(errs: I) -> bool
where
    'src: 'err,
    I: IntoIterator<IntoIter: 'err, Item: ContextErrorOrRef<'src>>,
{
    println!("errors:");
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

fn print_highlighted<'src, I, T, A>(tokens: I, syntax_style: &SyntaxStyle<T, A>)
where
    I: IntoIterator<Item: TokenHighlight<'src>>,
    T: StyleWrapper,
    A: AsRef<[T]>,
{
    use std::fmt::Write;
    if false {
        // ---- DEBUG -----
        for (lexeme, syntax) in highlight(tokens) {
            print!("{}", syntax_style[syntax].style(lexeme));
        }
        // ----------------
        return;
    }
    let mut buf = String::new();
    for (lexeme, syntax) in highlight(tokens) {
        _ = write!(buf, "{}", syntax_style[syntax].style(lexeme));
    }
    _ = write!(buf, "\x1b[0m");
    println!("```");
    let line_num_width = buf
        .split('\n')
        .count()
        .strict_add(' '.len_utf8())
        .to_string()
        .len();
    for (i, line) in buf.lines().enumerate() {
        let Range { start, .. } = buf
            .substr_range(line)
            .expect("lines should be substrings of buf");
        let last_ansi_seq = last_ansi_seq(&buf, start);
        println!(
            " \x1b[90m{:>line_num_width$}{last_ansi_seq}   {line}",
            i.strict_add(1)
        );
    }
    println!("```");
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RunSyntax<'src> {
    Mono(Syntax),
    Char(CharLiteral),
    Text(StrLiteral<'src>),
}

impl Default for RunSyntax<'_> {
    fn default() -> Self {
        Self::Mono(Syntax::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RunToken<'src> {
    lex: &'src str,
    // TODO: Subtokens
    syn: RunSyntax<'src>,
}

impl<'src> TokenHighlight<'src> for RunToken<'src> {
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>) {
        match self.syn {
            // HACK: assumes highlighting ignores LexValue except for char and text
            RunSyntax::Mono(syn) => (self.lex, syn, LexValue::Whitespace),
            RunSyntax::Char(val) => (self.lex, Syntax::CharLiteral, LexValue::CharLiteral(val)),
            RunSyntax::Text(val) => (self.lex, Syntax::TextLiteral, LexValue::TextLiteral(val)),
        }
    }
}

fn runtime_token(value: RunValue, buf: &mut String) -> RunToken<'_> {
    use std::fmt::Write;
    match value {
        RunValue::None | RunValue::CoalesceNone => RunToken {
            lex: "none",
            syn: RunSyntax::Mono(Syntax::LanguageDefined),
        },
        RunValue::Fail(e) => RunToken {
            lex: {
                _ = write!(buf, "failed: {e}"); // infallible for String
                buf
            },
            syn: RunSyntax::Mono(Syntax::Invalid),
        },
        RunValue::Bool(x) => RunToken {
            lex: if x { "true" } else { "fals" },
            syn: RunSyntax::Mono(Syntax::LanguageDefined),
        },
        RunValue::UInt(n) => RunToken {
            lex: {
                *buf = n.to_string();
                buf
            },
            syn: RunSyntax::Mono(Syntax::NumberLiteral),
        },
        RunValue::SInt(n) => RunToken {
            lex: {
                *buf = n.to_string();
                buf
            },
            syn: RunSyntax::Mono(Syntax::NumberLiteral),
        },
        RunValue::Frac(x) => RunToken {
            lex: {
                *buf = x.to_string();
                buf
            },
            syn: RunSyntax::Mono(Syntax::NumberLiteral),
        },
        RunValue::Char(ch) => {
            _ = write!(buf, "{ch:?}"); // infallible for String
            RunToken {
                lex: buf,
                syn: RunSyntax::Char(CharLiteral {
                    ch,
                    is_escaped: buf.contains('\\'),
                }),
            }
        }
        RunValue::Text(s) => {
            _ = write!(buf, "{s:?}"); // infallible for String
            RunToken {
                lex: buf,
                syn: RunSyntax::Text(StrLiteral {
                    content: buf
                        .strip_circumfix('\"', '\"')
                        .expect("string debug should include delimiters"),
                }),
            }
        }
        RunValue::Type(t) => {
            _ = write!(buf, "{t:?}"); // infallible for String
            RunToken {
                lex: buf,
                syn: RunSyntax::Mono(Syntax::Typename),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CliError {
    UnknownOption {
        prgm: PathBuf,
        option: std::ffi::OsString,
    },
    TooManyArgs {
        prgm: PathBuf,
        unexpected: std::ffi::OsString,
    },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOption { prgm, option } => {
                writeln!(f, "unknown option {:?}", option.display())?;
                writeln!(f)?;
                Cli::usage(&prgm.display(), f)
            }
            Self::TooManyArgs { prgm, unexpected } => {
                writeln!(f, "unexpected argument {:?}", unexpected.display())?;
                writeln!(f)?;
                Cli::usage(&prgm.display(), f)
            }
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
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
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

        source_path: Option<PathBuf>,
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
                .map(|(_, help)| *help)
                .unwrap_or_default();
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
            if let Some(option) = config.get_option_mut(arg.to_str().expect("guarded by next_if")) {
                *option = true;
            } else {
                return Err(CliError::UnknownOption {
                    prgm: config.prgm,
                    option: arg,
                });
            }
        }
        config.source_path = args.next().map(PathBuf::from);
        if config.source_path.is_some()
            && let Some(unexpected) = args.next()
        {
            return Err(CliError::TooManyArgs {
                prgm: config.prgm,
                unexpected,
            });
        }
        Ok(config)
    }
}

/// # Panics
/// This method can panic if [`scanner::Scanner`] isn't written correctly
fn run_code(source: &str, config: &Cli) {
    // token debug
    if config.echo_src {
        println!("source code:\n```\n{source}\n```");
    }

    // scanner
    let tokens: Vec<_> = tokenize(source).collect();
    if config.scanner_tokens {
        println!("\ntokenizer:");
        print_tokens(source, &tokens);
    }

    // syntax highlighted
    if config.scanner_highlight {
        println!("\nsyntax highlighting:");
        print_highlighted(&tokens, &SYNTAX_STYLE_ANSI);
    }

    // lex errors
    if if config.print_errors {
        println!();
        list_errors(tokens.iter().map(Result::as_ref).filter_map(Result::err))
    } else {
        tokens.iter().any(Result::is_err)
    } {
        return;
    }

    // preprocessing
    let tokens: Vec<_> = preprocess(source, tokens).collect();
    if config.preproc_tokens {
        println!("\npreprocessor:");
        print_tokens(source, &tokens);
    }

    // preprocessed + syntax highlighted
    if config.preproc_highlight {
        print_highlighted(&tokens, &SYNTAX_STYLE_ANSI);
    }

    // preproc errors
    if if config.print_errors {
        println!();
        list_errors(tokens.iter().map(Result::as_ref).filter_map(Result::err))
    } else {
        tokens.iter().any(Result::is_err)
    } {
        return;
    }

    // parse debug
    let ast: Vec<_> = parse(source, tokens.into_iter().flatten()).collect();
    if config.dbg_ast {
        println!("\nparser:");
        for res in &ast {
            match res {
                Ok(node) => print_ast(node, 0, 0),
                Err(e) => println!("{}", SYNTAX_STYLE_ANSI.invalid.style_dbg(e)),
            }
        }
    }

    // parse errors
    if if config.print_errors {
        println!();
        list_errors(ast.iter().map(Result::as_ref).filter_map(Result::err))
    } else {
        ast.iter().any(Result::is_err)
    } {
        return;
    }

    // semantic highlighting
    // TODO: need to find a way to have this take Result instead of flattening
    if config.semantic_highlight {
        println!("\nsemantic highlighting:");
        print_highlighted(semantic(source, ast.iter().flatten()), &SYNTAX_STYLE_ANSI);
    }

    // eval
    if config.do_eval {
        println!("\nevaluation:");
        let mut errors = Vec::new();
        {
            let mut buf = String::new();
            for (expr, res) in ast
                .iter()
                .flatten()
                .map(|expr| (expr, evaluate(source, expr)))
            {
                if config.echo_exprs {
                    print!("{:#}\n  \x1b[90m=\x1b[0m ", Lisp::new(expr));
                }

                buf.clear();
                let item = match res {
                    Ok(x) => Ok(runtime_token(x, &mut buf)),
                    Err(e) => {
                        buf = format!("<{}>", e.code());
                        errors.push(e);
                        Err(GenericError(&buf))
                    }
                };

                for (lexeme, syntax) in highlight(std::iter::once(item)) {
                    print!("{}", SYNTAX_STYLE_ANSI[syntax].style(lexeme));
                }
                println!("\x1b[0m");
                if config.echo_exprs {
                    println!();
                }
            }
        }

        // eval errors
        if if config.print_errors {
            println!();
            list_errors(errors)
        } else {
            !errors.is_empty()
        } {
            #[expect(
                clippy::needless_return,
                reason = "should return here if more items follow this in the future"
            )]
            return;
        }
    }
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
    match &config.source_path {
        // interactive
        None => {
            let mut input = String::new();
            loop {
                input.clear();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("failed to obtain input");
                pop_trailing_newline(&mut input);
                if matches!(input.trim(), "exit" | "quit") {
                    break; // finish
                }
                run_code(&input, &config);
            }
        }

        // from file
        Some(source_path) => match std::fs::read_to_string(source_path) {
            Err(e) => eprintln!("failed to read source code file: {e}"),

            Ok(mut source) => {
                pop_trailing_newline(&mut source);
                run_code(&source, &config);
            }
        },
    }
}
