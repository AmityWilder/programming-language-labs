//! Scanner tests.
//! All of these test cases are hand-written. Mostly on a very bumpy bus ride.

use crate::{
    error::DiagnosticKind,
    lex::{lex, token::TokenKind},
    src_code::span::uspan_to_usize,
};

mod scan {
    use super::*;

    #[track_caller]
    fn test_lex(src: &str) -> Vec<(&str, Result<TokenKind, DiagnosticKind>)> {
        use std::cell::Cell;
        let mut dup = src.as_bytes();
        let prev_err = Cell::new(None);
        lex(src, |span, kind| {
            let res = prev_err.replace(Some((&src[<std::range::Range<usize>>::from(span)], kind)));
            assert_eq!(res, None, "should have consumed error with invalid token");
        })
        .map(|(len, kind)| {
            let lex = dup
                .split_off(..uspan_to_usize(len))
                .and_then(|s| str::from_utf8(s).ok())
                .expect("lex lengths should be valid UTF-8");
            let kind = if matches!(kind, TokenKind::Invalid) {
                let (expect, kind) = prev_err
                    .take()
                    .expect("should push an error if a token is invalid");
                assert_eq!(lex, expect, "invalid lexeme should match error lexeme");
                Err(kind)
            } else {
                Ok(kind)
            };
            (lex, kind)
        })
        .collect::<Vec<_>>()
    }

    #[test]
    fn test_empty_macro() {
        const SOURCE: &str = "\\";
        assert_eq!(
            test_lex(SOURCE),
            [(SOURCE, Err(DiagnosticKind::EmptyMacroName))]
        );
    }

    #[test]
    fn test_empty_macro_param() {
        const SOURCE: &str = "$";
        assert_eq!(
            test_lex(SOURCE),
            [(SOURCE, Err(DiagnosticKind::EmptyMacroParam))]
        );
    }

    #[test]
    fn test_invalid_token() {
        const SOURCE: &str = "`";
        assert_eq!(
            test_lex(SOURCE),
            [(SOURCE, Err(DiagnosticKind::UnknownToken))]
        );
    }

    /// [`TokenType::Whitespace`]
    mod whitespace {
        use super::*;

        #[test]
        fn test_scan_whitespace_single() {
            const SOURCE: &str = " ";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::WSpace))]);
        }

        #[test]
        fn test_scan_whitespace_multi() {
            const SOURCE: &str = " \n\r\t ";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::WSpace))]);
        }
    }

    /// [`TokenType::Comment`]
    mod comments {
        use super::*;

        /// Line comments (`//[^\n\r]*`)
        mod line {
            use super::*;

            #[test]
            fn test_scan_line_comment_no_newline() {
                const SOURCE: &str = "// apple";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::Comment))]);
            }

            #[test]
            fn test_scan_line_comment_typical() {
                assert_eq!(
                    test_lex("// apple\n"),
                    &[
                        ("// apple", Ok(TokenKind::Comment)),
                        ("\n", Ok(TokenKind::WSpace)),
                    ]
                );
            }
        }

        /// Block comments (/* ... */)
        mod block {}
    }

    /// [`TokenType::Identifier`] (`[a-zA-Z_][a-zA-Z_']*`)
    mod ident {
        use super::*;

        #[test]
        fn test_scan_ident_simple() {
            const SOURCE: &str = "foo";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::Ident))]);
        }

        #[test]
        fn test_scan_ident_prime() {
            const SOURCE: &str = "x'";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::Ident))]);
        }

        #[test]
        fn test_scan_ident_apostrophe() {
            const SOURCE: &str = "can't";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::Ident))]);
        }
    }

    /// [`TokenType::NumberLiteral`]
    mod number {
        use super::*;
        use crate::lex::token::NumberLiteral;

        /// Simple (`-?\d+(\.\d+)?`)
        mod simple {
            use super::*;

            #[test]
            fn test_scan_number_simple() {
                const SOURCE: &str = "5";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(SOURCE.parse::<NumberLiteral>(), Ok(NumberLiteral::Sint(5)));
            }

            #[test]
            fn test_scan_number_multidigit() {
                const SOURCE: &str = "35";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(SOURCE.parse::<NumberLiteral>(), Ok(NumberLiteral::Sint(35)));
            }

            #[test]
            fn test_scan_number_underscored() {
                const SOURCE: &str = "3_5";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(SOURCE.parse::<NumberLiteral>(), Ok(NumberLiteral::Sint(35)));
            }

            #[test]
            fn test_scan_number_multi_underscored() {
                const SOURCE: &str = "3__5";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(SOURCE.parse::<NumberLiteral>(), Ok(NumberLiteral::Sint(35)));
            }

            #[test]
            fn test_scan_number_decimal() {
                const SOURCE: &str = "2.5";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(2.5))
                );
            }

            #[test]
            fn test_scan_number_multidigit_decimal() {
                const SOURCE: &str = "25.25";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(25.25))
                );
            }

            #[test]
            fn test_scan_number_underscored_negative() {
                const SOURCE: &str = "2_5.2_5";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(25.25))
                );
            }
        }

        /// Scientific notation (`-?\d+(\.\d+)?([eE]-?\d+)?`)
        #[cfg(false)] // TODO
        mod sci_notation {
            use super::*;

            #[test]
            fn test_scan_number_sci_notation() {
                const SOURCE: &str = "5e0";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(5e0))
                );
            }

            #[test]
            fn test_scan_number_sci_notation_underscored() {
                const SOURCE: &str = "5_5e0";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(55e0))
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation() {
                const SOURCE: &str = "5e-5";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(5e-5))
                );
            }

            #[test]
            fn test_scan_number_pos_sci_notation() {
                const SOURCE: &str = "5e+5";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(5e+5))
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp() {
                const SOURCE: &str = "5e-50";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(5e-50))
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_negative() {
                const SOURCE: &str = "-5e-50";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(-5e-50))
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_underscored_exp() {
                const SOURCE: &str = "5e-5_0";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(5e-5_0))
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_excess_negative() {
                assert_eq!(
                    test_lex("-5e-5-3"),
                    &[
                        ("-5e-5", Ok(TokenKind::NumberLit)),
                        ("-", Ok(TokenKind::Punc(Punctuation::SubNeg))),
                        ("3", Ok(TokenKind::NumberLit)),
                    ]
                );
                assert_eq!(
                    "-5e-5".parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Flpt(-5e-5))
                );
                assert_eq!("3".parse::<NumberLiteral>(), Ok(NumberLiteral::Sint(3)));
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_double_negative() {
                // without a number after the hyphen, the number literal is "-5e",
                // but "e" isn't a valid number literal suffix
                const SOURCE: &str = "-5e--5";
                assert_eq!(
                    test_lex("-5e--5"),
                    &[
                        ("-5e", Ok(TokenKind::NumberLit)),
                        ("-", Ok(TokenKind::Punc(Punctuation::SubNeg))),
                        ("3", Ok(TokenKind::NumberLit)),
                    ]
                );
                assert_eq!("-5e".parse::<NumberLiteral>(), Err(()));
                assert_eq!("3".parse::<NumberLiteral>(), Ok(NumberLiteral::Sint(3)));
            }
        }

        /// Hexadecimal (`-?0x[0-9a-fA-F]+`)
        #[cfg(false)] // TODO
        mod hex {
            use super::*;

            #[test]
            fn test_scan_number_hex() {
                const SOURCE: &str = "0x9F";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Sint(0x9f))
                );
            }

            #[test]
            fn test_scan_number_hex_bad_digit() {
                const SOURCE: &str = "0x9G";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Err(()),
                    "'G' is not a valid hex digit, nor a suffix"
                );
            }

            #[test]
            fn test_scan_number_hex_unsigned_suffix() {
                const SOURCE: &str = "0x9u";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Uint(0x9))
                );
            }

            #[test]
            fn test_scan_number_hex_signed_suffix() {
                const SOURCE: &str = "0x9s";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::NumberLit))]);
                assert_eq!(
                    SOURCE.parse::<NumberLiteral>(),
                    Ok(NumberLiteral::Sint(0x9))
                );
            }
        }

        /// Octal (`-?0o[0-7]+`)
        #[cfg(false)] // TODO
        mod oct {
            use super::*;

            #[test]
            fn test_scan_number_oct() {
                const SOURCE: &str = "0o253";
                assert_eq!(
                    test_lex(SOURCE),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(0o253),
                    })]
                );
            }

            #[test]
            fn test_scan_number_oct_bad_digit() {
                const SOURCE: &str = "0o258";
                assert_matches!(
                    test_lex(SOURCE),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: _,
                        mac: None,
                        err: DiagnosticKind::InvalidNumLiteral(_)
                    })]
                );
            }
        }

        /// Binary (`-?0b[01]+`)
        #[cfg(false)] // TODO
        mod bin {
            use super::*;

            #[test]
            fn test_scan_number_bin() {
                const SOURCE: &str = "0b11011011";
                assert_eq!(
                    test_lex(SOURCE),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(0b1101_1011),
                    })]
                );
            }

            #[test]
            fn test_scan_number_bin_wrong_digit() {
                const SOURCE: &str = "0b11011012";
                assert_matches!(
                    test_lex(SOURCE),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: _,
                        mac: None,
                        err: DiagnosticKind::InvalidNumLiteral(_)
                    })]
                );
            }
        }
    }

    mod char {
        use super::*;
        use crate::lex::token::{CharLiteral, CharLiteralError};

        #[test]
        fn test_scan_char_simple() {
            const SOURCE: &str = "'a'";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::CharLit))]);
            assert_eq!(SOURCE.parse::<CharLiteral>(), Ok(CharLiteral('a')));
        }

        #[test]
        fn test_scan_char_multi() {
            const SOURCE: &str = "'aa'";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::CharLit))]);
            assert_eq!(
                SOURCE.parse::<CharLiteral>(),
                Err(CharLiteralError::MultiCharLiteral)
            );
        }

        #[test]
        fn test_scan_char_empty() {
            const SOURCE: &str = "''";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::CharLit))]);
            assert_eq!(
                SOURCE.parse::<CharLiteral>(),
                Err(CharLiteralError::EmptyCharLiteral)
            );
        }

        #[test]
        fn test_scan_char_endless() {
            const SOURCE: &str = "' ";
            assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::CharLit))]);
            assert_eq!(
                SOURCE.parse::<CharLiteral>(),
                Err(CharLiteralError::EndlessCharLiteral)
            );
        }

        #[cfg(false)] // TODO
        mod escaped {
            use super::*;

            #[test]
            fn test_scan_char_escaped() {
                const SOURCE: &str = "'\\0'";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::CharLit))]);
                assert_eq!(SOURCE.parse::<CharLiteral>(), Ok(CharLiteral('\0')));
            }

            #[test]
            fn test_scan_char_escaped_hex() {
                const SOURCE: &str = "'\\x1b'";
                assert_eq!(test_lex(SOURCE), &[(SOURCE, Ok(TokenKind::CharLit))]);
                assert_eq!(SOURCE.parse::<CharLiteral>(), Ok(CharLiteral('\x1b')));
            }

            #[test]
            fn test_scan_char_escaped_multi() {
                const SOURCE: &str = "'\\1b'";
                assert_eq!(
                    test_lex(SOURCE),
                    &[(SOURCE, Err(DiagnosticKind::MultiCharLiteral))]
                );
            }

            #[test]
            fn test_scan_char_escaped_invalid() {
                const SOURCE: &str = "'\\'";
                assert_eq!(
                    test_lex(SOURCE),
                    &[(SOURCE, Err(DiagnosticKind::EscapedCharLiteralEnd))]
                );
            }
        }
    }

    #[cfg(false)] // TODO
    mod str {
        use super::*;
        use crate::lex::token::TextLiteral;

        #[test]
        fn test_scan_str_simple() {
            const SOURCE: &str = "\"a\"";
            assert_eq!(
                test_lex(SOURCE),
                &[Ok(Token {
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::TextLiteral(TextLiteral { content: "a" }),
                })]
            );
        }

        #[test]
        fn test_scan_str_multi() {
            const SOURCE: &str = "\"aa\"";
            assert_eq!(
                test_lex(SOURCE),
                &[Ok(Token {
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::TextLiteral(TextLiteral { content: "aa" }),
                })]
            );
        }

        #[test]
        fn test_scan_str_empty() {
            const SOURCE: &str = "\"\"";
            assert_eq!(
                test_lex(SOURCE),
                &[Ok(Token {
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::TextLiteral(TextLiteral { content: "" }),
                })]
            );
        }

        #[test]
        fn test_scan_str_endless() {
            const SOURCE: &str = "\" ";
            assert_eq!(
                test_lex(SOURCE),
                &[Err(ContextError::error(
                    SOURCE,
                    Some((0..SOURCE.len()).into()),
                    None,
                    DiagnosticKind::EndlessStringLiteral,
                ))]
            );
        }

        mod escaped {
            use super::*;

            #[test]
            fn test_scan_str_escaped() {
                const SOURCE: &str = "\"\\0\"";
                assert_eq!(
                    test_lex(SOURCE),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::TextLiteral(TextLiteral { content: "\\0" }),
                    })]
                );
            }

            #[test]
            fn test_scan_str_escaped_hex() {
                const SOURCE: &str = "\"\\x1b\"";
                assert_eq!(
                    test_lex(SOURCE),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::TextLiteral(TextLiteral { content: "\\x1b" }),
                    })]
                );
            }

            #[test]
            fn test_scan_str_escaped_multi() {
                const SOURCE: &str = "\"\\1b\"";
                assert_eq!(
                    test_lex(SOURCE),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::TextLiteral(TextLiteral { content: "\\1b" }),
                    })]
                );
            }

            #[test]
            fn test_scan_str_escaped_invalid() {
                const SOURCE: &str = "\"\\\"";
                assert_eq!(
                    test_lex(SOURCE),
                    &[Err(ContextError::error(
                        SOURCE,
                        Some((0..SOURCE.len()).into()),
                        None,
                        DiagnosticKind::EscapedStringLiteralEnd
                    ))]
                );
            }
        }
    }
}
