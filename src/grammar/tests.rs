use super::*;
use crate::scanner::tokenize;

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
        &[Expr::binary(Binary {
            lhs: Expr::literal(Token {
                lex: "5",
                val: LexValue::SIntLiteral(5),
                mac: None
            }),
            op: Token {
                lex: "+",
                val: LexValue::Punctuation(Punctuation::Add),
                mac: None
            },
            rhs: Expr::binary(Binary {
                lhs: Expr::unary(Unary {
                    side: OpSide::Right,
                    op: Token {
                        lex: "-",
                        val: LexValue::Punctuation(Punctuation::SubNeg),
                        mac: None
                    },
                    operand: Expr::grouping(Grouping {
                        open: Token {
                            lex: "(",
                            val: LexValue::Punctuation(Punctuation::LParen),
                            mac: None
                        },
                        expr: Expr::binary(Binary {
                            lhs: Expr::literal(Token {
                                lex: "7",
                                val: LexValue::SIntLiteral(7),
                                mac: None
                            }),
                            op: Token {
                                lex: "/",
                                val: LexValue::Punctuation(Punctuation::Div),
                                mac: None
                            },
                            rhs: Expr::literal(Token {
                                lex: "8",
                                val: LexValue::SIntLiteral(8),
                                mac: None
                            })
                        }),
                        close: Token {
                            lex: ")",
                            val: LexValue::Punctuation(Punctuation::RParen),
                            mac: None
                        }
                    })
                }),
                op: Token {
                    lex: "*",
                    val: LexValue::Punctuation(Punctuation::Mul),
                    mac: None
                },
                rhs: Expr::Literal(Token {
                    lex: "3",
                    val: LexValue::SIntLiteral(3),
                    mac: None
                })
            })
        })]
    );
}

macro_rules! spoof_scan {
    [$(($lex:expr, $Variant:ident$(($val:expr))?$(.$SubVariant:ident)? $(,)?)),* $(,)?] => {
        (concat!($($lex),*), [$(
            Token {
                lex: $lex,
                val: LexValue::$Variant$(($val))?$(($Variant::$SubVariant))?,
                mac: None,
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
        [Ok(Expr::binary(Binary {
            lhs: Expr::binary(Binary {
                lhs: Expr::literal(a),
                op: b,
                rhs: Expr::literal(c),
            }),
            op: d,
            rhs: Expr::literal(e),
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
        [Ok(Expr::binary(Binary {
            lhs: Expr::literal(a),
            op: b,
            rhs: Expr::binary(Binary {
                lhs: Expr::literal(c),
                op: d,
                rhs: Expr::literal(e),
            }),
        }))]
    );
}
