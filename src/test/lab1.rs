use crate::scanner::{
    error::ErrorType,
    token::{Keyword, Punctuation, Token, TokenType, TokenValue},
    tokenize,
};
use std::assert_matches;

#[test]
fn test1() {
    let mut tokens = tokenize("let x = 5;");
    assert_eq!(
        tokens.next(),
        Some(Ok((
            Token::new("let", TokenType::Keyword),
            Some(TokenValue::Keyword(Keyword::Let))
        )))
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((Token::new(" ", TokenType::Whitespace), None)))
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((
            Token::new("x", TokenType::Identifier),
            Some(TokenValue::Direct("x"))
        ))),
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((Token::new(" ", TokenType::Whitespace), None)))
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((
            Token::new("=", TokenType::Punctuation),
            Some(TokenValue::Punctuation(Punctuation::Assign))
        ))),
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((Token::new(" ", TokenType::Whitespace), None)))
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((
            Token::new("5", TokenType::NumberLiteral),
            Some(TokenValue::UIntLiteral(5))
        ))),
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((
            Token::new(";", TokenType::Punctuation),
            Some(TokenValue::Punctuation(Punctuation::Semi))
        ))),
    );
}

#[test]
fn test_multiple_errors() {
    let mut tokens = tokenize("5a \"\\\"").map(|res| res.map_err(|e| (&e.source[e.range], e.err)));
    assert_matches!(
        tokens.next(),
        Some(Err(("5a", ErrorType::InvalidNumLiteral(_))))
    );
    assert_eq!(
        tokens.next(),
        Some(Ok((Token::new(" ", TokenType::Whitespace), None)))
    );
    assert_matches!(
        tokens.next(),
        Some(Err(("\"\\\"", ErrorType::EscapedStringLiteralEnd)))
    );
}
