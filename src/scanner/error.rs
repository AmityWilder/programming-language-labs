use crate::scanner::{symbols::ESCAPE, token::escape_char};

use super::token::{Token, TokenValue};
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
    EndlessCharLiteral,
    EscapedCharLiteralEnd,
    EndlessStringLiteral,
    EscapedStringLiteralEnd,
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
            Self::MultiCharLiteral => {
                f.write_str("character literal may only contain one codepoint")
            }
            Self::EndlessCharLiteral | Self::EscapedCharLiteralEnd => {
                f.write_str("char literal opens (`'`) but never closes (missing unescaped `'`)")
            }
            Self::EndlessStringLiteral | Self::EscapedStringLiteralEnd => {
                f.write_str("string literal opens (`\"`) but never closes (missing unescaped `\"`)")
            }
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

    pub const fn code(&self) -> ContextErrorCode<'_, 'a> {
        ContextErrorCode(self)
    }

    pub const fn help(&self) -> ContextErrorHelp<'_, 'a> {
        ContextErrorHelp(self)
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
pub struct ContextErrorCode<'a, 'b>(&'b ContextError<'a>);

impl std::fmt::Display for ContextErrorCode<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let area = match self.0.err {
            ErrorType::UnknownToken
            | ErrorType::EndlessBlockComment
            | ErrorType::EmptyCharLiteral
            | ErrorType::MultiCharLiteral
            | ErrorType::EndlessCharLiteral
            | ErrorType::EscapedCharLiteralEnd
            | ErrorType::EndlessStringLiteral
            | ErrorType::EscapedStringLiteralEnd
            | ErrorType::InvalidEscape(_)
            | ErrorType::InvalidNumLiteral(_) => "LEX",
        };
        let code = match self.0.err {
            ErrorType::UnknownToken => 0,
            ErrorType::EndlessBlockComment => 1,
            ErrorType::EmptyCharLiteral => 2,
            ErrorType::MultiCharLiteral => 3,
            ErrorType::EndlessCharLiteral => 4,
            ErrorType::EscapedCharLiteralEnd => 5,
            ErrorType::EndlessStringLiteral => 6,
            ErrorType::EscapedStringLiteralEnd => 7,
            ErrorType::InvalidEscape(_) => 8,
            ErrorType::InvalidNumLiteral(_) => 9,
        };
        write!(f, "err[{area}{code:>03}]")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextErrorHelp<'a, 'b>(&'b ContextError<'a>);

impl std::fmt::Display for ContextErrorHelp<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let src = &self.0.source[self.0.range];
        match self.0.err {
            ErrorType::UnknownToken => write!(f, "remove the character"),

            ErrorType::EndlessBlockComment => f.write_str("try adding `*/`"),

            ErrorType::EmptyCharLiteral => f.write_str(
                "chars can't be empty, try replacing `''` with `\"\"` or insert a character",
            ),

            ErrorType::MultiCharLiteral => {
                let inner = src
                    .strip_circumfix("'", "'")
                    .expect("char literals should include delimiters");
                let (first, rest) = inner.split_at(match escape_char(inner) {
                    Some((len, _)) => len,
                    // normal character
                    None => inner.chars()
                        .next()
                        .expect("should have at least one character if MultiCharLiteral instead of EmptyCharLiteral")
                        .len_utf8(),
                });
                write!(
                    f,
                    "try removing the character(s) after `{first}` (remove trailing `{rest}`) \
                     or change this to a string (\"{inner}\")"
                )
            }

            ErrorType::EndlessCharLiteral => f.write_str("try adding a `'` to the end of the char"),

            ErrorType::EscapedCharLiteralEnd => {
                let substr = src[..src
                    .find("\\'")
                    .expect("should be EscapedCharLiteralEnd if this is not present")]
                    .strip_prefix('\"')
                    .expect("string literal should include delimiter");
                write!(
                    f,
                    "there is a closing single-quote candidate, but it is escaped (`\\'`). \
                     char literals cannot end with an unescaped backslash (`\\`), \
                     it is indistinguishable from an escaped single-quote (`\\'`). \
                     try adding a `'` to the end of the string or remove the `\\` from `\\'` to make the string `'{substr}'`"
                )
            }

            ErrorType::EndlessStringLiteral => {
                f.write_str("try adding a `\"` to the end of the string")
            }

            ErrorType::EscapedStringLiteralEnd => {
                let substr = src[..src
                    .find("\\\"")
                    .expect("should be EndlessStringLiteral if this is not present")]
                    .strip_prefix('"')
                    .expect("string literal should include delimiter");
                write!(
                    f,
                    "there is a closing double-quote candidate, but it is escaped (`\\\"`). \
                     char literals cannot end with an unescaped backslash (`\\`), \
                     it is indistinguishable from an escaped double-quote (`\\\"`). \
                     try adding a `\"` to the end of the string or remove the `\\` from `\\\"` to make the string `\"{substr}\"`"
                )
            }

            ErrorType::InvalidEscape(esc) => {
                let mut iter = esc.chars();
                iter.next()
                    .filter(|ch| *ch == ESCAPE)
                    .expect("InvalidEscape should include `\\`");
                let ch = iter.next().expect(
                    "should have at least 2 characters or else be an EscapedStringLiteralEnd",
                );

                if ch.is_alphabetic() {
                    f.write_str(r"`\a`, `\b`, `\e`, `\f`, `\n`, `\r`, `\t`, and `\v` are the only supported ASCII letters that can be escape sequences")
                } else if ch.is_numeric() {
                    f.write_str("only ascii digits (0-9) are supported for decimal (base-10) numeric escape sequences. ")
                } else if ch == 'x' {
                    let n = iter.take(2).filter(char::is_ascii_hexdigit).count();
                    assert!(n < 2, "why is this an error?");
                    write!(
                        f,
                        "`\\x` should be followed by 2 hexadecimal digits ([0-9a-fA-F]), this escape sequence has {n}"
                    )
                } else if ch == 'o' {
                    let n = iter.take(3).filter(|ch| ch.is_digit(8)).count();
                    assert!(n < 3, "why is this an error?");
                    write!(
                        f,
                        "`\\o` should be followed by 3 octal digits ([0-7]), this escape sequence has {n}"
                    )
                } else {
                    todo!("unknown pattern")
                }
            }

            ErrorType::InvalidNumLiteral(_) => {
                // TODO: "try removing [...]"
                f.write_str("I'm not sure how to help with this yet")
            }
        }
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
                n.checked_add(1 /* \n and \r are both ASCII */).expect(
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
