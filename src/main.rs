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

use scanner::{TokenType, tokenize};

mod scanner;

#[cfg(test)] // only include testing module in test builds
mod test;

pub fn run_code(source: &str) {
    println!("source code:\n```\n{source}\n```");
    let tokens: Vec<_> = tokenize(source).collect();
    for item in &tokens {
        match item {
            Ok((token, value)) => {
                print!("{token:?}:\n  ");
                match value {
                    None => println!("ignored"),
                    Some(value) => println!("{value:?}"),
                }
            }
            Err(error) => eprintln!("\x1b[91merror: {error}\x1b[0m"),
        }
    }
    // syntax highlighted
    println!("```");
    for item in &tokens {
        let (lexeme, ansi_color) = match item {
            Ok((token, _)) => {
                let ansi_color = match token.ty {
                    TokenType::Whitespace => "0",
                    TokenType::Comment => "32",
                    TokenType::NumberLiteral => "92",
                    TokenType::StringLiteral => "33",
                    TokenType::Identifier => "4;96",
                    TokenType::Keyword => "94",
                    TokenType::CtrlKeyword => "95",
                    TokenType::Punctuation => "37",
                };
                (token.src, ansi_color)
            }
            Err(e) => (&e.source[e.range], "91"),
        };
        print!("\x1b[{ansi_color}m{lexeme}");
    }
    println!("\x1b[0m\n```");
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
                if input.trim() == "exit" {
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
                    eprintln!("Usage: {} [script]", prgm.display());
                } else {
                    run_code(&source);
                }
            }
        },
    }
}
