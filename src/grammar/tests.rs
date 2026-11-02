use super::*;
use crate::{
    lex::token::Operator,
    scanner::{
        token::{Lexeme, punc::Punctuation},
        tokenize,
    },
};

#[test]
fn test_parse0() {
    const SOURCE: &str = "5 + -(7 / 8) * 3";
    let expr = parse(
        SOURCE,
        tokenize(SOURCE).map(|item| item.expect("this example should not have any lex errors")),
    )
    .collect::<Result<Vec<_>, _>>()
    .expect("this example should be a valid expression");
    assert_eq!(
        expr.as_slice(),
        &[ast::Expr::binary(ast::Binary {
            lhs: ast::Expr::literal(ast::Literal {
                value: Token {
                    lex: Lexeme {
                        src: "5",
                        mac: None
                    },
                    val: LexValue::SIntLiteral(5),
                }
            }),
            op: SpecToken {
                lex: Lexeme {
                    src: "+",
                    mac: None
                },
                val: Operator::Add,
            },
            rhs: ast::Expr::binary(ast::Binary {
                lhs: ast::Expr::unary(ast::Unary {
                    side: OpSide::Prefix,
                    op: SpecToken {
                        lex: Lexeme {
                            src: "-",
                            mac: None
                        },
                        val: Operator::SubNeg,
                    },
                    operand: ast::Expr::grouping(ast::Grouping {
                        open: Lexeme {
                            src: "(",
                            mac: None
                        },
                        expr: ast::Expr::binary(ast::Binary {
                            lhs: ast::Expr::literal(ast::Literal {
                                value: Token {
                                    lex: Lexeme {
                                        src: "7",
                                        mac: None
                                    },
                                    val: LexValue::SIntLiteral(7),
                                }
                            }),
                            op: SpecToken {
                                lex: Lexeme {
                                    src: "/",
                                    mac: None
                                },
                                val: Operator::Div,
                            },
                            rhs: ast::Expr::literal(ast::Literal {
                                value: Token {
                                    lex: Lexeme {
                                        src: "8",
                                        mac: None
                                    },
                                    val: LexValue::SIntLiteral(8),
                                }
                            })
                        }),
                        close: Lexeme {
                            src: ")",
                            mac: None
                        }
                    })
                }),
                op: SpecToken {
                    lex: Lexeme {
                        src: "*",
                        mac: None
                    },
                    val: Operator::Mul,
                },
                rhs: ast::Expr::Literal(ast::Literal {
                    value: Token {
                        lex: Lexeme {
                            src: "3",
                            mac: None
                        },
                        val: LexValue::SIntLiteral(3),
                    }
                })
            })
        })]
    );
}

macro_rules! spoof_scan {
    [$(($lex:expr, $Variant:ident$(($val:expr))?$(.$SubVariant:ident)? $(,)?)),* $(,)?] => {
        (concat!($($lex),*), [$(
            Token {
                lex: Lexeme { src: $lex, mac: None },
                val: LexValue::$Variant$(($val))?$(($Variant::$SubVariant))?,
            }
        ),*])
    };
}

#[test]
fn test_assoc_add() {
    let (source, tokens @ [a, b, c, d, e]) = spoof_scan![
        ("1", SIntLiteral(1)),
        ("+", Punctuation.Add),
        ("2", SIntLiteral(2)),
        ("+", Punctuation.Add),
        ("3", SIntLiteral(3)),
    ];
    let ast: Vec<_> = parse(source, tokens).collect();
    assert_eq!(
        ast,
        [Ok(ast::Expr::binary(ast::Binary {
            lhs: ast::Expr::binary(ast::Binary {
                lhs: ast::Expr::literal(ast::Literal { value: a }),
                op: b.try_into().expect("+ is an operator"),
                rhs: ast::Expr::literal(ast::Literal { value: c }),
            }),
            op: d.try_into().expect("+ is an operator"),
            rhs: ast::Expr::literal(ast::Literal { value: e }),
        }))]
    );
}

#[test]
fn test_precedence() {
    let (source, tokens @ [a, b, c, d, e]) = spoof_scan![
        ("1", SIntLiteral(1)),
        ("+", Punctuation.Add),
        ("2", SIntLiteral(2)),
        ("*", Punctuation.Mul),
        ("3", SIntLiteral(3)),
    ];
    let ast: Vec<_> = parse(source, tokens).collect();
    assert_eq!(
        ast,
        [Ok(ast::Expr::binary(ast::Binary {
            lhs: ast::Expr::literal(ast::Literal { value: a }),
            op: b.try_into().expect("+ is an operator"),
            rhs: ast::Expr::binary(ast::Binary {
                lhs: ast::Expr::literal(ast::Literal { value: c }),
                op: d.try_into().expect("* is an operator"),
                rhs: ast::Expr::literal(ast::Literal { value: e }),
            }),
        }))]
    );
}
