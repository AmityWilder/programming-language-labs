//! Scanner tests.
//! All of these test cases are hand-written. Mostly on a very bumpy bus ride.

use crate::{
    error::{ContextError, ErrorType},
    scanner::{
        token::{Token, punc::Punctuation, value::LexValue},
        tokenize,
    },
};
use std::assert_matches;

mod scan {
    use super::*;

    #[test]
    fn test_invalid_token() {
        const SOURCE: &str = "~";
        assert_eq!(
            tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
            &[Err(ContextError {
                source: SOURCE,
                range: (0..SOURCE.len()).into(),
                macro_range: None,
                err: ErrorType::UnknownToken,
            })]
        );
    }

    /// [`TokenType::Whitespace`]
    mod whitespace {
        use super::*;

        #[test]
        fn test_scan_whitespace_single() {
            const SOURCE: &str = " ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::Whitespace,
                    mac: None
                })]
            );
        }

        #[test]
        fn test_scan_whitespace_multi() {
            const SOURCE: &str = " \n\r\t ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::Whitespace,
                    mac: None
                })]
            );
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
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::Comment,
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_line_comment_typical() {
                const SOURCE: &str = "// apple\n";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[
                        Ok(Token {
                            lex: "// apple",
                            val: LexValue::Comment,
                            mac: None
                        }),
                        Ok(Token {
                            lex: "\n",
                            val: LexValue::Whitespace,
                            mac: None
                        })
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
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::Identifier,
                    mac: None
                })]
            );
        }

        #[test]
        fn test_scan_ident_prime() {
            const SOURCE: &str = "x'";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::Identifier,
                    mac: None
                })]
            );
        }

        #[test]
        fn test_scan_ident_apostrophe() {
            const SOURCE: &str = "can't";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::Identifier,
                    mac: None
                })]
            );
        }
    }

    /// [`TokenType::NumberLiteral`]
    mod number {
        use super::*;

        /// Simple (`-?\d+(\.\d+)?`)
        mod simple {
            use super::*;

            #[test]
            fn test_scan_number_simple() {
                const SOURCE: &str = "5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(5),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_multidigit() {
                const SOURCE: &str = "35";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(35),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_underscored() {
                const SOURCE: &str = "3_5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(35),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_multi_underscored() {
                const SOURCE: &str = "3__5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(35),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_negative() {
                const SOURCE: &str = "-5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(-5),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_decimal() {
                const SOURCE: &str = "2.5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(2.5),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_multidigit_decimal() {
                const SOURCE: &str = "25.25";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(25.25),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_multidigit_decimal_negative() {
                const SOURCE: &str = "-25.25";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(-25.25),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_underscored_negative() {
                const SOURCE: &str = "2_5.2_5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(25.25),
                        mac: None
                    })]
                );
            }
        }

        /// Scientific notation (`-?\d+(\.\d+)?([eE]-?\d+)?`)
        mod sci_notation {
            use super::*;

            #[test]
            fn test_scan_number_sci_notation() {
                const SOURCE: &str = "5e0";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(5e0),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_sci_notation_underscored() {
                const SOURCE: &str = "5_5e0";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(55e0),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation() {
                const SOURCE: &str = "5e-5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(5e-5),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_pos_sci_notation() {
                const SOURCE: &str = "5e+5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(5e+5),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp() {
                const SOURCE: &str = "5e-50";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(5e-50),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_negative() {
                const SOURCE: &str = "-5e-50";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(-5e-50),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_underscored_exp() {
                const SOURCE: &str = "5e-5_0";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::FracLiteral(5e-5_0),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_excess_negative() {
                const SOURCE: &str = "-5e-5-3";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[
                        Ok(Token {
                            lex: "-5e-5",
                            val: LexValue::FracLiteral(-5e-5),
                            mac: None
                        }),
                        Ok(Token {
                            lex: "-",
                            val: LexValue::Punctuation(Punctuation::SubNeg),
                            mac: None
                        }),
                        Ok(Token {
                            lex: "3",
                            val: LexValue::SIntLiteral(3),
                            mac: None
                        })
                    ]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_double_negative() {
                const SOURCE: &str = "-5e--5";
                assert_matches!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[
                        Err(ContextError {
                            err: ErrorType::InvalidNumLiteral(_),
                            ..
                        }),
                        // without a number after the hyphen, the number literal is "-5e",
                        // but "e" isn't a valid number literal suffix
                        Ok(Token {
                            lex: "-",
                            val: LexValue::Punctuation(Punctuation::SubNeg),
                            mac: None
                        }),
                        Ok(Token {
                            lex: "-5",
                            val: LexValue::SIntLiteral(-5),
                            mac: None
                        })
                    ]
                );
            }
        }

        /// Hexadecimal (`-?0x[0-9a-fA-F]+`)
        mod hex {
            use super::*;

            #[test]
            fn test_scan_number_hex() {
                const SOURCE: &str = "0x9F";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(0x9F),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_hex_bad_digit() {
                const SOURCE: &str = "0x9G";
                assert_matches!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: _,
                        macro_range: None,
                        err: ErrorType::InvalidNumLiteral(_)
                    })]
                );
            }
        }

        /// Octal (`-?0o[0-7]+`)
        mod oct {
            use super::*;

            #[test]
            fn test_scan_number_oct() {
                const SOURCE: &str = "0o253";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(0o253),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_oct_bad_digit() {
                const SOURCE: &str = "0o258";
                assert_matches!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: _,
                        macro_range: None,
                        err: ErrorType::InvalidNumLiteral(_)
                    })]
                );
            }
        }

        /// Binary (`-?0b[01]+`)
        mod bin {
            use super::*;

            #[test]
            fn test_scan_number_bin() {
                const SOURCE: &str = "0b11011011";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::SIntLiteral(0b1101_1011),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_number_bin_wrong_digit() {
                const SOURCE: &str = "0b11011012";
                assert_matches!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: _,
                        macro_range: None,
                        err: ErrorType::InvalidNumLiteral(_)
                    })]
                );
            }
        }
    }

    mod char {
        use super::*;
        use crate::scanner::token::value::CharLiteral;

        #[test]
        fn test_scan_char_simple() {
            const SOURCE: &str = "'a'";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::CharLiteral(CharLiteral {
                        ch: 'a',
                        is_escaped: false
                    }),
                    mac: None
                })]
            );
        }

        #[test]
        fn test_scan_char_multi() {
            const SOURCE: &str = "'aa'";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError {
                    source: SOURCE,
                    range: (0..SOURCE.len()).into(),
                    macro_range: None,
                    err: ErrorType::MultiCharLiteral,
                })]
            );
        }

        #[test]
        fn test_scan_char_empty() {
            const SOURCE: &str = "''";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError {
                    source: SOURCE,
                    range: (0..SOURCE.len()).into(),
                    macro_range: None,
                    err: ErrorType::EmptyCharLiteral,
                })]
            );
        }

        #[test]
        fn test_scan_char_endless() {
            const SOURCE: &str = "' ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError {
                    source: SOURCE,
                    range: (0..SOURCE.len()).into(),
                    macro_range: None,
                    err: ErrorType::EndlessCharLiteral,
                })]
            );
        }

        mod escaped {
            use super::*;

            #[test]
            fn test_scan_char_escaped() {
                const SOURCE: &str = "'\\0'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::CharLiteral(CharLiteral {
                            ch: '\0',
                            is_escaped: true
                        }),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_char_escaped_hex() {
                const SOURCE: &str = "'\\x1b'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::CharLiteral(CharLiteral {
                            ch: '\x1b',
                            is_escaped: true
                        }),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_char_escaped_multi() {
                const SOURCE: &str = "'\\1b'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: (0..SOURCE.len()).into(),
                        macro_range: None,
                        err: ErrorType::MultiCharLiteral,
                    })]
                );
            }

            #[test]
            fn test_scan_char_escaped_invalid() {
                const SOURCE: &str = "'\\'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: (0..SOURCE.len()).into(),
                        macro_range: None,
                        err: ErrorType::EscapedCharLiteralEnd,
                    })]
                );
            }
        }
    }

    mod str {
        use super::*;
        use crate::scanner::token::value::StrLiteral;

        #[test]
        fn test_scan_str_simple() {
            const SOURCE: &str = "\"a\"";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::TextLiteral(StrLiteral { content: "a" }),
                    mac: None
                })]
            );
        }

        #[test]
        fn test_scan_str_multi() {
            const SOURCE: &str = "\"aa\"";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::TextLiteral(StrLiteral { content: "aa" }),
                    mac: None
                })]
            );
        }

        #[test]
        fn test_scan_str_empty() {
            const SOURCE: &str = "\"\"";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: SOURCE,
                    val: LexValue::TextLiteral(StrLiteral { content: "" }),
                    mac: None
                })]
            );
        }

        #[test]
        fn test_scan_str_endless() {
            const SOURCE: &str = "\" ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError {
                    source: SOURCE,
                    range: (0..SOURCE.len()).into(),
                    macro_range: None,
                    err: ErrorType::EndlessStringLiteral,
                })]
            );
        }

        mod escaped {
            use super::*;

            #[test]
            fn test_scan_str_escaped() {
                const SOURCE: &str = "\"\\0\"";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::TextLiteral(StrLiteral { content: "\\0" }),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_str_escaped_hex() {
                const SOURCE: &str = "\"\\x1b\"";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::TextLiteral(StrLiteral { content: "\\x1b" }),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_str_escaped_multi() {
                const SOURCE: &str = "\"\\1b\"";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: SOURCE,
                        val: LexValue::TextLiteral(StrLiteral { content: "\\1b" }),
                        mac: None
                    })]
                );
            }

            #[test]
            fn test_scan_str_escaped_invalid() {
                const SOURCE: &str = "\"\\\"";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: (0..SOURCE.len()).into(),
                        macro_range: None,
                        err: ErrorType::EscapedStringLiteralEnd,
                    })]
                );
            }
        }
    }
}
