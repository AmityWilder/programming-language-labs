//! All of these test cases are hand-written. Mostly on a very bumpy bus ride

use crate::scanner::{
    error::{ContextError, ErrorType},
    token::{Token, TokenType, TokenValue},
    tokenize,
};

/// [`crate::scanner::Scanner`]
/// - [x] [`TokenType::Whitespace`]
/// - [x] [`TokenType::Comment`]
/// - [-] [`TokenType::NumberLiteral`]
/// - [ ] [`TokenType::CharLiteral`]
/// - [ ] [`TokenType::StringLiteral`]
/// - [x] [`TokenType::Identifier`]
/// - [ ] [`TokenType::Callable`]
/// - [ ] [`TokenType::Keyword`]
/// - [ ] [`TokenType::CtrlKeyword`]
/// - [ ] [`TokenType::Punctuation`]
mod scan {
    use super::*;

    /// [`TokenType::Whitespace`]
    mod whitespace {
        use super::*;

        #[test]
        fn test_scan_whitespace_single() {
            const SOURCE: &str = " ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok((
                    Token {
                        src: SOURCE,
                        ty: TokenType::Whitespace,
                    },
                    TokenValue::Ignore
                ))]
            );
        }

        #[test]
        fn test_scan_whitespace_multi() {
            const SOURCE: &str = " \n\r\t ";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok((
                    Token {
                        src: SOURCE,
                        ty: TokenType::Whitespace,
                    },
                    TokenValue::Ignore
                ))]
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
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::Comment,
                        },
                        TokenValue::Ignore
                    ))]
                );
            }

            #[test]
            fn test_scan_line_comment_typical() {
                assert_eq!(
                    tokenize("// apple\n").collect::<Vec<_>>().as_slice(),
                    &[
                        Ok((
                            Token {
                                src: "// apple",
                                ty: TokenType::Comment,
                            },
                            TokenValue::Ignore
                        )),
                        Ok((
                            Token {
                                src: "\n",
                                ty: TokenType::Whitespace,
                            },
                            TokenValue::Ignore
                        ))
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
                &[Ok((
                    Token {
                        src: SOURCE,
                        ty: TokenType::Identifier,
                    },
                    TokenValue::Direct(SOURCE)
                ))]
            );
        }

        #[test]
        fn test_scan_ident_prime() {
            const SOURCE: &str = "x'";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok((
                    Token {
                        src: SOURCE,
                        ty: TokenType::Identifier,
                    },
                    TokenValue::Direct(SOURCE)
                ))]
            );
        }

        #[test]
        fn test_scan_ident_apostrophe() {
            const SOURCE: &str = "can't";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok((
                    Token {
                        src: SOURCE,
                        ty: TokenType::Identifier,
                    },
                    TokenValue::Direct(SOURCE)
                ))]
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
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::UIntLiteral(5)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_multidigit() {
                const SOURCE: &str = "35";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::UIntLiteral(35)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_negative() {
                const SOURCE: &str = "-5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::SIntLiteral(-5)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_decimal() {
                const SOURCE: &str = "2.5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::FltLiteral(2.5)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_multidigit_decimal() {
                const SOURCE: &str = "25.25";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::FltLiteral(25.25)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_multidigit_decimal_negative() {
                const SOURCE: &str = "-25.25";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::FltLiteral(-25.25)
                    ))]
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
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::FltLiteral(5e0)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation() {
                const SOURCE: &str = "5e-5";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::FltLiteral(5e-5)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp() {
                const SOURCE: &str = "5e-50";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::FltLiteral(5e-50)
                    ))]
                );
            }

            #[test]
            fn test_scan_number_neg_sci_notation_multidigit_exp_negative() {
                const SOURCE: &str = "-5e-50";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::FltLiteral(-5e-50)
                    ))]
                );
            }
        }

        /// Hexadecimal (`-?0x[0-9a-fA-F]{2}`)
        mod hex {
            use super::*;

            #[test]
            fn test_scan_number_oct() {
                const SOURCE: &str = "0x9F";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::UIntLiteral(0x9F)
                    ))]
                );
            }
        }

        /// Octal (`-?0o[0-7]{3}`)
        mod oct {
            use super::*;

            #[test]
            fn test_scan_number_oct() {
                const SOURCE: &str = "0o253";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::UIntLiteral(0o253)
                    ))]
                );
            }
        }

        /// Binary (`-?0b[01]{8}`)
        mod bin {
            use super::*;

            #[test]
            fn test_scan_number_bin() {
                const SOURCE: &str = "0b11011011";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::NumberLiteral,
                        },
                        TokenValue::UIntLiteral(0b1101_1011)
                    ))]
                );
            }
        }
    }

    mod char {
        use super::*;
        use crate::scanner::token::CharLiteral;

        #[test]
        fn test_scan_char_simple() {
            const SOURCE: &str = "'a'";
            assert_eq!(
                tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                &[Ok((
                    Token {
                        src: SOURCE,
                        ty: TokenType::CharLiteral,
                    },
                    TokenValue::CharLiteral(CharLiteral {
                        ch: 'a',
                        is_escaped: false
                    }),
                ))]
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
                    err: ErrorType::EmptyCharLiteral,
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
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::CharLiteral,
                        },
                        TokenValue::CharLiteral(CharLiteral {
                            ch: '\0',
                            is_escaped: true
                        })
                    ))]
                );
            }

            #[test]
            fn test_scan_char_escaped_hex() {
                const SOURCE: &str = "'\\x1b'";
                assert_eq!(
                    tokenize(SOURCE).collect::<Vec<_>>().as_slice(),
                    &[Ok((
                        Token {
                            src: SOURCE,
                            ty: TokenType::CharLiteral,
                        },
                        TokenValue::CharLiteral(CharLiteral {
                            ch: '\x1b',
                            is_escaped: true
                        })
                    ))]
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
                        err: ErrorType::EscapedStringLiteralEnd,
                    })]
                );
            }
        }
    }
}
