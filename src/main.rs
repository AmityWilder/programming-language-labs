//! # Batscript
//!
//! This project is not, and will not ever be, written with the help of any form of generative AI.
//! I do not like generative AI. I do not support it. It is a net negative on society and harms learning.

#![allow(unused_features)]
#![feature(
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
    trim_prefix_suffix,
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
    grammar::{Binary, Expr, Grouping, Lisp, Unary, parse},
    highlight::{
        GenericError, TokenHighlight, highlight,
        style::{Style, StyleWrapper},
        syntax::{Syntax, SyntaxStyle, syntax_style},
    },
    preproc::preprocess,
    scanner::{
        token::{
            Token,
            keyword::Keyword,
            value::{CharLiteral, LexValue, StrLiteral},
        },
        tokenize,
    },
};
use std::range::Range;

mod error;
mod eval;
mod grammar;
mod highlight;
mod preproc;
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

fn runtime_token(value: RunValue, buf: &mut String) -> Token<'_> {
    match value {
        RunValue::None | RunValue::CoalesceNone => Token {
            lex: "none",
            val: LexValue::Keyword(Keyword::None),
            mac: None,
        },
        RunValue::Bool(x) => Token {
            lex: if x { "true" } else { "fals" },
            val: LexValue::BoolLiteral(x),
            mac: None,
        },
        RunValue::UInt(n) => Token {
            lex: {
                *buf = n.to_string();
                buf
            },
            val: LexValue::UIntLiteral(n),
            mac: None,
        },
        RunValue::SInt(n) => Token {
            lex: {
                *buf = n.to_string();
                buf
            },
            val: LexValue::SIntLiteral(n),
            mac: None,
        },
        RunValue::Frac(x) => Token {
            lex: {
                *buf = x.to_string();
                buf
            },
            val: LexValue::FracLiteral(x),
            mac: None,
        },
        RunValue::Char(ch) => {
            let lex = {
                use std::fmt::Write;
                // infallible for String
                _ = write!(buf, "{ch:?}");
                buf
            };
            Token {
                lex,
                val: LexValue::CharLiteral(CharLiteral {
                    ch,
                    is_escaped: lex.contains('\\'),
                }),
                mac: None,
            }
        }
        RunValue::Text(s) => {
            let lex = {
                use std::fmt::Write;
                // infallible for String
                _ = write!(buf, "{s:?}");
                buf
            };
            Token {
                lex,
                val: LexValue::TextLiteral(StrLiteral {
                    content: lex
                        .strip_circumfix('\"', '\"')
                        .expect("string debug should include delimiters"),
                }),
                mac: None,
            }
        }
    }
}

/// # Panics
/// This method can panic if [`scanner::Scanner`] isn't written correctly
pub fn run_code(source: &str) {
    // token debug
    println!("source code:\n```\n{source}\n```");

    // scanner
    println!("\ntokenizer:");
    let tokens: Vec<_> = tokenize(source).collect();
    print_tokens(source, &tokens);

    // syntax highlighted
    println!("\nsyntax highlighting:");
    print_highlighted(&tokens, &SYNTAX_STYLE_ANSI);

    // lex errors
    println!();
    if list_errors(tokens.iter().map(Result::as_ref).filter_map(Result::err)) {
        return;
    }

    // preprocessing
    println!("\npreprocessor:");
    let tokens: Vec<_> = preprocess(source, tokens).collect();
    print_tokens(source, &tokens);

    // preprocessed + syntax highlighted
    print_highlighted(&tokens, &SYNTAX_STYLE_ANSI);

    // preproc errors
    println!();
    if list_errors(tokens.iter().map(Result::as_ref).filter_map(Result::err)) {
        return;
    }

    // parse debug
    println!("\nparser:");
    let ast: Vec<_> = parse(source, tokens.into_iter().flatten()).collect();
    for res in &ast {
        match res {
            Ok(node) => print_ast(node, 0, 0),
            Err(e) => println!("{}", SYNTAX_STYLE_ANSI.invalid.style_dbg(e)),
        }
    }

    // parse errors
    println!();
    if list_errors(ast.iter().map(Result::as_ref).filter_map(Result::err)) {
        return;
    }

    // ----------------------------------------------
    // TODO: YUCKY! too many allocations and copies!!
    if false {
        // TODO: instead of this, just do the regular highlighting
        // but inject special highlighting over ranges from the AST data
        const NEWLINE: Token = Token {
            lex: "\n",
            val: scanner::token::value::LexValue::Whitespace,
            mac: None,
        };
        println!("\nsemantic highlighting (EXPERIMENTAL):");
        print_highlighted(
            ast.iter()
                .flatten()
                .flat_map(|root| grammar::AstIter::new(root).chain(std::iter::once(&NEWLINE))),
            &SYNTAX_STYLE_ANSI,
        );
    }
    // ----------------------------------------------

    // eval
    println!("\nevaluation:");
    let mut errors = Vec::new();
    let mut buf = String::new();
    for (expr, res) in ast
        .iter()
        .flatten()
        .map(|expr| (expr, evaluate(source, expr)))
    {
        print!("{:#}\n  \x1b[90m=\x1b[0m ", Lisp::new(expr));

        let item = match res {
            Ok(x) => {
                buf.clear();
                Ok(runtime_token(x, &mut buf))
            }
            Err(e) => {
                errors.push(e);
                Err(GenericError)
            }
        };

        for (lexeme, syntax) in highlight(std::iter::once(item)) {
            print!("{}", SYNTAX_STYLE_ANSI[syntax].style(lexeme));
        }
        println!("\x1b[0m\n");
    }
    drop(buf);

    // eval errors
    println!();
    if list_errors(errors) {
        #[expect(
            clippy::needless_return,
            reason = "should return here if more items follow this in the future"
        )]
        return;
    }
}

fn main() {
    let mut args = std::env::args_os();
    let prgm = args
        .next()
        .expect("must have a program argument to be running");
    match args.next() {
        // interactive
        None => {
            let mut input = String::new();
            loop {
                input.clear();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("failed to obtain input");
                if input.ends_with('\n') {
                    input.pop();
                    if input.ends_with('\r') {
                        input.pop();
                    }
                }
                if matches!(input.trim(), "exit" | "quit") {
                    break; // finish
                }
                run_code(&input);
            }
        }

        // from file
        Some(source_path) => match std::fs::read_to_string(std::path::Path::new(&source_path)) {
            Err(e) => eprintln!("failed to read source code file: {e}"),

            Ok(mut source) => {
                if source.ends_with('\n') {
                    source.pop();
                    if source.ends_with('\r') {
                        source.pop();
                    }
                }
                if args.next().is_some() {
                    eprintln!("usage: {} [script]", prgm.display());
                } else {
                    run_code(&source);
                }
            }
        },
    }
}
