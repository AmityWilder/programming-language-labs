//! Errors regarding code validity

use crate::scanner::{
    Bracket,
    symbols::{
        BIN_PREFIX, BLOCK_COMMENT_CLOSE, CHAR_DELIM, ESCAPE, HEX_PREFIX, OCT_PREFIX, STR_DELIM,
    },
    token::{Token, escape_char},
};
use std::range::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Article {
    A,
    An,
}

impl std::fmt::Display for Article {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Article::A => "a",
            Article::An => "an",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Expecting {
    pub expect: &'static str,

    /// Determines "a/an".
    ///
    /// **Note:** This cannot be determined programatically by whether a word is prefixed with a vowel.
    ///
    /// 1. Sometimes words start with a silent consonant followed by a not-silent vowel.
    ///
    ///     **Examples:**
    ///     - honest ("on-est")
    ///     - hour ("our")
    ///     - heir ("air")
    ///     - honor ("on-or")
    ///
    /// 2. Additionally, letters saying their names (such as initialisms) may be pronounced starting
    ///    with a vowel despite *being* a consonant.
    ///
    ///     **Examples:**
    ///     - F ("eff")
    ///     - H ("ayche")
    ///     - L ("el")
    ///     - M ("em")
    ///     - N ("en")
    ///     - R ("are")
    ///     - S ("ess")
    ///     - X ("ecks")
    ///
    /// 3. And to make it even more confusing, sometimes vowels will make a **consonant** sound.
    ///
    ///     **Examples:**
    ///     - unit ("you-nit"; 'u' is made to say its name by 'i' on the other side of 'n')
    ///     - utility ("you-till-itty"; 'u' is made to say its name by 'i' on the other side of 't')
    ///     - one ("won"; I don't even know why it's pronounced this way)
    ///
    /// And of course there are limitless exceptions when it comes to English, because while all
    /// languages are formulated by culture rather than committees, English in particular was
    /// formulated by three separate cultures all doing their own thing independently before
    /// deciding to mash all their languages together with little regard for bystanders.
    pub article: Article,
}

impl Expecting {
    pub const fn a(expect: &'static str) -> Self {
        Self {
            expect,
            article: Article::A,
        }
    }

    pub const fn an(expect: &'static str) -> Self {
        Self {
            expect,
            article: Article::An,
        }
    }
}

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
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType<'a> {
    // lex
    // ------
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

    // parse
    // ------
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
    /// A token was expected, but instead found EOF
    MissingToken {
        /// The token pattern expected
        expect: Expecting,
    },
    /// A token was expected, but instead found `actual`
    UnexpectedToken {
        /// The token pattern expected
        expect: Expecting,
        /// The token found
        actual: Token<'a>,
    },
}

impl std::fmt::Display for ErrorType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownToken => write!(f, "unknown token"),

            Self::EndlessBlockComment => {
                write!(
                    f,
                    "block comment opens (`/*`) but never closes (missing `*/`)"
                )
            }

            Self::EmptyCharLiteral => write!(f, "empty character literal"),

            Self::MultiCharLiteral => {
                write!(f, "character literal may only contain one codepoint")
            }

            Self::EndlessCharLiteral | Self::EscapedCharLiteralEnd => {
                write!(
                    f,
                    "char literal opens (`'`) but never closes (missing unescaped `'`)"
                )
            }

            Self::EndlessStringLiteral | Self::EscapedStringLiteralEnd => {
                write!(
                    f,
                    "string literal opens (`\"`) but never closes (missing unescaped `\"`)"
                )
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

            Self::MissingToken {
                expect: Expecting { expect, article },
            } => write!(f, "missing {article} {expect}"),

            Self::UnexpectedToken {
                expect: Expecting { expect, article },
                actual: Token { src: found, .. },
            } => {
                write!(f, "expected {article} {expect}, found `{found}`")
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

/// A code error with the range of the error in the source code
#[derive(Clone, PartialEq)]
pub struct ContextError<'a> {
    /// A string view of the FULL, ENTIRE source code
    pub source: &'a str,
    /// The range in [`Self::source`] of precisely where the error occurred
    pub range: Range<usize>,
    /// The exact error that was found
    pub err: ErrorType<'a>,
}

impl<'a> ContextError<'a> {
    pub fn unexpected(token: Token<'a>, source: &'a str, expected: Expecting) -> ContextError<'a> {
        ContextError {
            source,
            range: source
                .substr_range(token.src)
                .expect("token src should be a substring of the source code"),
            err: ErrorType::UnexpectedToken {
                expect: expected,
                actual: token,
            },
        }
    }

    pub const fn missing(source: &'a str, expected: Expecting) -> ContextError<'a> {
        ContextError {
            source,
            range: Range {
                start: source.len(),
                end: source.len(),
            },
            err: ErrorType::MissingToken { expect: expected },
        }
    }

    pub fn missing_or_unexpected(
        token: Option<Token<'a>>,
        source: &'a str,
        expected: Expecting,
    ) -> ContextError<'a> {
        match token {
            Some(token) => Self::unexpected(token, source, expected),
            None => Self::missing(source, expected),
        }
    }
}

impl std::fmt::Debug for ContextError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Invalid;
        f.debug_struct("ContextError")
            .field(
                "source[range]",
                // this closure looks pointless, but it's actually coercing `s` from `&&str` into `&dyn std::fmt::Debug`
                self.source.get(self.range).as_ref().map_or(&Invalid, |s| s),
            )
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

/// The line and column of `position` within `s`
pub fn line_col(s: &str, position: usize) -> Option<LineCol> {
    s.get(..position).map(|s| {
        s.split('\n') // assumes \n\r will never happen, except for \r\n\r\n
            .enumerate()
            .last()
            .map_or_default(|(row, line)| LineCol {
                line: row.strict_add(1), // +1 to convert from 0-based to 1-based
                col: line.len(),
            })
    })
}

/// The lines and columns of `start` and `end` within `s`
fn line_col_range(s: &str, range: Range<usize>) -> Option<Range<LineCol>> {
    line_col(s, range.start)
        .zip(line_col(s, range.end))
        .map(|(a, b)| (a..b).into())
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
        let Range { start, end } =
            line_col_range(self.source, self.range).expect("range should be a range within source");
        write!(f, "at {start}-{end}: {}", self.err)
    }
}

impl std::error::Error for ContextError<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}

/// [`std::fmt::Display`] the error code (number)
#[derive(Debug, Clone, PartialEq)]
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

            ErrorType::IncorrectCloseBracket { .. }
            | ErrorType::ExcessCloseBracket { .. }
            | ErrorType::MissingCloseBracket { .. }
            | ErrorType::MissingToken { .. }
            | ErrorType::UnexpectedToken { .. } => "GRA",
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
            ErrorType::MissingToken { .. } => 21,
            ErrorType::UnexpectedToken { .. } => 22,
        };
        write!(f, "err[{area}{code:>03}]")
    }
}

/// [`std::fmt::Display`] tips for resolving an error
#[derive(Debug, Clone)]
pub struct ContextErrorHelp<'a, 'b>(&'b ContextError<'a>);

impl std::fmt::Display for ContextErrorHelp<'_, '_> {
    #[expect(
        clippy::too_many_lines,
        reason = "it would be even more complicated to make a separate function for each of these"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let src = self
            .0
            .source
            .get(self.0.range)
            .expect("range should be a range in source");
        match &self.0.err {
            ErrorType::UnknownToken => write!(f, "try removing the character"),

            ErrorType::EndlessBlockComment => write!(f, "try adding `{BLOCK_COMMENT_CLOSE}`"),

            ErrorType::EmptyCharLiteral => write!(
                f,
                "chars can't be empty, try replacing `{CHAR_DELIM}{CHAR_DELIM}` with `{STR_DELIM}{STR_DELIM}` or insert a character",
            ),

            ErrorType::MultiCharLiteral => {
                let inner = src
                    .strip_circumfix(CHAR_DELIM, CHAR_DELIM)
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
                     or change this to a string ({STR_DELIM}{inner}{STR_DELIM})"
                )
            }

            ErrorType::EndlessCharLiteral => {
                write!(f, "try adding a `{CHAR_DELIM}` to the end of the char")
            }

            ErrorType::EscapedCharLiteralEnd => {
                let substr = src.strip_prefix(CHAR_DELIM)
                    .expect("string literal should include at least the open delimiter, in EscapedCharLiteralEnd")
                    .split_once("\\'")
                    .expect("should be EscapedCharLiteralEnd if this is not present")
                    .0;
                write!(
                    f,
                    "there is a closing single-quote candidate, but it is escaped (`{ESCAPE}{CHAR_DELIM}`). \n\
                     char literals cannot end with an unescaped backslash (`{ESCAPE}`), \
                     it is indistinguishable from an escaped single-quote (`{ESCAPE}{CHAR_DELIM}`). \n\
                     try adding a `{CHAR_DELIM}` to the end of the char or remove the `{ESCAPE}` from `{ESCAPE}{CHAR_DELIM}` \
                     to make the char `{CHAR_DELIM}{substr}{CHAR_DELIM}`"
                )
            }

            ErrorType::EndlessStringLiteral => {
                write!(f, "try adding a `{STR_DELIM}` to the end of the string")
            }

            ErrorType::EscapedStringLiteralEnd => {
                let substr = src
                    .strip_prefix(STR_DELIM)
                    .expect("string literal should include delimiter")
                    .split_once("\\\"")
                    .expect("should be EndlessStringLiteral if this is not present")
                    .0;
                write!(
                    f,
                    "there is a closing double-quote candidate, but it is escaped (`{ESCAPE}{STR_DELIM}`).\n\
                     string literals cannot end with an unescaped backslash (`{ESCAPE}`), \
                     it is indistinguishable from an escaped double-quote (`{ESCAPE}{STR_DELIM}`).\n\
                     try adding a `{STR_DELIM}` to the end of the string or remove the `{ESCAPE}` from `{ESCAPE}{STR_DELIM}` \
                     to make the string `{STR_DELIM}{substr}{STR_DELIM}`"
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
                    write!(
                        f,
                        "`\\a`, `\\b`, `\\e`, `\\f`, `\\n`, `\\r`, `\\t`, and `\\v` are the only supported \
                                ASCII letters that can be escape sequences"
                    )
                } else if ch.is_numeric() {
                    write!(
                        f,
                        "only ascii digits (0-9) are supported for decimal (base-10) numeric escape sequences"
                    )
                } else {
                    write!(
                        f,
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
                                        .split_once(|ch: char| !ch.is_ascii_hexdigit())
                                        .map(|(_, suffix)| (suffix, "hexadecimal"))
                                } else if let Some(digits) = src.strip_prefix(OCT_PREFIX) {
                                    digits
                                        .split_once(|ch: char| !ch.is_digit(8))
                                        .map(|(_, suffix)| (suffix, "octal"))
                                } else if let Some(digits) = src.strip_prefix(BIN_PREFIX) {
                                    digits
                                        .split_once(|ch: char| !ch.is_digit(2))
                                        .map(|(_, suffix)| (suffix, "binary"))
                                } else {
                                    src.split_once(|ch: char| !ch.is_ascii_digit())
                                        .map(|(_, suffix)| (suffix, "decimal"))
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
                                        .split_once(|ch: char| !ch.is_ascii_hexdigit())
                                        .map(|(_, suffix)| (suffix, "hexadecimal"))
                                } else if let Some(digits) = digits.strip_prefix(OCT_PREFIX) {
                                    digits
                                        .split_once(|ch: char| !ch.is_digit(8))
                                        .map(|(_, suffix)| (suffix, "octal"))
                                } else if let Some(digits) = digits.strip_prefix(BIN_PREFIX) {
                                    digits
                                        .split_once(|ch: char| !ch.is_digit(2))
                                        .map(|(_, suffix)| (suffix, "binary"))
                                } else {
                                    digits
                                        .split_once(|ch: char| !ch.is_ascii_digit())
                                        .map(|(_, suffix)| (suffix, "decimal"))
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

                    NumLitError::Flt(_) => write!(f, "I'm not sure how to help with this yet"),
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

            ErrorType::MissingToken {
                expect: Expecting { expect, article },
            } => write!(f, "try inserting {article} {expect}"),

            ErrorType::UnexpectedToken {
                expect: Expecting { expect, article },
                actual: Token { src: found, .. },
            } => {
                write!(
                    f,
                    "try inserting {article} {expect} before `{found}` or remove `{found}`"
                )
            }
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
#[derive(Debug, Clone)]
pub struct RenderedContextError<'a, 'b>(&'b ContextError<'a>);

/// Outputs a line reference to `f`.
///
/// Example:
/// ```not_code
///    |
///  1 |    let foo = 5;
///    |        ~~~ message
/// ```
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

    let (Range { start, end }, line_range) = line_col_range(source, range)
        .zip(line_containing(source, range))
        .expect("range should be a range in source");
    // bigger numbers have more digits so the last line number should have the most digits
    let num_width = end.line.to_string().len(); // ew, an allocation just to count the digits :c
    writeln!(f, "{PRE_NUM}{:>num_width$}{POST_NUM}", "")?;
    let num_lines = end
        .line
        .checked_sub(start.line)
        .expect("range should be ascending order");
    for (idx, line) in source
        .get(line_range)
        .expect("line_containing should return a valid range within the source string")
        .split('\n')
        .enumerate()
    {
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
            write!(f, " ")?;
        }
        write!(f, "{underline_style}")?;
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
        let mut has_prev = false;

        // error
        if !self.0.range.is_empty() {
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
                    ErrorType::ExcessCloseBracket { .. } => "missing a partner",
                    ErrorType::MissingCloseBracket { .. } => "missing close bracket",
                    ErrorType::MissingToken { .. } => "missing token",
                    ErrorType::UnexpectedToken { .. } => "wrong token",
                },
            )?;
            has_prev = true;
        }

        // info
        let items = match self.0.err {
            ErrorType::IncorrectCloseBracket {
                expect: (_, range), ..
            } => &[(range, "bracket type introduced here")],

            ErrorType::MissingCloseBracket { expect: (_, range) } => {
                &[(range, "missing a partner")]
            }

            _ => [].as_slice(),
        };
        for &(range, explanation) in items {
            if has_prev {
                writeln!(f)?;
            }
            line_ref(f, self.0.source, range, "\x1b[96m", '-', explanation)?;
            has_prev = true;
        }

        Ok(())
    }
}

/// A [`Token`] and its [`TokenValue`], or a [`ContextError`]
pub type TokenResult<'a> = Result<Token<'a>, ContextError<'a>>;
