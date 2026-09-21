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
#![warn(clippy::too_many_lines, reason = "yucky. clean that up.")]

use crate::{
    grammar::{
        highlight,
        style::{Color, Style},
        syntax::{BracketPair, Syntax, SyntaxStyle},
    },
    scanner::{ContextError, InterpolatedExpr, Token, TokenType, tokenize},
};

mod grammar;
mod scanner;

#[cfg(test)] // only include testing module in test builds
mod test;

#[deprecated = "use `grammar` module instead"]
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
                    TokenType::CharLiteral
                    | TokenType::StringLiteral
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

const SYNTAX_STYLE: SyntaxStyle = SyntaxStyle {
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

    bracket: Style::new().foreground(Color::BrightWhite),

    invalid: Style::new().foreground(Color::Rgb(0xcc, 0x0e, 0x0e)),
};

const BRACKET_PAIRS: BracketPair = BracketPair {
    depth: &[
        Style::new().foreground(Color::Rgb(0xff, 0xd7, 0x00)),
        Style::new().foreground(Color::Rgb(0xda, 0x70, 0xd6)),
        Style::new().foreground(Color::Rgb(0x17, 0x9f, 0xff)),
    ],
};

/// # Panics
/// This method can panic if [`scanner::Scanner`] isn't written correctly
pub fn run_code(source: &str) {
    use scanner::{InterpolatedString, TokenValue};

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

    // grammar highlighted
    println!("```");
    let mut bracket_depth = 0;
    for (lexeme, syntax) in highlight(&tokens) {
        if syntax == Syntax::Bracket {
            match lexeme {
                "[" | "(" | "{" => {
                    print!("{}", BRACKET_PAIRS.stylize(bracket_depth, lexeme));
                    bracket_depth += 1;
                }
                "]" | ")" | "}" => {
                    bracket_depth -= 1;
                    print!("{}", BRACKET_PAIRS.stylize(bracket_depth, lexeme));
                }
                _ => unimplemented!(),
            }
        } else {
            print!("{}", SYNTAX_STYLE.stylize(syntax, lexeme));
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
