use super::*;
use crate::scanner::token::Lexeme;

// Escape sequences are the only difficult ones

#[test]
fn test_char_literal_unescaped() {
    let token = Token {
        lex: Lexeme {
            src: "'a'",
            mac: None,
        },
        val: LexValue::CharLiteral(CharLiteral {
            ch: 'a',
            is_escaped: false,
        }),
    };
    let syntax: Vec<_> = highlight([token]).collect();
    assert_eq!(&syntax, &[("'a'", Syntax::CharLiteral)]);
}

#[test]
fn test_char_literal_escaped() {
    let token = Token {
        lex: Lexeme {
            src: r"'\0'",
            mac: None,
        },
        val: LexValue::CharLiteral(CharLiteral {
            ch: '\0',
            is_escaped: true,
        }),
    };
    let syntax: Vec<_> = highlight([token]).collect();
    assert_eq!(
        &syntax,
        &[
            ("'", Syntax::CharLiteral),
            (r"\0", Syntax::EscapeSeq),
            ("'", Syntax::CharLiteral),
        ]
    );
}

#[test]
fn test_text_literal_unescaped() {
    let token = Token {
        lex: Lexeme {
            src: r#""apple""#,
            mac: None,
        },
        val: LexValue::TextLiteral(TextLiteral { content: "apple" }),
    };
    let syntax: Vec<_> = highlight([token]).collect();
    assert_eq!(&syntax, &[(r#""apple""#, Syntax::TextLiteral),]);
}

#[test]
fn test_text_literal_escaped() {
    let token = Token {
        lex: Lexeme {
            src: r#""\0""#,
            mac: None,
        },
        val: LexValue::TextLiteral(TextLiteral { content: r"\0" }),
    };
    let syntax: Vec<_> = highlight([token]).collect();
    assert_eq!(
        &syntax,
        &[
            ("\"", Syntax::TextLiteral),
            (r"\0", Syntax::EscapeSeq),
            ("\"", Syntax::TextLiteral),
        ]
    );
}

#[test]
fn test_text_literal_contains_escaped() {
    let token = Token {
        lex: Lexeme {
            src: r#""pre\0post""#,
            mac: None,
        },
        val: LexValue::TextLiteral(TextLiteral {
            content: r"pre\0post",
        }),
    };
    let syntax: Vec<_> = highlight([token]).collect();
    assert_eq!(
        &syntax,
        &[
            ("\"pre", Syntax::TextLiteral),
            (r"\0", Syntax::EscapeSeq),
            ("post\"", Syntax::TextLiteral),
        ]
    );
}

#[test]
fn test_text_literal_multi_escape_nonadjacent() {
    let token = Token {
        lex: Lexeme {
            src: r#""pre\"mid\"post""#,
            mac: None,
        },
        val: LexValue::TextLiteral(TextLiteral {
            content: r#"pre\"mid\"post"#,
        }),
    };
    let syntax: Vec<_> = highlight([token]).collect();
    assert_eq!(
        &syntax,
        &[
            ("\"pre", Syntax::TextLiteral),
            (r#"\""#, Syntax::EscapeSeq),
            ("mid", Syntax::TextLiteral),
            (r#"\""#, Syntax::EscapeSeq),
            ("post\"", Syntax::TextLiteral),
        ]
    );
}

#[test]
fn test_text_literal_multi_escape_adjacent() {
    let token = Token {
        lex: Lexeme {
            src: r#""pre\"\"post""#,
            mac: None,
        },
        val: LexValue::TextLiteral(TextLiteral {
            content: r#"pre\"\"post"#,
        }),
    };
    let syntax: Vec<_> = highlight([token]).collect();
    assert_eq!(
        &syntax,
        &[
            ("\"pre", Syntax::TextLiteral),
            (r#"\"\""#, Syntax::EscapeSeq),
            ("post\"", Syntax::TextLiteral),
        ]
    );
}
