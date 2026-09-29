//! # Batscript
//!
//! This project is not, and will not ever be, written with the help of any form of generative AI.
//! I do not like generative AI. I do not support it. It is a net negative on society and harms learning.

#![feature(
    try_from_int_error_kind, // used in number literal error
    iter_next_chunk,
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
    // clippy::pedantic,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
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
// #![warn(clippy::expect_used, clippy::panic)] // not actually a problem, just be aware
// #![warn(unsafe_code)] // not actually a problem, just be very careful
#![allow(clippy::wildcard_imports, reason = "don't care")]

use error::ContextError;
use eval::evaluate;
use grammar::{Binary, Expr, Unary, parse};
use highlight::{
    highlight,
    style::{Color, Style, StyleWrapper},
    syntax::{Syntax, SyntaxStyle, syntax_of},
};
use scanner::tokenize;
use std::{fmt::Write, range::Range};

mod error;
mod eval;
mod grammar;
mod highlight;
mod scanner;

#[cfg(test)] // only include testing module in test builds
mod test;

/// The style table currently being used
// TODO: make this configurable by file(?)
const SYNTAX_STYLE_ANSI: SyntaxStyle<Style> = SyntaxStyle {
    normal: Style::new(),

    comment: Style::new().foreground(Color::Rgb(0x6a, 0x99, 0x55)),

    dimmed: Style::new().foreground(Color::BrightBlack),

    number_literal: Style::new().foreground(Color::Rgb(0xb5, 0xce, 0xa8)),

    char_literal: Style::new().foreground(Color::Rgb(0xce, 0x91, 0x78)),

    string_literal: Style::new().foreground(Color::Rgb(0xce, 0x91, 0x78)),

    escape_seq: Style::new().foreground(Color::Rgb(0xd7, 0xba, 0x7d)),

    language_defined: Style::new().foreground(Color::Rgb(0x56, 0x9c, 0xd6)),

    variable: Style::new()
        .underline()
        .foreground(Color::Rgb(0x9c, 0xdc, 0xfe)),

    constant: Style::new()
        .underline()
        .foreground(Color::Rgb(0x4f, 0xc1, 0xff)),

    callable: Style::new()
        .underline()
        .foreground(Color::Rgb(0xdc, 0xdc, 0xaa)),

    keyword: Style::new().foreground(Color::Rgb(0x56, 0x9c, 0xd6)),

    ctrl_keyword: Style::new().foreground(Color::Rgb(0xc5, 0x86, 0xc0)),

    typename: Style::new().foreground(Color::Rgb(0x4e, 0xc9, 0xb0)),

    macro_name: Style::new()
        .underline()
        .foreground(Color::Rgb(0x56, 0x9c, 0xd6)),

    macro_arg: Style::new()
        .underline()
        .foreground(Color::Rgb(0x4f, 0xc1, 0xff)),

    bracket: &[
        Style::new().foreground(Color::Rgb(0xff, 0xd7, 0x00)),
        Style::new().foreground(Color::Rgb(0xda, 0x70, 0xd6)),
        Style::new().foreground(Color::Rgb(0x17, 0x9f, 0xff)),
    ],

    invalid: Style::new().foreground(Color::Rgb(0xcc, 0x0e, 0x0e)),
};

pub fn print_ast(depth: usize, node: &Expr<'_>) {
    match node {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            println!("Binary:");
            print!("{:>depth$} lhs: ", "");
            print_ast(depth.strict_add(2), lhs);
            println!("{:>depth$} op: {op:?}", "");
            print!("{:>depth$} rhs: ", "");
            print_ast(depth.strict_add(2), rhs);
        }
        Expr::Unary(inner) => {
            let Unary { op, rhs } = &**inner;
            println!("Unary:");
            println!("{:>depth$} op: {op:?}", "");
            print!("{:>depth$} rhs: ", "");
            print_ast(depth.strict_add(2), rhs);
        }
        Expr::Literal(token) => {
            println!("Literal: {token:?}");
        }
        Expr::Grouping(inner) => {
            print!("Grouping:");
            print_ast(depth.strict_add(2), inner);
        }
    }
}

/// Print a list of all errors with clean formatting
fn list_errors<'src, 'err, I>(errs: I)
where
    'src: 'err,
    I: IntoIterator<IntoIter: 'err, Item = &'err ContextError<'src>>,
{
    println!("errors:");
    let mut any_errors = false;
    for e in errs {
        const INDENT: &str = "          ";
        eprint!(
            "  \x1b[91m{}:\x1b[0m {}\n{}    \x1b[92mhelp:\x1b[0m ",
            e.code(),
            e.err,
            e.render(),
        );
        let mut has_prev = false;
        for line in e.help().to_string().lines() {
            if has_prev {
                eprint!("{INDENT}");
            }
            eprintln!("{line}");
            has_prev = true;
        }
        eprintln!();
        any_errors = true;
    }
    if !any_errors {
        println!("  \x1b[92mnone\x1b[0m");
    }
}

/// # Panics
/// This method can panic if [`scanner::Scanner`] isn't written correctly
pub fn run_code(source: &str) {
    // token debug
    println!("source code:\n```\n{source}\n```");

    println!();
    println!("tokens:");
    let tokens: Vec<_> = tokenize(source).collect();
    let max_cols = source.lines().map(str::len).max().unwrap_or(0);
    let max_range_digits = max_cols.to_string().len().strict_mul(2);
    for item in &tokens {
        let (_, syn, _) = syntax_of(item);
        let style = SYNTAX_STYLE_ANSI[syn];
        let range = match item {
            Ok(token) => token.lex_range(source),
            Err(e) => e.range,
        };
        print!("\x1b[90m{range:>max_range_digits$?}:\x1b[0m ");
        match item {
            Ok(token) => {
                println!("{}{token:?}{}", style.begin(), style.end());
            }
            Err(ContextError { source, range, err }) => {
                let src = source
                    .get(*range)
                    .expect("range should be a range in source");
                println!(
                    "{}ContextError({src:?}): {err:?}{}",
                    style.begin(),
                    style.end()
                );
            }
        }
    }

    // grammar highlighted
    let mut buf = String::new();
    for (lexeme, syntax) in highlight(&tokens) {
        _ = write!(buf, "{}", SYNTAX_STYLE_ANSI[syntax].style(lexeme));
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
        let pre = buf
            .get(..start)
            .expect("substr_range start should not be within a UTF-8 character");
        let last_ansi_seq = pre
            .rfind("\x1b[")
            .and_then(|pos| {
                let s = buf.get(pos..).expect(
                    "rfind should return a valid position within buf. \
                    pre only shortens the end, not the start, so pos should still be a valid start position.",
                );
                s.split_inclusive('m').next()
            })
            .unwrap_or("\x1b[0m");
        println!(
            " \x1b[90m{:>line_num_width$}{last_ansi_seq}   {line}",
            i.strict_add(1)
        );
    }
    println!("```");

    // lex errors
    println!();
    list_errors(tokens.iter().map(Result::as_ref).filter_map(Result::err));

    // parse debug
    println!();
    println!("ast:");
    let ast: Vec<_> = parse(source, tokens.into_iter().filter_map(Result::ok)).collect();
    for res in &ast {
        match res {
            Ok(node) => {
                print_ast(0, node);
            }
            Err(ContextError { source, range, err }) => {
                let style = SYNTAX_STYLE_ANSI[Syntax::Invalid];
                let src = source
                    .get(*range)
                    .expect("range should be a range in source");
                println!(
                    "{}ContextError({src:?}): {err:?}{}",
                    style.begin(),
                    style.end()
                );
            }
        }
    }

    // parse errors
    println!();
    list_errors(ast.iter().map(Result::as_ref).filter_map(Result::err));

    // eval
    println!();
    println!("evaluation:");
    let errors: Vec<_> = ast
        .iter()
        .flatten()
        .map(|expr| {
            evaluate(source, expr)
                .map(|x| {
                    print!("\x1b[90m{expr}:\x1b[0m ");
                    match x {
                        eval::Value::Bool(x) => println!("{x}"),
                        eval::Value::UInt(x) => println!("{x}"),
                        eval::Value::SInt(x) => println!("{x}"),
                        eval::Value::Frac(x) => println!("{x}"),
                        eval::Value::Char(x) => println!("{x:?}"),
                        eval::Value::Str(x) => println!("{x:?}"),
                    }
                })
                .inspect_err(|e| println!("error: {e:?}"))
        })
        .filter_map(Result::err)
        .collect();

    println!();
    list_errors(errors.iter());
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
