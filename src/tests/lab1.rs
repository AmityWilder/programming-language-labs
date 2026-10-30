//! Scanner tests.
//! All of these test cases are hand-written. Mostly on a very bumpy bus ride.

use crate::{
    error::{ContextError, ErrorType},
    scanner::{
        token::{Lexeme, Token, punc::Punctuation, value::LexValue},
        tokenize,
    },
};
use std::assert_matches;

mod scan {
    use super::*;

    #[test]
    fn test_invalid_token() {
        const SOURCE: &str = "\\";
        assert_eq!(
            tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
            &[Err(ContextError::error(
                SOURCE,
                Some((0..SOURCE.len()).into()),
                None,
                ErrorType::UnknownToken
            ))]
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
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::Whitespace,
                })]
            );
        }

        #[test]
        fn test_scan_whitespace_multi() {
            const SOURCE: &str = " \n\r\t ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::Whitespace,
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
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::Comment,
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
                            lex: Lexeme {
                                src: "// apple",
                                mac: None,
                            },
                            val: LexValue::Comment,
                        }),
                        Ok(Token {
                            lex: Lexeme {
                                src: "\n",
                                mac: None,
                            },
                            val: LexValue::Whitespace,
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
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::Identifier,
                })]
            );
        }

        #[test]
        fn test_scan_ident_prime() {
            const SOURCE: &str = "x'";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::Identifier,
                })]
            );
        }

        #[test]
        fn test_scan_ident_apostrophe() {
            const SOURCE: &str = "can't";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok(Token {
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::Identifier,
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
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(5),
                    })]
                );
            }

            #[test]
            fn test_scan_number_multidigit() {
                const SOURCE: &str = "35";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(35),
                    })]
                );
            }

            #[test]
            fn test_scan_number_underscored() {
                const SOURCE: &str = "3_5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(35),
                    })]
                );
            }

            #[test]
            fn test_scan_number_multi_underscored() {
                const SOURCE: &str = "3__5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(35),
                    })]
                );
            }

            #[test]
            fn test_scan_number_negative() {
                const SOURCE: &str = "-5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(-5),
                    })]
                );
            }

            #[test]
            fn test_scan_number_decimal() {
                const SOURCE: &str = "2.5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(2.5),
                    })]
                );
            }

            #[test]
            fn test_scan_number_multidigit_decimal() {
                const SOURCE: &str = "25.25";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(25.25),
                    })]
                );
            }

            #[test]
            fn test_scan_number_multidigit_decimal_negative() {
                const SOURCE: &str = "-25.25";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(-25.25),
                    })]
                );
            }

            #[test]
            fn test_scan_number_underscored_negative() {
                const SOURCE: &str = "2_5.2_5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(25.25),
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
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(5e0),
                    })]
                );
            }

            #[test]
            fn test_scan_number_sci_notation_underscored() {
                const SOURCE: &str = "5_5e0";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(55e0),
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation() {
                const SOURCE: &str = "5e-5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(5e-5),
                    })]
                );
            }

            #[test]
            fn test_scan_number_pos_sci_notation() {
                const SOURCE: &str = "5e+5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(5e+5),
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp() {
                const SOURCE: &str = "5e-50";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(5e-50),
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_negative() {
                const SOURCE: &str = "-5e-50";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(-5e-50),
                    })]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_underscored_exp() {
                const SOURCE: &str = "5e-5_0";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::FlptLiteral(5e-5_0),
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
                            lex: Lexeme {
                                src: "-5e-5",
                                mac: None,
                            },
                            val: LexValue::FlptLiteral(-5e-5),
                        }),
                        Ok(Token {
                            lex: Lexeme {
                                src: "-",
                                mac: None,
                            },
                            val: LexValue::Punctuation(Punctuation::SubNeg),
                        }),
                        Ok(Token {
                            lex: Lexeme {
                                src: "3",
                                mac: None,
                            },
                            val: LexValue::SIntLiteral(3),
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
                        // without a number after the hyphen, the number literal is "-5e",
                        // but "e" isn't a valid number literal suffix
                        Err(ContextError {
                            err: ErrorType::InvalidNumLiteral(_),
                            ..
                        }),
                        // what's important is just that everything after the e ISN'T part of the flpt literal,
                        // and that it isn't used up. it doesn't matter what it actually resolves to.
                        Ok(Token {
                            lex: Lexeme {
                                src: "--",
                                mac: None,
                            },
                            val: LexValue::Punctuation(Punctuation::Decr),
                        }),
                        Ok(Token {
                            lex: Lexeme {
                                src: "5",
                                mac: None,
                            },
                            val: LexValue::SIntLiteral(5),
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
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::SIntLiteral(0x9f),
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
                        mac: None,
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
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: _,
                        mac: None,
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
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError {
                        source: SOURCE,
                        range: _,
                        mac: None,
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
                    lex: Lexeme {
                        src: SOURCE,
                        mac: None,
                    },
                    val: LexValue::CharLiteral(CharLiteral {
                        ch: 'a',
                        is_escaped: false
                    }),
                })]
            );
        }

        #[test]
        fn test_scan_char_multi() {
            const SOURCE: &str = "'aa'";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError::error(
                    SOURCE,
                    Some((0..SOURCE.len()).into()),
                    None,
                    ErrorType::MultiCharLiteral
                ))]
            );
        }

        #[test]
        fn test_scan_char_empty() {
            const SOURCE: &str = "''";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError::error(
                    SOURCE,
                    Some((0..SOURCE.len()).into()),
                    None,
                    ErrorType::EmptyCharLiteral,
                ))]
            );
        }

        #[test]
        fn test_scan_char_endless() {
            const SOURCE: &str = "' ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError::error(
                    SOURCE,
                    Some((0..SOURCE.len()).into()),
                    None,
                    ErrorType::EndlessCharLiteral,
                ))]
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
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::CharLiteral(CharLiteral {
                            ch: '\0',
                            is_escaped: true
                        }),
                    })]
                );
            }

            #[test]
            fn test_scan_char_escaped_hex() {
                const SOURCE: &str = "'\\x1b'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok(Token {
                        lex: Lexeme {
                            src: SOURCE,
                            mac: None,
                        },
                        val: LexValue::CharLiteral(CharLiteral {
                            ch: '\x1b',
                            is_escaped: true
                        }),
                    })]
                );
            }

            #[test]
            fn test_scan_char_escaped_multi() {
                const SOURCE: &str = "'\\1b'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError::error(
                        SOURCE,
                        Some((0..SOURCE.len()).into()),
                        None,
                        ErrorType::MultiCharLiteral,
                    ))]
                );
            }

            #[test]
            fn test_scan_char_escaped_invalid() {
                const SOURCE: &str = "'\\'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError::error(
                        SOURCE,
                        Some((0..SOURCE.len()).into()),
                        None,
                        ErrorType::EscapedCharLiteralEnd,
                    ))]
                );
            }
        }
    }

    mod str {
        use super::*;
        use crate::scanner::token::value::TextLiteral;

        #[test]
        fn test_scan_str_simple() {
            const SOURCE: &str = "\"a\"";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
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
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
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
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
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
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Err(ContextError::error(
                    SOURCE,
                    Some((0..SOURCE.len()).into()),
                    None,
                    ErrorType::EndlessStringLiteral,
                ))]
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
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
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
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
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
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Err(ContextError::error(
                        SOURCE,
                        Some((0..SOURCE.len()).into()),
                        None,
                        ErrorType::EscapedStringLiteralEnd
                    ))]
                );
            }
        }
    }
}
