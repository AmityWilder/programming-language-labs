//! # Batscript
//!
//! This project is not, and will not ever be, written with the help of any form of generative AI.
//! I do not like generative AI. I do not support it. It is a net negative on society and harms learning.

#![warn(
    clippy::pedantic,
    clippy::indexing_slicing,
    clippy::missing_const_for_fn
)]
#![warn(clippy::missing_safety_doc, clippy::missing_panics_doc, clippy::todo)]
#![deny(clippy::undocumented_unsafe_blocks, reason = "must prove soundness")]
#![deny(
    clippy::unwrap_used,
    clippy::missing_assert_message,
    reason = "give a reason for panics"
)]
#![warn(clippy::too_many_lines, reason = "yucky. clean that up.")]
#![allow(clippy::wildcard_imports)]
#![warn(clippy::arithmetic_side_effects, clippy::as_conversions)]
// #![warn(clippy::expect_used, clippy::panic)] // not actually a problem, just be aware

use grammar::syntax::syntax_of;

use crate::{
    grammar::{
        highlight,
        style::{Color, Style, StyleWrapper},
        syntax::{Syntax, SyntaxStyle},
    },
    scanner::tokenize,
};
use std::fmt::Write;

mod grammar;
mod scanner;

#[cfg(test)] // only include testing module in test builds
mod test;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Bracket {
    Brack,
    Paren,
    Brace,
}

const SYNTAX_STYLE_ANSII: SyntaxStyle<Style> = SyntaxStyle {
    normal: Style::new(),

    comment: Style::new().foreground(Color::Rgb(0x6a, 0x99, 0x55)),

    number_literal: Style::new().foreground(Color::Rgb(0xb5, 0xce, 0xa8)),

    char_literal: Style::new().foreground(Color::Rgb(0xce, 0x91, 0x78)),

    string_literal: Style::new().foreground(Color::Rgb(0xce, 0x91, 0x78)),

    interp_str_literal: Style::new().foreground(Color::Rgb(0xce, 0x91, 0x78)),

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

    bracket: Style::new().foreground(Color::BrightWhite),

    bracket_pairs: &[
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
    let tokens: Vec<_> = tokenize(source).collect();
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
            Err(e) => eprintln!("\x1b[91merror: {e}\x1b[0m"),
        }
    }

    // grammar highlighted
    let mut buf = String::new();
    let mut bracket_stack = Vec::new();
    for (lexeme, syntax) in highlight(&tokens) {
        let style = if syntax == Syntax::Bracket {
            let (kind, is_open) = match lexeme {
                "[" => (Bracket::Brack, true),
                "(" => (Bracket::Paren, true),
                "{" => (Bracket::Brace, true),

                "]" => (Bracket::Brack, false),
                ")" => (Bracket::Paren, false),
                "}" => (Bracket::Brace, false),

                // not "unreachable" because that isn't guaranteed to be true for future updates
                // and I don't want it assuming that's impossible and breaking in release builds
                _ => unimplemented!("only `[]`, `()`, and `{{}}` currently supported as brackets"),
            };
            if is_open {
                let n = bracket_stack.len();
                bracket_stack.push(kind);
                SYNTAX_STYLE_ANSII.bracket_pair(n)
            } else if bracket_stack
                .pop_if(|expecting| *expecting == kind)
                .is_some()
            {
                SYNTAX_STYLE_ANSII.bracket_pair(bracket_stack.len())
            } else {
                // bracket_stack is empty
                &SYNTAX_STYLE_ANSII[Syntax::Invalid]
            }
        } else {
            &SYNTAX_STYLE_ANSII[syntax]
        };
        _ = write!(buf, "{}", style.style(lexeme));
    }
    _ = write!(buf, "\x1b[0m");
    println!("```");
    for (i, line) in buf.lines().enumerate() {
        // TODO: need to reaffirm ansi styles across lines, or something
        // TODO: may want to right-align line numbers to the width of the
        // *longest one*, in case someone has a file with > 99999 lines
        println!("{:>5}   {line}", i.strict_add(1));
    }
    println!("```");

    // error list
    println!("errors:");
    let mut any_errors = false;
    for e in tokens.iter().filter_map(|item| item.as_ref().err()) {
        eprintln!("  \x1b[91m{}:\x1b[0m {}\n{}", e.code(), e.err, e.render());
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
