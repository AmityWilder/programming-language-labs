//! Errors regarding code validity

use crate::scanner::{
    Bracket,
    symbols::{BIN_PREFIX, ESCAPE, HEX_PREFIX, OCT_PREFIX},
    token::escape_char,
    token::{Token, TokenValue},
};
use std::range::Range;

/// Invalid number literal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumLitError {
    /// Unsigned integer
    UInt(std::num::ParseIntError),
    /// Signed integer
    SInt(std::num::TryFromIntError),
    /// Floating point
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

/// The kind of error describing a [`ContextError`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType<'a> {
    /// Token type could not be identified from the initial character, and so is not a valid token
    UnknownToken,
    /// A block comment has no `*/` to end it
    EndlessBlockComment,
    /// A character literal that is just `''`
    EmptyCharLiteral,
    /// A character literal with multiple codepoints
    MultiCharLiteral,
    /// A character literal has no `'` to end it
    EndlessCharLiteral,
    /// A character literal has no `'` to end it, but contains a `\'`
    EscapedCharLiteralEnd,
    /// A string literal has no `"` to end it
    EndlessStringLiteral,
    /// A string literal has no `"` to end it, but contains a `\"`
    EscapedStringLiteralEnd,
    /// A string/character literal contains an escape sequence (identified by a `\`) that does not exist
    InvalidEscape(&'a str),
    /// A number literal could not be evaluated as a number
    InvalidNumLiteral(NumLitError),
    /// A closing bracket is of the wrong type for the open bracket at its depth
    IncorrectCloseBracket {
        /// The bracket type being expected based on the opening side
        expect: (Bracket, Range<usize>),
        /// The bracket type that was found
        actual: Bracket,
    },
    /// A closing bracket was found with no open bracket
    ExcessCloseBracket {
        /// The bracket type that was found
        actual: Bracket,
    },
    /// An open bracket was found with no close bracket
    MissingCloseBracket {
        /// The bracket type being expected based on the opening side
        expect: (Bracket, Range<usize>),
    },
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
            Self::IncorrectCloseBracket { expect, actual } => write!(
                f,
                "incorrect close bracket: expected `{}`, found `{}`",
                expect.0.close(),
                actual.close()
            ),
            Self::ExcessCloseBracket { actual } => write!(
                f,
                "too many close brackets: expected none, found `{}`",
                actual.close()
            ),
            Self::MissingCloseBracket { expect } => write!(
                f,
                "missing close bracket: expected `{}`, found none",
                expect.0.close()
            ),
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

/// A code error with the range of the error in the source code
#[derive(Clone, PartialEq, Eq)]
pub struct ContextError<'a> {
    /// A string view of the FULL, ENTIRE source code
    pub source: &'a str,
    /// The range in [`Self::source`] of precisely where the error occurred
    pub range: Range<usize>,
    /// The exact error that was found
    pub err: ErrorType<'a>,
}

impl std::fmt::Debug for ContextError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextError")
            .field("source[range]", &&self.source[self.range])
            .field("range", &self.range)
            .field("err", &self.err)
            .finish()
    }
}

/// The line (row) and position (column) of a position in a string
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LineCol {
    /// The row
    ///
    /// 1-based index
    pub line: usize,

    /// The position within the row
    ///
    /// 0-based index
    // TODO: why are they different? would it make sense for both to be 1-based?
    pub col: usize,
}

impl std::fmt::Display for LineCol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { line, col } = self;
        write!(f, "{line}:{col}")
    }
}

fn line_col(s: &str, position: usize) -> LineCol {
    s[..position]
        .split('\n') // assumes \n\r will never happen, except for \r\n\r\n
        .enumerate()
        .last()
        .map_or_default(|(row, line)| LineCol {
            line: row.strict_add(1), // +1 to convert from 0-based to 1-based
            col: line.len(),
        })
}

fn line_col_range(s: &str, range: Range<usize>) -> Range<LineCol> {
    (line_col(s, range.start)..line_col(s, range.end)).into()
}

impl<'a> ContextError<'a> {
    /// Returns a struct that implements [`std::fmt::Display`] to show detailed line reference information
    #[must_use]
    pub const fn render(&self) -> RenderedContextError<'_, 'a> {
        RenderedContextError(self)
    }

    /// Returns a struct that implements [`std::fmt::Display`] to show the error code (number)
    #[must_use]
    pub const fn code(&self) -> ContextErrorCode<'_, 'a> {
        ContextErrorCode(self)
    }

    /// Returns a struct that implements [`std::fmt::Display`] to show tips for resolving the error
    #[must_use]
    pub const fn help(&self) -> ContextErrorHelp<'_, 'a> {
        ContextErrorHelp(self)
    }
}

impl std::fmt::Display for ContextError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Range { start, end } = line_col_range(self.source, self.range);
        write!(f, "at {start}-{end}: {}", self.err)
    }
}

impl std::error::Error for ContextError<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}

/// [`std::fmt::Display`] the error code (number)
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
            | ErrorType::InvalidNumLiteral(_)
            | ErrorType::IncorrectCloseBracket { .. }
            | ErrorType::ExcessCloseBracket { .. }
            | ErrorType::MissingCloseBracket { .. } => "LEX",
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
            ErrorType::IncorrectCloseBracket { .. } => 10,
            ErrorType::ExcessCloseBracket { .. } => 11,
            ErrorType::MissingCloseBracket { .. } => 12,
        };
        write!(f, "err[{area}{code:>03}]")
    }
}

/// [`std::fmt::Display`] tips for resolving an error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextErrorHelp<'a, 'b>(&'b ContextError<'a>);

impl std::fmt::Display for ContextErrorHelp<'_, '_> {
    #[expect(
        clippy::too_many_lines,
        reason = "it would be even more complicated to make a separate function for each of these"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let src: &str = &self.0.source[self.0.range];
        match &self.0.err {
            ErrorType::UnknownToken => f.write_str("try removing the character"),

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
                    "there is a closing single-quote candidate, but it is escaped (`\\'`). \n\
                     char literals cannot end with an unescaped backslash (`\\`), \
                     it is indistinguishable from an escaped single-quote (`\\'`). \n\
                     try adding a `'` to the end of the string or remove the `\\` from `\\'` \
                     to make the string `'{substr}'`"
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
                    "there is a closing double-quote candidate, but it is escaped (`\\\"`).\n\
                     string literals cannot end with an unescaped backslash (`\\`), \
                     it is indistinguishable from an escaped double-quote (`\\\"`).\n\
                     try adding a `\"` to the end of the string or remove the `\\` from `\\\"` \
                     to make the string `\"{substr}\"`"
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

                if ch == 'x' {
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
                } else if ch.is_alphabetic() {
                    f.write_str("`\\a`, `\\b`, `\\e`, `\\f`, `\\n`, `\\r`, `\\t`, and `\\v` are the only supported \
                                ASCII letters that can be escape sequences")
                } else if ch.is_numeric() {
                    f.write_str("only ascii digits (0-9) are supported for decimal (base-10) numeric escape sequences")
                } else {
                    f.write_str(
                        "supported escape sequences: `\\a`, `\\b`, `\\e`, `\\f`, `\\n`, `\\r`, `\\t`, `\\v`, `\\0`-`\\9`,\n\\
                        `\\x##` (where # is a hexadecimal digit), `\\o###` (where # is an octal digit)"
                    )
                }
            }

            ErrorType::InvalidNumLiteral(e) => {
                use std::num::IntErrorKind;
                match e {
                    NumLitError::UInt(e) => match e.kind() {
                        IntErrorKind::Empty => unreachable!(
                            "tokenizer should not emit number tokens that have no number"
                        ),

                        IntErrorKind::InvalidDigit => {
                            let (suffix, base_name) =
                                if let Some(digits) = src.strip_prefix(HEX_PREFIX) {
                                    digits
                                        .find(|ch: char| !ch.is_ascii_hexdigit())
                                        .map(|n| (&digits[n..], "hexadecimal"))
                                } else if let Some(digits) = src.strip_prefix(OCT_PREFIX) {
                                    digits
                                        .find(|ch: char| !ch.is_digit(8))
                                        .map(|n| (&digits[n..], "octal"))
                                } else if let Some(digits) = src.strip_prefix(BIN_PREFIX) {
                                    digits
                                        .find(|ch: char| !ch.is_digit(2))
                                        .map(|n| (&digits[n..], "binary"))
                                } else {
                                    src.find(|ch: char| !ch.is_ascii_digit())
                                        .map(|n| (&src[n..], "decimal"))
                                }
                                .expect("digits are unexpectedly valid");
                            write!(
                                f,
                                "the suffix `{suffix}` is not valid for {base_name} integer literals",
                            )
                        }

                        IntErrorKind::PosOverflow => write!(
                            f,
                            "the largest supported unsigned integer value is {}",
                            usize::MAX
                        ),

                        _ => unimplemented!(),
                    },

                    NumLitError::SInt(e) => match e.kind() {
                        IntErrorKind::Empty => unreachable!(
                            "tokenizer should not emit number tokens that have no number"
                        ),

                        IntErrorKind::InvalidDigit => {
                            let digits = src.strip_prefix('-').unwrap_or(src);
                            let (suffix, base_name) =
                                if let Some(digits) = digits.strip_prefix(HEX_PREFIX) {
                                    digits
                                        .find(|ch: char| !ch.is_ascii_hexdigit())
                                        .map(|n| (&digits[n..], "hexadecimal"))
                                } else if let Some(digits) = digits.strip_prefix(OCT_PREFIX) {
                                    digits
                                        .find(|ch: char| !ch.is_digit(8))
                                        .map(|n| (&digits[n..], "octal"))
                                } else if let Some(digits) = digits.strip_prefix(BIN_PREFIX) {
                                    digits
                                        .find(|ch: char| !ch.is_digit(2))
                                        .map(|n| (&digits[n..], "binary"))
                                } else {
                                    digits
                                        .find(|ch: char| !ch.is_ascii_digit())
                                        .map(|n| (&digits[n..], "decimal"))
                                }
                                .expect("digits are unexpectedly valid");
                            write!(
                                f,
                                "the suffix `{suffix}` is not valid for {base_name} integer literals",
                            )
                        }

                        IntErrorKind::PosOverflow => write!(
                            f,
                            "the largest supported signed integer value is {}",
                            isize::MAX
                        ),

                        IntErrorKind::NegOverflow => write!(
                            f,
                            "the smallest supported signed integer value is {}",
                            isize::MIN
                        ),

                        _ => unimplemented!(),
                    },

                    NumLitError::Flt(_) => f.write_str("I'm not sure how to help with this yet"),
                }
            }

            ErrorType::IncorrectCloseBracket { expect, actual } => write!(
                f,
                "try inserting a `{}` before the `{}`, add a `{}` before it and after the `{}`, or remove either the `{}` or `{}`",
                expect.0.close(),
                actual.close(),
                actual.open(),
                expect.0.open(),
                expect.0.open(),
                actual.close(),
            ),
            ErrorType::ExcessCloseBracket { actual } => write!(
                f,
                "try removing the `{}` or add a `{}` before it",
                actual.close(),
                actual.open(),
            ),
            ErrorType::MissingCloseBracket { expect } => write!(
                f,
                "try inserting a `{}` or remove the `{}`",
                expect.0.close(),
                expect.0.open(),
            ),
        }
    }
}

/// Returns [`None`] if `range` is out of bounds for `src`
#[must_use]
pub fn line_containing(src: &str, range: Range<usize>) -> Option<Range<usize>> {
    let line_start = src
        .get(..range.start)?
        .rfind(['\n', '\r'])
        .map_or(range.start, |n| {
            // SAFETY: `n` is the position of the start of a 1-byte ASCII char, therefore we can
            // add the length of that char (1 byte) to get the end, which is at most src.len().
            unsafe { n.unchecked_add(1) }
        });
    let line_end = src
        .get(range.end..)?
        .find(['\n', '\r'])
        .map_or(src.len(), |n| {
            // SAFETY: `n` is a position in `source[range.end..]`, therefore `range.end + n`
            // is a position in `source[..]`, which must be in memory whose len therefore fits in usize.
            unsafe { n.unchecked_add(range.end) }
        });
    Some((line_start..line_end).into())
}

/// [`std::fmt::Display`] advanced error information with line references for a context error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedContextError<'a, 'b>(&'b ContextError<'a>);

fn line_ref(
    f: &mut std::fmt::Formatter<'_>,
    source: &str,
    range: Range<usize>,
    underline_style: &str,
    underline_char: char,
    msg: &str,
) -> std::fmt::Result {
    const PRE_NUM: &str = "   \x1b[94m";
    const POST_NUM: &str = " |\x1b[0m  ";

    let Range { start, end } = line_col_range(source, range);
    let line_range = line_containing(source, range).expect("range should be a range in source");
    // bigger numbers have more digits so the last line number should have the most digits
    let num_width = end.line.to_string().len(); // ew, an allocation just to count the digits :c
    writeln!(f, "{PRE_NUM}{:>num_width$}{POST_NUM}", "")?;
    let num_lines = end
        .line
        .checked_sub(start.line)
        .expect("range should be ascending order");
    for (idx, line) in source[line_range].split('\n').enumerate() {
        let line_number = start
            .line
            .checked_add(idx)
            .expect("the number of lines should be at most the number of bytes in source");
        let start_col = if idx == 0 { start.col } else { 0 };
        let end_col = if idx == num_lines {
            end.col
        } else {
            line.len()
        };

        writeln!(f, "{PRE_NUM}{line_number:>num_width$}{POST_NUM}{line}")?;
        write!(f, "{PRE_NUM}{:>num_width$}{POST_NUM}", "")?;
        for _ in 0..start_col {
            f.write_str(" ")?;
        }
        f.write_str(underline_style)?;
        for _ in start_col..end_col {
            write!(f, "{underline_char}")?;
        }
        if idx == num_lines {
            write!(f, " {msg}")?;
        }
        writeln!(f, "\x1b[0m")?;
    }
    Ok(())
}

impl std::fmt::Display for RenderedContextError<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // error

        line_ref(
            f,
            self.0.source,
            self.0.range,
            "\x1b[91m",
            '^',
            match self.0.err {
                ErrorType::UnknownToken => "what is this?",
                ErrorType::EndlessBlockComment
                | ErrorType::EndlessCharLiteral
                | ErrorType::EndlessStringLiteral => "never ends",
                ErrorType::EmptyCharLiteral => "empty",
                ErrorType::MultiCharLiteral => "a char should be 1 char",
                ErrorType::EscapedCharLiteralEnd | ErrorType::EscapedStringLiteralEnd => {
                    "never ends, unless you remove the `\\`"
                }
                ErrorType::InvalidEscape(_) => "has an invalid escape sequence",
                ErrorType::InvalidNumLiteral(_) => "not a valid number",
                ErrorType::IncorrectCloseBracket { .. } => "incorrect partner",
                ErrorType::ExcessCloseBracket { .. } | ErrorType::MissingCloseBracket { .. } => {
                    "missing a partner"
                }
            },
        )?;

        // info
        let items = match self.0.err {
            ErrorType::IncorrectCloseBracket {
                expect: (_, range), ..
            }
            | ErrorType::MissingCloseBracket { expect: (_, range) } => {
                &[(range, "bracket type introduced here")]
            }

            _ => [].as_slice(),
        };
        for &(range, explanation) in items {
            writeln!(f)?;
            line_ref(f, self.0.source, range, "\x1b[96m", '-', explanation)?;
        }

        Ok(())
    }
}

/// A [`Token`] and its [`TokenValue`], or a [`ContextError`]
pub type TokenResult<'a, S> = Result<(Token<'a>, TokenValue<'a, S>), ContextError<'a>>;
