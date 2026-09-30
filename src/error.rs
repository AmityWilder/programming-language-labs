//! Errors regarding code validity

use crate::{
    eval::ValueType,
    scanner::{
        Bracket,
        symbols::{
            BIN_PREFIX, BLOCK_COMMENT_CLOSE, CHAR_DELIM, ESCAPE, HEX_PREFIX, OCT_PREFIX, STR_DELIM,
        },
        token::{Token, escape_char, punc::Punctuation},
    },
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

/// Remove an article ("a ", "an ", "a(n) ", "the ", or "") from the beginning of a string
#[expect(
    dead_code,
    reason = "reserved for if error expectations ever need to replace \"a\"/\"an\" with \"the\""
)]
fn trim_article(s: &str) -> &str {
    const ARTICLES: [&str; 4] = ["a ", "an ", "a(n) ", "the "];
    for article in ARTICLES {
        if let Some(trimmed) = s.strip_prefix(article) {
            return trimmed;
        }
    }
    s
}

/// The kind of error describing a [`ContextError`]
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType<'src> {
    // ----------------------------
    // lex
    // ----------------------------
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
    InvalidEscape(&'src str),
    /// A number literal could not be evaluated as a number
    InvalidNumLiteral(NumLitError),

    // ----------------------------
    // preprocessor
    // ----------------------------
    /// A macro was encountered that has not been defined
    MacroUndefined,

    // ----------------------------
    // parse
    // ----------------------------
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
        /// The token pattern expected - should start with the proper article
        /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
        expect: &'static str,
    },
    /// A token was expected, but instead found `actual`
    UnexpectedToken {
        /// The token pattern expected - should start with the proper article
        /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
        expect: &'static str,
        /// The token found
        actual: &'src str,
    },

    // ----------------------------
    // eval
    // ----------------------------
    /// Attempted to find the quotient or remainder with a denominator of 0
    DivByZero {
        /// The range of the expression evaluating to zero
        zero: Range<usize>,
    },
    /// The operands in a binary operation are of incompatible type
    Incompatible {
        /// The binary operator
        op: Punctuation,
        /// The type of the value on the left side of the operator
        lhs: ValueType,
        /// The type of the value on the right side of the operator
        rhs: ValueType,
    },
    /// The operand in a unary operation is of an unsupported type
    Unsupported {
        /// The unary operator
        op: Punctuation,
        /// The type of the value on the right side of the operator
        rhs: ValueType,
    },
    /// Unsigned cannot be negated
    UnsignedNeg,
    /// An operation resulted in overflow/underflow
    Overflow,
    /// Failed to convert between integer types
    FailedConvert(std::num::TryFromIntError),
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

            Self::MacroUndefined => write!(f, "macro has not been defined at this point"),

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

            Self::MissingToken { expect } => write!(f, "missing {expect}"),

            Self::UnexpectedToken { expect, actual } => {
                write!(f, "expected {expect}, found `{actual}`")
            }

            Self::DivByZero { .. } => write!(f, "divide by zero"),
            Self::Incompatible { op, lhs, rhs } => {
                write!(f, "{lhs} is not compatible with {rhs} for `{op}`")
            }
            Self::Unsupported { op, rhs } => write!(f, "`{op}` is not supported for {rhs}"),
            Self::UnsignedNeg => write!(f, "unsigned integer cannot be negated"),
            Self::Overflow => write!(f, "arithmetic overflow"),
            Self::FailedConvert(e) => write!(f, "failed conversion: {e}"),
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
pub struct ContextError<'src> {
    /// A string view of the FULL, ENTIRE source code
    pub source: &'src str,
    /// The range in [`Self::source`] of precisely where the error occurred
    pub range: Range<usize>,
    /// The range in [`Self::source`] of the macro call site that expanded to the erroneous code.
    /// [`None`] if the error did not occur in a macro expansion.
    pub macro_range: Option<Range<usize>>,
    /// The exact error that was found
    pub err: ErrorType<'src>,
}

impl<'src> ContextError<'src> {
    /// Produce an error with optional range
    pub const fn error(
        source: &'src str,
        range: Option<Range<usize>>,
        macro_range: Option<Range<usize>>,
        err: ErrorType<'src>,
    ) -> Self {
        Self {
            source,
            range: match range {
                Some(range) => range,
                None => Range {
                    start: source.len(),
                    end: source.len(),
                },
            },
            macro_range,
            err,
        }
    }

    /// Produce an error on an optional token (the range will be its lexeme)
    pub fn token_error(
        source: &'src str,
        token: Option<Token<'src>>,
        err: ErrorType<'src>,
    ) -> Self {
        Self::error(
            source,
            token.map(|token| token.lex_range(source)),
            token.and_then(|token| token.mac),
            err,
        )
    }

    /// Map the [`ErrorType`] of a [`ContextError`]
    pub fn map_type<F>(mut self, f: F) -> Self
    where
        F: FnOnce(ErrorType<'src>) -> ErrorType<'src>,
    {
        self.err = f(self.err);
        self
    }

    /// A token was found but not the right kind
    pub fn unexpected(token: Token<'src>, source: &'src str, expected: &'static str) -> Self {
        Self::token_error(
            source,
            Some(token),
            ErrorType::UnexpectedToken {
                expect: expected,
                actual: token.lex,
            },
        )
    }

    /// No token was found despite expecting one
    pub const fn missing(source: &'src str, expected: &'static str) -> Self {
        Self::error(
            source,
            None,
            None, // TODO: is there a case where macro expansion can have a missing token?
            ErrorType::MissingToken { expect: expected },
        )
    }

    /// A token is expected but wasn't found; determine from its existence if it's unexpected or missing
    pub fn missing_or_unexpected(
        token: Option<Token<'src>>,
        source: &'src str,
        expected: &'static str,
    ) -> Self {
        match token {
            Some(token) => Self::unexpected(token, source, expected),
            None => Self::missing(source, expected),
        }
    }
}

impl std::fmt::Debug for ContextError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            source,
            range,
            macro_range,
            err,
        } = self;
        let src = source
            .get(*range)
            .expect("range should be a range in source");
        write!(f, "ContextError({src:?}")?;
        if let Some(macro_range) = macro_range {
            let src = source
                .get(*macro_range)
                .expect("macro_range should be a range in source");
            write!(f, " in expansion of {src:?}")?;
        }
        writeln!(f, "): {err:?}")
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
pub fn line_col_range(s: &str, range: Range<usize>) -> Option<Range<LineCol>> {
    line_col(s, range.start)
        .zip(line_col(s, range.end))
        .map(|(a, b)| (a..b).into())
}

impl<'src> ContextError<'src> {
    /// Returns a struct that implements [`std::fmt::Display`] to show detailed line reference information
    #[must_use]
    pub const fn render(&self) -> RenderedContextError<'_, 'src> {
        RenderedContextError(self)
    }

    /// Returns a struct that implements [`std::fmt::Display`] to show the error code (number)
    #[must_use]
    pub const fn code(&self) -> ContextErrorCode<'_, 'src> {
        ContextErrorCode(self)
    }

    /// Returns a struct that implements [`std::fmt::Display`] to show tips for resolving the error
    #[must_use]
    pub const fn help(&self) -> ContextErrorHelp<'_, 'src> {
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
pub struct ContextErrorCode<'src, 'err>(&'err ContextError<'src>);

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

            ErrorType::MacroUndefined => "PRE",

            ErrorType::IncorrectCloseBracket { .. }
            | ErrorType::ExcessCloseBracket { .. }
            | ErrorType::MissingCloseBracket { .. }
            | ErrorType::MissingToken { .. }
            | ErrorType::UnexpectedToken { .. } => "GRA",

            ErrorType::DivByZero { .. }
            | ErrorType::Incompatible { .. }
            | ErrorType::Unsupported { .. }
            | ErrorType::UnsignedNeg
            | ErrorType::Overflow
            | ErrorType::FailedConvert(_) => "RUN",
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

            ErrorType::MacroUndefined => 11,

            ErrorType::IncorrectCloseBracket { .. } => 21,
            ErrorType::ExcessCloseBracket { .. } => 22,
            ErrorType::MissingCloseBracket { .. } => 23,
            ErrorType::MissingToken { .. } => 24,
            ErrorType::UnexpectedToken { .. } => 25,

            ErrorType::DivByZero { .. } => 31,
            ErrorType::Incompatible { .. } => 32,
            ErrorType::Unsupported { .. } => 33,
            ErrorType::UnsignedNeg => 34,
            ErrorType::Overflow => 35,
            ErrorType::FailedConvert(_) => 36,
        };
        write!(f, "err[{area}{code:>03}]")
    }
}

/// [`std::fmt::Display`] tips for resolving an error
#[derive(Debug, Clone)]
pub struct ContextErrorHelp<'src, 'err>(&'err ContextError<'src>);

impl std::fmt::Display for ContextErrorHelp<'_, '_> {
    #[expect(
        clippy::too_many_lines,
        reason = "it would be even more complicated to make a separate function for each of these"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let src: &str = self
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
                            // TODO: dry this up
                            let (suffix, base_name) =
                                if let Some(digits) = src.strip_prefix(HEX_PREFIX) {
                                    let pos = digits
                                        .find(|ch: char| !ch.is_ascii_hexdigit())
                                        .expect("should contain an invalid digit");
                                    let suffix = digits
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "hexadecimal")
                                } else if let Some(digits) = src.strip_prefix(OCT_PREFIX) {
                                    digits
                                        .find(|ch: char| !ch.is_digit(8))
                                        .expect("should contain an invalid digit");
                                    let pos = digits
                                        .find(|ch: char| !ch.is_digit(8))
                                        .expect("should contain an invalid digit");
                                    let suffix = digits
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "octal")
                                } else if let Some(digits) = src.strip_prefix(BIN_PREFIX) {
                                    let pos = digits
                                        .find(|ch: char| !ch.is_digit(2))
                                        .expect("should contain an invalid digit");
                                    let suffix = digits
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "binary")
                                } else {
                                    let pos = src
                                        .find(|ch: char| !ch.is_ascii_digit())
                                        .expect("should contain an invalid digit");
                                    let suffix = src
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "decimal")
                                };
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
                                    let pos = digits
                                        .find(|ch: char| !ch.is_ascii_hexdigit())
                                        .expect("should contain an invalid digit");
                                    let suffix = digits
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "hexadecimal")
                                } else if let Some(digits) = digits.strip_prefix(OCT_PREFIX) {
                                    let pos = digits
                                        .find(|ch: char| !ch.is_digit(8))
                                        .expect("should contain an invalid digit");
                                    let suffix = digits
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "octal")
                                } else if let Some(digits) = digits.strip_prefix(BIN_PREFIX) {
                                    let pos = digits
                                        .find(|ch: char| !ch.is_digit(2))
                                        .expect("should contain an invalid digit");
                                    let suffix = digits
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "binary")
                                } else {
                                    let pos = digits
                                        .find(|ch: char| !ch.is_ascii_digit())
                                        .expect("should contain an invalid digit");
                                    let suffix = digits
                                        .get(pos..)
                                        .expect("find should not be within a UTF-8 character");
                                    (suffix, "decimal")
                                };
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

            ErrorType::MacroUndefined => write!(f, "try moving the definition ahead of this usage"),

            ErrorType::IncorrectCloseBracket { expect, actual } => write!(
                f,
                "try inserting a `{}` before the `{}`, add a `{}` before it and after the `{}`, or remove either the `{}` or the `{}`",
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

            ErrorType::MissingToken { expect } => write!(f, "try inserting {expect}"),

            ErrorType::UnexpectedToken { expect, actual } => {
                write!(
                    f,
                    "try inserting {expect} before the `{actual}` or remove the `{actual}`"
                )
            }

            ErrorType::DivByZero { .. } => write!(f, "ensure the right side cannot be 0"),
            ErrorType::Incompatible { .. } => {
                write!(f, "try a different operator or convert the types")
            }
            ErrorType::Unsupported { .. } => {
                write!(f, "try a different operator or convert the type")
            }
            ErrorType::UnsignedNeg => write!(f, "remove the `-` or convert the integer to signed"),
            ErrorType::Overflow => write!(f, "ensure the result will fit in an integer"),
            ErrorType::FailedConvert(_) => {
                write!(f, "ensure the conversion will not result in overflow")
            }
        }
    }
}

/// Returns [`None`] if `range` is out of bounds for `src`
#[must_use]
pub fn line_containing(src: &str, range: Range<usize>) -> Option<Range<usize>> {
    let line_start = src.get(..range.start)?.rfind('\n').map_or(0, |pos| {
        // SAFETY: `pos` is the position of the start of a 1-byte ASCII char ('\n'), therefore we can
        // add the length of that char (1 byte) to get the end, which is at most src.len().
        unsafe { pos.unchecked_add('\n'.len_utf8()) }
    });
    let line_end = src.get(range.end..)?.find('\n').map_or(src.len(), |n| {
        // SAFETY: `n` is a position in `source[range.end..]`, therefore `range.end + n`
        // is a position in `source[..]`, which must be in memory whose len therefore fits in usize.
        unsafe { n.unchecked_add(range.end) }
    });
    Some((line_start..line_end).into())
}

/// [`std::fmt::Display`] advanced error information with line references for a context error
#[derive(Debug, Clone)]
pub struct RenderedContextError<'src, 'err>(&'err ContextError<'src>);

/// Dynamic write function
// TODO: surely this can be done more cheaply?
type DynDisplay = Box<dyn FnOnce(&mut std::fmt::Formatter<'_>) -> std::fmt::Result>;

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
    msg: DynDisplay,
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
    }
    f.write_str(" ")?;
    msg(f)?;
    writeln!(f, "\x1b[0m")?;
    Ok(())
}

impl std::fmt::Display for RenderedContextError<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut has_prev = false;

        // info
        if let Some(macro_range) = self.0.macro_range {
            if has_prev {
                writeln!(f)?;
            }
            line_ref(
                f,
                self.0.source,
                macro_range,
                "\x1b[96m",
                '-',
                Box::new(|f| f.write_str("within this macro expansion")),
            )?;
            has_prev = true;
        }

        // error
        if !self.0.range.is_empty() {
            if has_prev {
                writeln!(f)?;
            }
            line_ref(
                f,
                self.0.source,
                self.0.range,
                "\x1b[91m",
                '^',
                match self.0.err {
                    ErrorType::UnknownToken => Box::new(|f| f.write_str("what is this?")),
                    ErrorType::EndlessBlockComment
                    | ErrorType::EndlessCharLiteral
                    | ErrorType::EndlessStringLiteral => Box::new(|f| f.write_str("never ends")),
                    ErrorType::EmptyCharLiteral => Box::new(|f| f.write_str("empty")),
                    ErrorType::MultiCharLiteral => {
                        Box::new(|f| f.write_str("a char should be 1 char"))
                    }
                    ErrorType::EscapedCharLiteralEnd | ErrorType::EscapedStringLiteralEnd => {
                        Box::new(|f| f.write_str("never ends, unless you remove the `\\`"))
                    }
                    ErrorType::InvalidEscape(_) => {
                        Box::new(|f| f.write_str("has an invalid escape sequence"))
                    }
                    ErrorType::InvalidNumLiteral(_) => {
                        Box::new(|f| f.write_str("not a valid number"))
                    }
                    ErrorType::MacroUndefined => Box::new(|f| f.write_str("not defined")),
                    ErrorType::IncorrectCloseBracket { .. } => {
                        Box::new(|f| f.write_str("incorrect partner"))
                    }
                    ErrorType::ExcessCloseBracket { .. } => {
                        Box::new(|f| f.write_str("missing a partner"))
                    }
                    ErrorType::MissingCloseBracket { .. } => {
                        Box::new(|f| f.write_str("missing close bracket"))
                    }
                    ErrorType::MissingToken { .. } => Box::new(|f| f.write_str("missing token")),
                    ErrorType::UnexpectedToken { .. } => Box::new(|f| f.write_str("wrong token")),
                    ErrorType::DivByZero { .. } => Box::new(|f| f.write_str("dividing by 0")),
                    ErrorType::Incompatible { op, .. } => {
                        Box::new(move |f| write!(f, "operands do not support {op}"))
                    }
                    ErrorType::Unsupported { op, .. } => {
                        Box::new(move |f| write!(f, "operand does not support {op}"))
                    }
                    ErrorType::UnsignedNeg => Box::new(|f| f.write_str("uint can't be negated")),
                    ErrorType::Overflow => Box::new(|f| f.write_str("unhandled integer overflow")),
                    ErrorType::FailedConvert(_) => {
                        Box::new(|f| f.write_str("integer conversion failed"))
                    }
                },
            )?;
            has_prev = true;
        }

        // info
        let items: Vec<(Range<usize>, DynDisplay)> = match self.0.err {
            ErrorType::IncorrectCloseBracket {
                expect: (_, range), ..
            } => vec![(
                range,
                Box::new(|f| f.write_str("bracket type introduced here")),
            )],

            ErrorType::MissingCloseBracket { expect: (_, range) } => {
                vec![(range, Box::new(|f| f.write_str("missing a partner")))]
            }

            ErrorType::DivByZero { zero } => {
                vec![(
                    zero,
                    Box::new(|f| f.write_str("this expression evaluates to 0")),
                )]
            }

            _ => Vec::new(),
        };
        for (range, explanation) in items {
            if has_prev {
                writeln!(f)?;
            }
            line_ref(f, self.0.source, range, "\x1b[96m", '-', explanation)?;
            has_prev = true;
        }

        Ok(())
    }
}
