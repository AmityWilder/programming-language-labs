use crate::scanner::{
    Bracket,
    symbols::{BIN_PREFIX, ESCAPE, HEX_PREFIX, OCT_PREFIX},
    token::escape_char,
    token::{Token, TokenValue},
};
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
    UnbalancedBrackets {
        expect: Option<(Bracket, Range<usize>)>,
        actual: Bracket,
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
            Self::UnbalancedBrackets { expect, actual } => {
                let mut buf = [0; char::MAX_LEN_UTF8];
                write!(
                    f,
                    "unbalanced brackets, expected `{}`, found `{}`",
                    expect.map_or("none", |(x, _)| x.close().encode_utf8(buf.as_mut_slice())),
                    actual.close()
                )
            }
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

#[derive(Clone, PartialEq, Eq)]
pub struct ContextError<'a> {
    pub source: &'a str,
    pub range: Range<usize>,
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
        let Range { start, end } = line_col_range(self.source, self.range);
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
            | ErrorType::InvalidNumLiteral(_)
            | ErrorType::UnbalancedBrackets { .. } => "LEX",
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
            ErrorType::UnbalancedBrackets { .. } => 10,
        };
        write!(f, "err[{area}{code:>03}]")
    }
}

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

            ErrorType::UnbalancedBrackets { expect, actual } => {
                if let Some(expect) = expect {
                    write!(
                        f,
                        "try inserting a `{}` before the `{}` or add a `{}` before it and after the `{}`",
                        expect.0.close(),
                        actual.close(),
                        actual.open(),
                        expect.0.open(),
                    )
                } else {
                    write!(
                        f,
                        "try removing the `{}` or add a `{}` before it",
                        actual.close(),
                        actual.open(),
                    )
                }
            }
        }
    }
}

/// Returns [`None`] if `range` is out of bounds for `src`
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
    for (idx, line) in source[line_range].lines().enumerate() {
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
                ErrorType::UnbalancedBrackets { .. } => "missing a partner",
            },
        )?;

        // info
        let items = match self.0.err {
            ErrorType::UnbalancedBrackets {
                expect: Some((_, range)),
                ..
            } => &[(range, "bracket type introduced here")],

            _ => [].as_slice(),
        };
        for &(range, explanation) in items {
            writeln!(f)?;
            line_ref(f, self.0.source, range, "\x1b[96m", '-', explanation)?;
        }

        Ok(())
    }
}

pub type TokenResult<'a, S> = Result<(Token<'a>, TokenValue<'a, S>), ContextError<'a>>;
