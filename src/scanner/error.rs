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
            Self::InvalidEscape(s) => write!(f, "unknown character escape: {s:?}"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LineCol {
    /// 1-based index
    line: usize,
    col: usize,
}

impl std::fmt::Display for LineCol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { line, col } = self;
        write!(f, "{line}:{col}")
    }
}

fn line_col(s: &str, position: usize) -> LineCol {
    s[..position]
        .lines()
        .enumerate()
        .last()
        .map_or_default(|(row, line)| LineCol {
            line: row.strict_add(1),
            col: line.len(),
        })
}

impl<'a> ContextError<'a> {
    pub fn position(&self) -> Range<LineCol> {
        (line_col(self.source, self.range.start)..line_col(self.source, self.range.end)).into()
    }

    pub const fn render(&self) -> RenderedContextError<'_, 'a> {
        RenderedContextError(self)
    }
}

impl std::fmt::Display for ContextError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Range { start, end } = self.position();
        write!(f, "at {start}-{end}: {}", self.err)
    }
}

impl std::error::Error for ContextError<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedContextError<'a, 'b>(&'b ContextError<'a>);

impl std::fmt::Display for RenderedContextError<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const PRE_NUM: &str = "   \x1b[94m";
        const POST_NUM: &str = " |\x1b[0m  ";
        let Range { start, end } = self.0.position();
        let line_start = self.0.source[..self.0.range.start]
            .rfind(['\n', '\r'])
            .map_or(0, |n| {
                n.checked_add(1 /* ASCII */).expect(
                    "`n` is the position of the start of the char, \
                    we should be able to add the length of that char",
                )
            });
        let line_end =
            self.0.source[self.0.range.end..]
                .find(['\n', '\r'])
                .map_or(self.0.source.len(), |n| {
                    n.checked_add(self.0.range.end).expect(
                        "`n` should be an offset from self.0.range.end in source, \
                        which is a string in memory whose len must fit in usize",
                    )
                });
        // numbers get bigger as they get bigger, so the last line should be the biggest number
        let num_width = end.line.to_string().len(); // ew, an allocation just to count the digits :c
        writeln!(f, "{PRE_NUM}{:>num_width$}{POST_NUM}", "")?;
        for (idx, line) in self.0.source[line_start..line_end].lines().enumerate() {
            let line_number = start
                .line
                .checked_add(idx)
                .expect("the number of lines should be at most the number of bytes in source");
            let is_first_line = idx == 0;
            let is_last_line = idx
                == end
                    .line
                    .checked_sub(start.line)
                    .expect("range should be ascending order");
            let start_col = if is_first_line { start.col } else { 0 };
            let end_col = if is_last_line { end.col } else { line.len() };

            writeln!(f, "{PRE_NUM}{line_number:>num_width$}{POST_NUM}{line}")?;
            write!(f, "{PRE_NUM}{:>num_width$}{POST_NUM}", "")?;
            for _ in 0..start_col {
                f.write_str(" ")?;
            }
            f.write_str("\x1b[91m")?;
            for _ in start_col..end_col {
                f.write_str("~")?;
            }
            f.write_str("\x1b[0m\n")?;
        }
        Ok(())
    }
}

pub type TokenResult<'a, S> = Result<(Token<'a>, TokenValue<'a, S>), ContextError<'a>>;
pub type SimpleTokenResult<'a> = Result<(Token<'a>, NestedTokenValue<'a>), ContextError<'a>>;
pub type NestedTokenResult<'a> = TokenResult<'a, Allocated<Vec<SimpleTokenResult<'a>>>>;
