//! AST Printer
//!
//! - Expression classes
//!   - `Binary`
//!     - [x] As root
//!     - [ ] Nested in `Binary`
//!     - [ ] Nested in `Unary`
//!     - [ ] Nested in `Group`
//!   - `Unary`
//!     - [ ] As root
//!     - [x] Nested in `Binary`
//!     - [ ] Nested in `Unary`
//!     - [ ] Nested in `Group`
//!   - `Literal`
//!     - [ ] As root
//!     - [ ] Nested in `Binary`
//!     - [x] Nested in `Unary`
//!     - [x] Nested in `Group`
//!   - `Group`
//!     - [ ] As root
//!     - [x] Nested in `Binary`
//!     - [ ] Nested in `Unary`
//!     - [ ] Nested in `Group`
//! - Literal types
//!   - [ ] `BoolLiteral`
//!   - [ ] `Keyword(None)`
//!   - [ ] `UIntLiteral`
//!   - [x] `SIntLiteral`
//!   - [x] `FracLiteral`
//!   - [ ] `CharLiteral`
//!   - [ ] `StringLiteral`
//! - Operators
//!   - [ ] `Add`
//!   - [ ] `And`
//!   - [ ] `Div`
//!   - [ ] `Eq`
//!   - [ ] `Ge`
//!   - [ ] `Gt`
//!   - [ ] `Le`
//!   - [ ] `Lt`
//!   - [ ] `Mul`
//!   - [ ] `Nand`
//!   - [ ] `Ne`
//!   - [ ] `Nor`
//!   - [ ] `Not`
//!   - [ ] `Or`
//!   - [ ] `Pow`
//!   - [ ] `Rem`
//!   - [ ] `Rotl`
//!   - [ ] `Rotr`
//!   - [ ] `Shl`
//!   - [ ] `Shr`
//!   - `SubNeg`
//!     - [ ] Binary (Sub)
//!     - [x] Unary (Neg)
//!   - [ ] `Xnor`
//!   - [ ] `Xor`

use crate::{
    error::OpSide,
    grammar::{
        ast::{Binary, Expr, Grouping, Unary},
        fmt::Lisp,
    },
    print_ast,
    scanner::token::{Token, punc::Punctuation, value::LexValue},
};

#[test]
fn test_ast_printer1() {
    let ast: Expr = Expr::binary(Binary {
        lhs: Expr::unary(Unary {
            side: OpSide::Right,
            op: Token {
                lex: "-",
                val: LexValue::Punctuation(Punctuation::SubNeg),
                mac: None,
            },
            operand: Expr::Literal(Token {
                lex: "123",
                val: LexValue::SIntLiteral(123),
                mac: None,
            }),
        }),
        op: Token {
            lex: "*",
            val: LexValue::Punctuation(Punctuation::Mul),
            mac: None,
        },
        rhs: Expr::grouping(Grouping {
            open: Token {
                lex: "(",
                val: LexValue::Punctuation(Punctuation::LParen),
                mac: None,
            },
            expr: Expr::literal(Token {
                lex: "45.67",
                val: LexValue::FracLiteral(45.67),
                mac: None,
            }),
            close: Token {
                lex: ")",
                val: LexValue::Punctuation(Punctuation::RParen),
                mac: None,
            },
        }),
    });
    println!("debug:\n{ast:#?}\n\ncustom:");
    print_ast(&ast, 0, 0);
    let lisp_repr = Lisp::new(&ast).to_string();
    println!("\nlisp:\n{lisp_repr}");
    println!("\nlisp (colored):\n{:#}", Lisp::new(&ast));
    assert_eq!(lisp_repr, "(* (- 123) (group 45.67))");
}

#[test]
fn test_ast_printer2() {
    let ast: Expr = Expr::binary(Binary {
        lhs: Expr::unary(Unary {
            side: OpSide::Right,
            op: Token {
                lex: "-",
                val: LexValue::Punctuation(Punctuation::SubNeg),
                mac: None,
            },
            operand: Expr::Literal(Token {
                lex: "123",
                val: LexValue::SIntLiteral(123),
                mac: None,
            }),
        }),
        op: Token {
            lex: "*",
            val: LexValue::Punctuation(Punctuation::Mul),
            mac: None,
        },
        rhs: Expr::grouping(Grouping {
            open: Token {
                lex: "(",
                val: LexValue::Punctuation(Punctuation::LParen),
                mac: None,
            },
            expr: Expr::literal(Token {
                lex: "45.67",
                val: LexValue::FracLiteral(45.67),
                mac: None,
            }),
            close: Token {
                lex: ")",
                val: LexValue::Punctuation(Punctuation::RParen),
                mac: None,
            },
        }),
    });
    println!("debug:\n{ast:?}\n\ncustom:");
    print_ast(&ast, 0, 0);
    let lisp_repr = Lisp::new(&ast).to_string();
    println!("\nlisp:\n{lisp_repr}");
    assert_eq!(lisp_repr, "(* (- 123) (group 45.67))");
}

#[test]
fn test_ast_printer3() {
    let ast: Expr = Expr::binary(Binary {
        lhs: Expr::unary(Unary {
            side: OpSide::Right,
            op: Token {
                lex: "-",
                val: LexValue::Punctuation(Punctuation::SubNeg),
                mac: None,
            },
            operand: Expr::Literal(Token {
                lex: "123",
                val: LexValue::SIntLiteral(123),
                mac: None,
            }),
        }),
        op: Token {
            lex: "*",
            val: LexValue::Punctuation(Punctuation::Mul),
            mac: None,
        },
        rhs: Expr::grouping(Grouping {
            open: Token {
                lex: "(",
                val: LexValue::Punctuation(Punctuation::LParen),
                mac: None,
            },
            expr: Expr::literal(Token {
                lex: "45.67",
                val: LexValue::FracLiteral(45.67),
                mac: None,
            }),
            close: Token {
                lex: ")",
                val: LexValue::Punctuation(Punctuation::RParen),
                mac: None,
            },
        }),
    });
    println!("debug:\n{ast:?}\n\ncustom:");
    print_ast(&ast, 0, 0);
    let lisp_repr = Lisp::new(&ast).to_string();
    println!("\nlisp:\n{lisp_repr}");
    assert_eq!(lisp_repr, "(* (- 123) (group 45.67))");
}
