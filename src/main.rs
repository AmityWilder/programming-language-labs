//! # Batscript
//!
//! This project is not, and will not ever be, written with the help of any form of generative AI.
//! I do not like generative AI. I do not support it. It is a net negative on society and harms learning.

#![warn(clippy::pedantic)]
#![warn(clippy::missing_safety_doc, clippy::missing_panics_doc, clippy::todo)]
#![deny(clippy::undocumented_unsafe_blocks, reason = "must prove soundness")]
#![deny(
    clippy::unwrap_used,
    clippy::missing_assert_message,
    reason = "give a reason for panics"
)]
#![allow(clippy::too_many_lines, clippy::enum_glob_use, reason = "don't care.")]
use scanner::{TokenType, tokenize};

use crate::scanner::{ContextError, InterpolatedExpr, Token};

mod scanner;

#[cfg(test)] // only include testing module in test builds
mod test;

fn token_highlight<'a, T>(
    item: &Result<(Token<'a>, T), ContextError<'a>>,
) -> (&'a str, (&'static str, Option<&'static str>)) {
    match item {
        Ok((token, _)) => {
            (
                token.src,
                match token.ty {
                    TokenType::Whitespace => ("0", None),
                    TokenType::Comment => ("32", None),
                    TokenType::NumberLiteral => ("92", None),
                    // TODO: what about escape sequences/expressions within literals?
                    TokenType::StringLiteral
                    | TokenType::CharLiteral
                    | TokenType::InterpolatedString => ("33", None),
                    TokenType::Identifier => ("4;96", Some("24")),
                    TokenType::Callable => ("4;93", Some("24")),
                    TokenType::Keyword => ("94", None),
                    TokenType::CtrlKeyword => ("95", None),
                    TokenType::Punctuation => ("37", None),
                },
            )
        }
        Err(e) => (&e.source[e.range], ("91", None)),
    }
}

/// # Panics
/// This method can panic if [`scanner::Scanner`] isn't written correctly
pub fn run_code(source: &str) {
    use scanner::{CharLiteral, InterpolatedString, StringLiteral, TokenValue};
    use std::borrow::Cow;

    // token debug
    println!("source code:\n```\n{source}\n```");
    let tokens: Vec<_> = tokenize(source).collect();
    for item in &tokens {
        match item {
            Ok((token, value)) => {
                print!("{token:?}:\n  ");
                match value {
                    None => println!("ignored"),
                    Some(value) => {
                        if let TokenValue::InterpolatedString(InterpolatedString {
                            text,
                            expressions,
                        }) = value
                        {
                            println!(
                                "InterpolatedString(InterpolatedString {{ text: {text:?}, expressions: {} }})",
                                if expressions.is_empty() {
                                    "[]"
                                } else {
                                    "<below>"
                                }
                            );
                            for InterpolatedExpr {
                                range,
                                position,
                                expr,
                            } in expressions
                            {
                                println!(
                                    "    InterpolatedExpr {{ range: {range:?}, positions: {position:?}, expr: {} }}",
                                    if expr.is_empty() { "[]" } else { "<below>" }
                                );
                                for item in expr {
                                    match item {
                                        Ok((token, value)) => {
                                            print!("      {token:?}:\n        ");
                                            match value {
                                                Some(value) => println!("{value:?}"),
                                                None => println!("ignored"),
                                            }
                                        }
                                        Err(e) => eprintln!("\x1b[91merror: {e}\x1b[0m"),
                                    }
                                }
                            }
                        } else {
                            println!("{value:?}");
                        }
                    }
                }
            }
            Err(e) => eprintln!("\x1b[91merror: {e}\x1b[0m"),
        }
    }

    // syntax highlighted
    println!("```");
    for item in &tokens {
        let (lexeme, (ansi_color, ansi_finish)) = token_highlight(item);
        if let Ok((
            _,
            Some(
                value @ (TokenValue::CharLiteral(..)
                | TokenValue::StringLiteral(StringLiteral {
                    text: Cow::Owned(_),
                    ..
                })
                | TokenValue::InterpolatedString(InterpolatedString {
                    text:
                        StringLiteral {
                            text: Cow::Owned(_),
                            ..
                        },
                    ..
                })),
            ),
        )) = item
        {
            const ESCAPE_COLOR: &str = "95";
            match value {
                TokenValue::StringLiteral(StringLiteral { escapes, .. }) => {
                    const DELIM: char = '"';
                    let inner = lexeme
                        .strip_prefix(DELIM)
                        .and_then(|s| s.strip_suffix(DELIM))
                        .expect("character literal lexeme should include delimiters");
                    print!("\x1b[{ansi_color}m\"");
                    let mut prev_end = 0;
                    for &escape in escapes {
                        print!(
                            "\x1b[{ansi_color}m{}\x1b[{ESCAPE_COLOR}m{}",
                            &inner[prev_end..escape.start],
                            &inner[escape],
                        );
                        prev_end = escape.end;
                    }
                    print!("\x1b[{ansi_color}m\"");
                }

                TokenValue::CharLiteral(CharLiteral { is_escaped, .. }) => {
                    if *is_escaped {
                        const DELIM: char = '\'';
                        let inner = lexeme
                            .strip_prefix(DELIM)
                            .and_then(|s| s.strip_suffix(DELIM))
                            .expect("character literal lexeme should include delimiters");
                        print!("\x1b[{ansi_color}m'\x1b[{ESCAPE_COLOR}m{inner}\x1b[{ansi_color}m'");
                    } else {
                        print!("\x1b[{ansi_color}m{lexeme}");
                    }
                }

                TokenValue::InterpolatedString(InterpolatedString {
                    text: StringLiteral { escapes, .. },
                    expressions,
                }) => {
                    const INTERP_BRACES_COLOR: &str = "94";
                    const DELIM: char = '`';
                    let inner = lexeme
                        .strip_prefix(DELIM)
                        .and_then(|s| s.strip_suffix(DELIM))
                        .expect("character literal lexeme should include delimiters");
                    print!("\x1b[{ansi_color}m\"");
                    let mut prev_end = 0;
                    let mut esc_iter = escapes.iter().peekable();
                    let mut expr_iter = expressions.iter().peekable();
                    // need to visit in order
                    for (range, expr) in std::iter::from_fn(|| {
                        esc_iter
                            .next_if(|esc_range| {
                                expr_iter
                                    .peek()
                                    .is_none_or(|expr| esc_range.start < expr.range.start)
                            })
                            .map(|&range| (range, None))
                            .or_else(|| expr_iter.next().map(|expr| (expr.range, Some(&expr.expr))))
                    }) {
                        if let Some(tokens) = expr {
                            print!("\x1b[{INTERP_BRACES_COLOR}m${{");
                            for item in tokens {
                                let (lexeme, (ansi_color, ansi_finish)) = token_highlight(item);
                                print!("\x1b[{ansi_color}m{lexeme}");
                                if let Some(ansi_finish) = ansi_finish {
                                    print!("\x1b[{ansi_finish}m");
                                }
                            }
                            print!("\x1b[{INTERP_BRACES_COLOR}m}}");
                        } else {
                            print!(
                                "\x1b[{ansi_color}m{}\x1b[{ESCAPE_COLOR}m{}",
                                &inner[prev_end..range.start],
                                &inner[range],
                            );
                        }
                        prev_end = range.end;
                    }
                    print!("\x1b[{ansi_color}m\"");
                }

                _ => unreachable!("guarded by if condition"),
            }
        } else {
            print!("\x1b[{ansi_color}m{lexeme}");
        }
        if let Some(ansi_finish) = ansi_finish {
            print!("\x1b[{ansi_finish}m");
        }
    }
    println!("\x1b[0m\n```");

    // error list
    println!("errors:");
    let mut any_errors = false;
    for e in tokens.iter().filter_map(|item| item.as_ref().err()) {
        eprintln!("  \x1b[91m{e}\x1b[0m");
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
