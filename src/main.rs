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
    clippy::pedantic,
    clippy::missing_const_for_fn,
    missing_docs,
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
// #![warn(clippy::expect_used, clippy::panic, unsafe_code)] // not actually a problem, just be aware

use grammar::{
    highlight,
    style::{Color, Style, StyleWrapper},
    syntax::{SyntaxStyle, syntax_of},
};
use scanner::tokenize_noalloc;
use std::{fmt::Write, range::Range};

pub mod error;
pub mod grammar;
pub mod scanner;

#[cfg(test)] // only include testing module in test builds
mod test;

const SYNTAX_STYLE_ANSII: SyntaxStyle<Style> = SyntaxStyle {
    normal: Style::new(),

    comment: Style::new().foreground(Color::Rgb(0x6a, 0x99, 0x55)),

    number_literal: Style::new().foreground(Color::Rgb(0xb5, 0xce, 0xa8)),

    char_literal: Style::new().foreground(Color::Rgb(0xce, 0x91, 0x78)),

    string_literal: Style::new().foreground(Color::Rgb(0xce, 0x91, 0x78)),

    escape_seq: Style::new().foreground(Color::Rgb(0xd7, 0xba, 0x7d)),

    interp_expr: Style::new().foreground(Color::Rgb(0x56, 0x9c, 0xd6)),

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

    macro_name: Style::new()
        .underline()
        .foreground(Color::Rgb(0xc5, 0x86, 0xc0)),

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

/// # Panics
/// This method can panic if [`scanner::Scanner`] isn't written correctly
pub fn run_code(source: &str) {
    // token debug
    println!("source code:\n```\n{source}\n```");
    let tokens: Vec<_> = tokenize_noalloc(source).collect();
    for item in &tokens {
        let (lex, syn, _) = syntax_of(item);
        match item {
            Ok((token, value)) => {
                let style = SYNTAX_STYLE_ANSII[syn];
                println!(
                    "{:?}: {}{token:?}:\n  {value:?}{}",
                    source
                        .substr_range(lex)
                        .expect("every lexeme should be a substr of source"),
                    style.begin(),
                    style.end()
                );
            }
            Err(e) => eprintln!("\x1b[91merror: {e:#?}\x1b[0m"),
        }
    }

    // grammar highlighted
    let mut buf = String::new();
    for (lexeme, syntax) in highlight(&tokens) {
        _ = write!(buf, "{}", SYNTAX_STYLE_ANSII[syntax].style(lexeme));
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

    // error list
    println!("errors:");
    let mut any_errors = false;
    for e in tokens.iter().filter_map(|item| item.as_ref().err()) {
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
                if matches!(input.trim(), "exit" | "quit") {
                    break; // finish
                }
                run_code(&input);
            }
        }

        // from file
        Some(source_path) => match std::fs::read_to_string(std::path::Path::new(&source_path)) {
            Err(e) => eprintln!("failed to read source code file: {e}"),

            Ok(source) => {
                if args.next().is_some() {
                    eprintln!("usage: {} [script]", prgm.display());
                } else {
                    run_code(&source);
                }
            }
        },
    }
}
