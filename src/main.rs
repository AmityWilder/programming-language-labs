use crate::scanner::{TokenType, tokenize};

mod scanner;

mod test;

fn run_code(source: &str) {
    println!("source code:\n```\n{source}\n```");
    let tokens: Vec<_> = tokenize(source).collect();
    for item in &tokens {
        match item {
            Ok(token) => {
                print!("{token:?}:\n  ");
                match token.value() {
                    Ok(None) => println!("ignored"),
                    Ok(Some(value)) => println!("{value:?}"),
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            Err(error) => eprintln!("\x1b[91merror: {error}\x1b[0m"),
        }
    }
    // syntax highlighted
    println!("```");
    for item in tokens.iter() {
        let (lexeme, ansi_color) = match item {
            Ok(token) => {
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
                } else {
                    run_code(&input);
                }
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
    };
}
