use super::token::{Allocated, NestedTokenValue, Token, TokenValue};
use std::range::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumLitError {
    UInt(std::num::ParseIntError),
    SInt(std::num::TryFromIntError),
    Flt(std::num::ParseFloatError),
}

impl std::fmt::Display for NumLitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UInt(e) => e.fmt(f),
            Self::SInt(e) => e.fmt(f),
            Self::Flt(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for NumLitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UInt(e) => Some(e),
            Self::SInt(e) => Some(e),
            Self::Flt(e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType<'a> {
    UnknownToken,
    EndlessBlockComment,
    EmptyCharLiteral,
    MultiCharLiteral,
    EndlessStringLiteral,
    EscapedStringLiteralEnd,
    EndlessInterpStrExpr,
    InvalidEscape(&'a str),
    InvalidNumLiteral(NumLitError),
}

impl std::fmt::Display for ErrorType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownToken => f.write_str("unknown token"),
            Self::EndlessBlockComment => {
                f.write_str("block comment opens (`/*`) but never closes (missing `*/`)")
            }
            Self::EmptyCharLiteral => f.write_str("empty character literal"),
            Self::MultiCharLiteral => f.write_str(
                "character literal may only contain one codepoint; \
                if you meant to write a string literal, use double quotes (`\"`). \
                if you meant to write an interpolated string, use graves (`` ` ``).",
            ),
            Self::EndlessStringLiteral => {
                f.write_str("string literal opens (`\"`) but never closes (missing unescaped `\"`)")
            }
            Self::EscapedStringLiteralEnd => f.write_str(
                "string literal opens (`\"`) but never closes (missing unescaped `\"`). \
                there is a closing double-quote candidate, but it is escaped (`\\\"`). \
                string literals cannot end with an unescaped backslash (`\\`), \
                it is indistinguishable from an escaped double-quote (`\\\"`)",
            ),
            Self::EndlessInterpStrExpr => f.write_str(
                "interpolated string expression opened (`${`) but never closes (missing `}`)",
            ),
            Self::InvalidEscape(s) => write!(f, "unknown character escape: {s}"),
            Self::InvalidNumLiteral(e) => write!(f, "invalid number literal: {e}"),
        }
    }
}

impl std::error::Error for ErrorType<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidNumLiteral(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error<'a> {
    pub range: Range<usize>,
    pub err: ErrorType<'a>,
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            range: Range { start, end },
            err,
        } = self;
        write!(f, "at {start}..{end}: {err}")
    }
}

impl std::error::Error for Error<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}

impl<'a> Error<'a> {
    pub const fn add_context(self, source: &'a str) -> ContextError<'a> {
        ContextError {
            source,
            range: self.range,
            err: self.err,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextError<'a> {
    pub source: &'a str,
    pub range: Range<usize>,
    pub err: ErrorType<'a>,
}

fn line_col(s: &str, position: usize) -> (usize, usize) {
    s[..position]
        .lines()
        .enumerate()
        .last()
        .map_or((0, 0), |(row, line)| {
            (/* 1-based index */ row.strict_add(1), line.len())
        })
}

impl std::fmt::Display for ContextError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            source,
            range,
            ref err,
        } = *self;
        let (start_line, start_col) = line_col(source, range.start);
        let (end_line, end_col) = line_col(source, range.end);
        write!(f, "at {start_line}:{start_col}-{end_line}:{end_col}: {err}")
    }
}

impl std::error::Error for ContextError<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}

pub type TokenResult<'a, S> = Result<(Token<'a>, TokenValue<'a, S>), ContextError<'a>>;
pub type SimpleTokenResult<'a> = Result<(Token<'a>, NestedTokenValue<'a>), ContextError<'a>>;
pub type NestedTokenResult<'a> = TokenResult<'a, Allocated<Vec<SimpleTokenResult<'a>>>>;
