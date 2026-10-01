//! Errors regarding code validity

use crate::{
    eval::ValueType,
    highlight::style::{Color, Style, StyleWrapper},
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

macro_rules! define_error_type {
    (
        formatter: $f:ident
        source: $src:ident

        $(#[$emeta:meta])*
        $vis:vis enum $Enum:ident<$lt:lifetime> {$(
            $(#[$vmeta:meta])*
            $Variant:ident$({$(
                $(#[$sfmeta:meta])*
                $sfield:ident: $SType:ty
            ),* $(,)?})?$(($(
                $TType:ty
            ),* $(,)?))?
            $kind:ident $code:literal {
                err$({$(..)? $( $err_s_ident:ident$(: $err_s_pat:pat)? ),* $(, ..)?})?$(($( $err_t_pat:pat ),*))? => $display:expr,
                inlay$({$(..)? $( $inlay_s_ident:ident$(: $inlay_s_pat:pat)? ),* $(, ..)?})?$(($( $inlay_t_pat:pat ),*))? => $inlay:expr,
                help$({$(..)? $( $help_s_ident:ident$(: $help_s_pat:pat)? ),* $(, ..)?})?$(($( $help_t_pat:pat ),*))? => $help:expr$(,
                info$({$(..)? $( $info_s_ident:ident$(: $info_s_pat:pat)? ),* $(, ..)?})?$(($( $info_t_pat:pat ),*))? => [$($info:expr),+$(,)?])?$(,)?
            }
        ),* $(,)?}
    ) => {
        $(#[$emeta])*
        $vis enum $Enum<$lt> {$(
            $(#[$vmeta])*
            $Variant$({$(
                $(#[$sfmeta])*
                $sfield: $SType
            ),*})?$(($(
                $TType
            ),*))?
        ),*}

        impl<'src> ContextError<'src> {
            fn info_line<'msg, A>(&'msg self, vec: &mut A)
            where
                'src: 'msg,
                A: Extend<LineRef<'msg>>
            {
                match &self.err {
                    $($($Enum::$Variant$({$( $info_s_ident$(: $info_s_pat)?, )* ..})?$(($( $info_t_pat ),*))? => {
                        vec.extend([$({
                            let (range, msg) = $info;
                            LineRef::new(self.source, RefStyleKind::Info, *range, Box::new(msg))
                        }),*])
                    },)?)*
                    _ => ()
                }
            }
        }

        impl std::fmt::Display for $Enum<'_> {
            fn fmt(&self, $f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {$(
                    $Enum::$Variant$({$( $err_s_ident$(: $err_s_pat)?, )* .. })?$(($( $err_t_pat ),*))? => $display
                ),*}
            }
        }

        impl std::fmt::Display for InlineErrMsg<'_, '_> {
            fn fmt(&self, $f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match &self.0 {$(
                    $Enum::$Variant$({$( $inlay_s_ident$(: $inlay_s_pat)?, )* ..})?$(($( $inlay_t_pat ),*))? => $inlay
                ),*}
            }
        }

        impl std::fmt::Display for ContextErrorHelp<'_, '_> {
            // #[expect(
            //     clippy::too_many_lines,
            //     reason = "it would be even more complicated to make a separate function for each of these"
            // )]
            fn fmt(&self, $f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let $src: &str = self
                    .0
                    .source
                    .get(self.0.range)
                    .expect("range should be a range in source");
                match &self.0.err {$(
                    $Enum::$Variant$({$( $help_s_ident$(: $help_s_pat)?, )* ..})?$(($( $help_t_pat ),*))? => $help
                ),*}
            }
        }

        impl std::fmt::Display for ContextErrorCode<'_, '_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let area = match &self.0.err {$(
                    $Enum::$Variant{..} => stringify!($kind)
                ),*};
                let code = match &self.0.err {$(
                    $Enum::$Variant{..} => $code
                ),*};
                write!(f, "err[{area}.{code:>03}]")
            }
        }
    };
}

define_error_type! {
    formatter: f
    source: src

    /// The kind of error describing a [`ContextError`]
    #[derive(Debug, Clone, PartialEq)]
    pub enum ErrorType<'src> {
        // ----------------------------
        // lex
        // ----------------------------
        /// Token type could not be identified from the initial character, and so is not a valid token
        UnknownToken LEX 1 {
            err => write!(f, "unknown token"),
            inlay => write!(f, "what is this?"),
            help => write!(f, "try removing the character"),
        },
        /// A block comment has no `*/` to end it
        EndlessBlockComment LEX 2 {
            err => write!(f, "block comment opens (`/*`) but never closes (missing `*/`)"),
            inlay => write!(f, "never ends"),
            help => write!(f, "try adding `{BLOCK_COMMENT_CLOSE}`"),
        },
        /// A character literal that is just `''`
        EmptyCharLiteral LEX 3 {
            err => write!(f, "empty character literal"),
            inlay => write!(f, "empty"),
            help => write!(f, "chars can't be empty, try replacing `{CHAR_DELIM}{CHAR_DELIM}` \
                               with `{STR_DELIM}{STR_DELIM}` or insert a character"),
        },
        /// A character literal with multiple codepoints
        MultiCharLiteral LEX 4 {
            err => write!(f, "character literal may only contain one codepoint"),
            inlay => write!(f, "a char should be 1 char"),
            help => {
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
                write!(f, "try removing the character(s) after `{first}` (remove trailing `{rest}`) \
                           or change this to a string ({STR_DELIM}{inner}{STR_DELIM})")
            },
        },
        /// A character literal has no `'` to end it
        EndlessCharLiteral LEX 5 {
            err => write!(f, "char literal opens (`'`) but never closes (missing unescaped `'`)"),
            inlay => write!(f, "never ends"),
            help => write!(f, "try adding a `{CHAR_DELIM}` to the end of the char"),
        },
        /// A character literal has no `'` to end it, but contains a `\'`
        EscapedCharLiteralEnd LEX 5 {
            err => write!(f, "char literal opens (`'`) but never closes (missing unescaped `'`)"),
            inlay => write!(f, "never ends, unless you remove the `\\`"),
            help => {
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
            },
        },
        /// A string literal has no `"` to end it
        EndlessStringLiteral LEX 6 {
            err => write!(f, "string literal opens (`\"`) but never closes (missing unescaped `\"`)"),
            inlay => write!(f, "never ends"),
            help => write!(f, "try adding a `{STR_DELIM}` to the end of the string"),
        },
        /// A string literal has no `"` to end it, but contains a `\"`
        EscapedStringLiteralEnd LEX 6 {
            err => write!(f, "string literal opens (`\"`) but never closes (missing unescaped `\"`)"),
            inlay => write!(f, "never ends, unless you remove the `\\`"),
            help => {
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
            },
        },
        /// A string/character literal contains an escape sequence (identified by a `\`) that does not exist
        InvalidEscape(&'src str) LEX 7 {
            err(esc) => write!(f, "unknown character escape: {esc:?}"),
            inlay(_) => write!(f, "has an invalid escape sequence"),
            help(esc) => {
                let mut iter = esc.chars();
                iter.next()
                    .filter(|ch| *ch == ESCAPE)
                    .expect("InvalidEscape should include `\\`");
                let ch = iter.next().expect("should have at least 2 characters or else be an EscapedStringLiteralEnd");

                if ch == 'x' {
                    let n = iter.take(2).filter(char::is_ascii_hexdigit).count();
                    assert!(n < 2, "why is this an error?");
                    write!(f, "`\\x` should be followed by 2 hexadecimal digits ([0-9a-fA-F]), this escape sequence has {n}")
                } else if ch == 'o' {
                    let n = iter.take(3).filter(|ch| ch.is_digit(8)).count();
                    assert!(n < 3, "why is this an error?");
                    write!(f, "`\\o` should be followed by 3 octal digits ([0-7]), this escape sequence has {n}")
                } else if ch.is_alphabetic() {
                    write!(
                        f,
                        "`\\a`, `\\b`, `\\e`, `\\f`, `\\n`, `\\r`, `\\t`, and `\\v` are the only supported \
                        ASCII letters that can be escape sequences"
                    )
                } else if ch.is_numeric() {
                    write!(f, "only ascii digits (0-9) are supported for decimal (base-10) numeric escape sequences")
                } else {
                    write!(
                        f,
                        "supported escape sequences: `\\a`, `\\b`, `\\e`, `\\f`, `\\n`, `\\r`, `\\t`, `\\v`, `\\0`-`\\9`,\n\\
                        `\\x##` (where # is a hexadecimal digit), `\\o###` (where # is an octal digit)"
                    )
                }
            }
        },
        /// A number literal could not be evaluated as a number
        InvalidNumLiteral(NumLitError) LEX 8 {
            err(e) => write!(f, "invalid number literal: {e}"),
            inlay(_) => write!(f, "not a valid number"),
            help(e) => {
                use std::num::IntErrorKind;
                match e {
                    NumLitError::UInt(e) => match e.kind() {
                        IntErrorKind::Empty => unreachable!("tokenizer should not emit number tokens that have no number"),

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
                            write!(f, "the suffix `{suffix}` is not valid for {base_name} integer literals")
                        }

                        IntErrorKind::PosOverflow => write!(f, "the largest supported unsigned integer value is {}", usize::MAX),

                        _ => unimplemented!(),
                    },

                    NumLitError::SInt(e) => match e.kind() {
                        IntErrorKind::Empty => unreachable!("tokenizer should not emit number tokens that have no number"),

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

                        IntErrorKind::PosOverflow => write!(f, "the largest supported signed integer value is {}", isize::MAX),

                        IntErrorKind::NegOverflow => write!(f, "the smallest supported signed integer value is {}", isize::MIN),

                        _ => unimplemented!(),
                    },

                    NumLitError::Flt(_) => write!(f, "I'm not sure how to help with this yet"),
                }
            }
        },

        // ----------------------------
        // preprocessor
        // ----------------------------
        /// A macro was encountered that has not been defined
        MacroUndefined PRE 21 {
            err => write!(f, "macro has not been defined at this point"),
            inlay => write!(f, "not defined"),
            help => write!(f, "try moving the definition ahead of this usage"),
        },

        // ----------------------------
        // parse
        // ----------------------------
        /// A closing bracket is of the wrong type for the open bracket at its depth
        IncorrectCloseBracket {
            /// The bracket type being expected based on the opening side
            expect: (Bracket, Range<usize>),
            /// The bracket type that was found
            actual: Bracket,
        } GRA 31 {
            err { expect: (expect, _), actual } => write!(f,
                "incorrect close bracket: expected `{}`, found `{}`",
                expect.close(),
                actual.close(),
            ),
            inlay { .. } => write!(f, "incorrect partner"),
            help { expect, actual } => write!(
                f,
                "try inserting a `{}` before the `{}`, add a `{}` before it and after the `{}`, \
                or remove either the `{}` or the `{}`",
                expect.0.close(),
                actual.close(),
                actual.open(),
                expect.0.open(),
                expect.0.open(),
                actual.close(),
            ),
            info { expect: (_, range) } => [(range, "bracket type introduced here")],
        },
        /// A closing bracket was found with no open bracket
        ExcessCloseBracket {
            /// The bracket type that was found
            actual: Bracket,
        } GRA 32 {
            err { actual } => write!(f, "too many close brackets: expected none, found `{}`", actual.close()),
            inlay { .. } => write!(f, "missing a partner"),
            help { actual } => write!(f, "try removing the `{}` or add a `{}` before it", actual.close(), actual.open()),
        },
        /// An open bracket was found with no close bracket
        MissingCloseBracket {
            /// The bracket type being expected based on the opening side
            expect: (Bracket, Range<usize>),
        } GRA 33 {
            err { expect: (expect, _) } => write!(f, "missing close bracket: expected `{}`, found none", expect.close()),
            inlay { .. } => write!(f, "missing close bracket"),
            help { expect } => write!(
                f,
                "try inserting a `{}` or remove the `{}`",
                expect.0.close(),
                expect.0.open(),
            ),
            info { expect: (_, range) } => [(range, "missing a partner")],
        },
        /// A token was expected, but instead found EOF
        MissingToken {
            /// The token pattern expected - should start with the proper article
            /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
            expect: &'static str,
        } GRA 34 {
            err { expect } => write!(f, "missing {expect}"),
            inlay { .. } => write!(f, "missing token"),
            help { expect } => write!(f, "try inserting {expect}"),
        },
        /// A token was expected, but instead found `actual`
        UnexpectedToken {
            /// The token pattern expected - should start with the proper article
            /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
            expect: &'static str,
            /// The token found
            actual: &'src str,
        } GRA 35 {
            err { expect, actual } => write!(f, "expected {expect}, found `{actual}`"),
            inlay { .. } => write!(f, "wrong token"),
            help { expect, actual } => write!(f, "try inserting {expect} before the `{actual}` or remove the `{actual}`"),
        },

        // ----------------------------
        // eval
        // ----------------------------
        /// Attempted to find the quotient or remainder with a denominator of 0
        DivByZero {
            /// The range of the expression evaluating to zero
            zero: Range<usize>,
        } RUN 41 {
            err { .. } => write!(f, "divide by zero"),
            inlay { .. } => write!(f, "dividing by 0"),
            help { .. } => write!(f, "ensure the right side cannot be 0"),
            info { zero } => [(zero, "this expression evaluates to 0")],
        },
        /// The operands in a binary operation are of incompatible type
        Incompatible {
            /// The binary operator
            op: Punctuation,
            /// The type of the value on the left side of the operator
            lhs: (ValueType, Range<usize>),
            /// The type of the value on the right side of the operator
            rhs: (ValueType, Range<usize>),
        } RUN 42 {
            err { op, lhs: (l_ty, _), rhs: (r_ty, _) } => write!(f, "{l_ty} is not compatible with {r_ty} for `{op}`"),
            inlay { op, .. } => write!(f, "{} is not supported for operands of these types", op_desc(*op, true)),
            help { .. } => write!(f, "try a different operator or convert the types"),
            info { lhs: (l_ty, l_range), rhs: (r_ty, r_range), .. } => [
                (l_range, TypeResolutionMsg { ty: *l_ty }),
                (r_range, TypeResolutionMsg { ty: *r_ty }),
            ],
        },
        /// The operand in a unary operation is of an unsupported type
        Unsupported {
            /// The unary operator
            op: Punctuation,
            /// The type of the value on the right side of the operator
            rhs: (ValueType, Range<usize>),
        } RUN 43 {
            err { op, rhs: (r_ty, _) } => write!(f, "`{op}` is not supported for {r_ty}"),
            inlay { op, .. } => write!(f, "{} is not supported for operands of this type", op_desc(*op, false)),
            help { .. } => write!(f, "try a different operator or convert the type"),
            info { rhs: (r_ty, r_range), .. } => [(r_range, TypeResolutionMsg { ty: *r_ty })],
        },
        /// Unsigned cannot be negated
        UnsignedNeg RUN 44 {
            err => write!(f, "unsigned integer cannot be negated"),
            inlay => write!(f, "uint can't be negated"),
            help => write!(f, "remove the `-` or convert the integer to signed"),
        },
        /// An operation resulted in overflow/underflow
        Overflow RUN 45 {
            err => write!(f, "arithmetic overflow"),
            inlay => write!(f, "unhandled integer overflow"),
            help => write!(f, "ensure the result will fit in an integer"),
        },
        /// Failed to convert between integer types
        FailedConvert(std::num::TryFromIntError) RUN 46 {
            err(e) => write!(f, "failed conversion: {e}"),
            inlay(_) => write!(f, "integer conversion failed"),
            help(_) => write!(f, "ensure the conversion will not result in overflow"),
        },
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

/// [`std::fmt::Display`] tips for resolving an error
#[derive(Debug, Clone)]
pub struct ContextErrorHelp<'src, 'err>(&'err ContextError<'src>);

/// Returns the range from the start of the first line in the range to the end of the last line in the range.
///
/// [`None`] if `range` is out of bounds for `src`
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct RefStyle {
    pub color: Style,
    pub underline: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum RefStyleKind {
    #[default]
    Info,
    Warning,
    Error,
}

impl RefStyleKind {
    const INFO_STYLE: RefStyle = RefStyle {
        color: Style::new().foreground(Color::BrightBlue),
        underline: '-',
    };
    const WARNING_STYLE: RefStyle = RefStyle {
        color: Style::new().foreground(Color::Yellow),
        underline: '~',
    };
    const ERROR_STYLE: RefStyle = RefStyle {
        color: Style::new().foreground(Color::BrightRed),
        underline: '^',
    };

    pub const fn style(self) -> RefStyle {
        match self {
            Self::Info => Self::INFO_STYLE,
            Self::Warning => Self::WARNING_STYLE,
            Self::Error => Self::ERROR_STYLE,
        }
    }
}

struct LineRef<'msg> {
    pub style: RefStyleKind,
    pub range: Range<usize>,
    pub block: Range<usize>,
    pub span: Range<LineCol>,
    pub msg: Box<dyn 'msg + std::fmt::Display>,
}

impl<'msg> LineRef<'msg> {
    fn new(
        source: &str,
        style: RefStyleKind,
        range: Range<usize>,
        msg: Box<dyn 'msg + std::fmt::Display>,
    ) -> Self {
        Self {
            style,
            range,
            block: line_containing(source, range).expect("range should be within source"),
            span: line_col_range(source, range).expect("range should be within source"),
            msg,
        }
    }
}

impl std::fmt::Debug for LineRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LineRef")
            .field("style", &self.style)
            .field("range", &self.range)
            .field_with("msg", |f| write!(f, "{}", self.msg))
            .finish()
    }
}

impl PartialEq for LineRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.style == other.style
            && self.range == other.range
            && std::ptr::eq(&self.msg, &other.msg)
    }
}
impl Eq for LineRef<'_> {}

/// User must ensure slice is in order of range
#[derive(Debug)]
struct LineRefs<'src, 'arr, 'msg> {
    pub source: &'src str,
    pub items: &'arr [LineRef<'msg>],
}

/// Outputs a line reference to `f`.
///
/// Example:
/// ```not_code
///    |
///  1 |    let foo = 5;
///    |        ^^^ message
/// ```
///
/// Multiple items in one line:
/// ```not_code
///    |
///  1 |    let foo = 5x;
///    |    ^^^ ^^^   ^^ message 3
///    |    |   |
///    |    |   message 2
///    |    |
///    |    message 1
/// ```
///
/// # Panics
/// This implementation may panic if refs overlap
impl std::fmt::Display for LineRefs<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const EDGE_STYLE: Style = Style::new().foreground(Color::BrightBlue);

        debug_assert!(
            self.items
                .is_sorted_by_key(|item| (item.range.start, item.range.end)),
            "LineRefs expects references to be sorted by range"
        );

        let Some(line_num_width) = self
            .items
            .iter()
            .map(|item| item.span.end.line)
            .max()
            .map(|n| n.to_string().len())
        else {
            // no items to display
            return Ok(());
        };

        macro_rules! write_line_start {
            ($f:expr, $n:expr) => {
                write!(
                    $f,
                    " {}{:>line_num_width$} |{}  ",
                    EDGE_STYLE.begin(),
                    $n,
                    EDGE_STYLE.end()
                )
            };

            ($f:expr) => {
                write_line_start!($f, "")
            };
        }

        for line_items in self.items.chunk_by(|a, b| {
            a.span.start.line == b.span.start.line
                && a.span.end.line == b.span.end.line
                // overlapping items not supported - they go in separate chunks
                && a.span.start.line == a.span.end.line
        }) {
            let first = line_items
                .first()
                .expect("chunk_by should not produce empty chunks");
            let start_line = first.span.start.line;
            let end_line = first.span.end.line;
            // the collection of source code lines containing the items
            let block: &str = self
                .source
                .get(first.block)
                .expect("block should be a range in source");
            let lines = block
                .lines()
                .enumerate()
                .map(|(n, line)| (n.strict_add(start_line), line));
            // balancing line
            write_line_start!(f)?;
            writeln!(f)?;
            // draw the underlines
            for (i, line) in lines {
                // first code line
                write_line_start!(f, i)?;
                writeln!(f, "{line}")?;
                // per-line
                write_line_start!(f)?;
                // assumes line items are in order
                let mut prev_end = 0;
                // assumes that if there are multiple lines, there is only one line_item
                for item in line_items {
                    let start_col = if i == start_line {
                        item.span.start.col
                    } else {
                        0
                    };
                    let end_col = if i == end_line {
                        item.span.end.col
                    } else {
                        line.len()
                    };
                    for _ in prev_end..start_col {
                        write!(f, " ")?;
                    }
                    let style = item.style.style();
                    style.color.begin().fmt(f)?;
                    for _ in start_col..end_col {
                        write!(f, "{}", style.underline)?;
                    }
                    style.color.end().fmt(f)?;
                    prev_end = end_col;
                }
                if i != end_line {
                    writeln!(f)?;
                }
            }
            // this iterator is over the item whose name will be displayed next.
            // it is in reverse, since the names are printed right to left.
            let mut rev_items = line_items
                .iter()
                .enumerate()
                .map(|(n, item)| (n.strict_add(1), item))
                .rev();
            // last one (on the same line as the underlines) is displayed immediately without pipes
            let (_, last) = rev_items
                .next()
                .expect("chunk_by should not produce empty chunks");
            writeln!(f, " {}", last.style.style().color.style(&last.msg))?;
            for (n, item) in rev_items {
                write_line_start!(f)?;
                // this loop is forward, because it prints the bar annotating the underline
                let mut prev_end = 0;
                for item in line_items.iter().take(n) {
                    for _ in prev_end..item.span.start.col {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item.style.style().color.style('|'))?;
                    prev_end = item.span.start.col.strict_add(1);
                }
                for msg_line in item.msg.to_string().lines() {
                    writeln!(f)?;
                    write_line_start!(f)?;
                    // second loop replaces the final bar with its message.
                    // there are two loops because we want a line of space between each message.
                    let mut prev_end = 0;
                    for (i, item) in line_items.iter().take(n).enumerate() {
                        for _ in prev_end..item.span.start.col {
                            write!(f, " ")?;
                        }
                        if i < n.saturating_sub(1) {
                            write!(f, "{}", item.style.style().color.style('|'))?;
                            prev_end = item.span.start.col.strict_add(1);
                        }
                    }
                    writeln!(f, "{}", item.style.style().color.style(msg_line))?;
                }
            }
        }
        writeln!(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct InlineErrMsg<'src, 'err>(&'err ErrorType<'src>);

fn op_desc(op: Punctuation, is_binary: bool) -> &'static str {
    match op {
        Punctuation::Not => "logical or bitwise 'not'",
        Punctuation::MacroStringify => "token stringification",
        Punctuation::Rem => "remainder",
        Punctuation::And => "logical or bitwise 'and'",
        Punctuation::Mul => "multiplication",
        Punctuation::Add => "addition",
        Punctuation::Sub if is_binary => "subtraction",
        Punctuation::Sub => "arithmetic negation",
        Punctuation::Div => "division",
        Punctuation::Ref => "referencing",
        Punctuation::Xor => "logical or bitwise 'xor'",
        Punctuation::Or => "logical or bitwise 'or'",
        Punctuation::Nand => "logical or bitwise 'nand'",
        Punctuation::Nor => "logical or bitwise 'nor'",
        Punctuation::Xnor => "logical or bitwise 'xnor'",
        Punctuation::MacroConcatenate => "token concatenation",
        Punctuation::Exp => "exponentiation",
        Punctuation::Shl => "left bitshift",
        Punctuation::Shr => "right bitshift",

        Punctuation::Lt
        | Punctuation::Gt
        | Punctuation::Neq
        | Punctuation::Le
        | Punctuation::Eq
        | Punctuation::Ge => "comparison",

        Punctuation::LParen
        | Punctuation::RParen
        | Punctuation::Comma
        | Punctuation::Dot
        | Punctuation::Colon
        | Punctuation::Semi
        | Punctuation::Assign
        | Punctuation::QMark // TODO: will this be an operation?
        | Punctuation::LBrack
        | Punctuation::RBrack
        | Punctuation::LBrace
        | Punctuation::RBrace
        | Punctuation::RemAssign
        | Punctuation::AndAssign
        | Punctuation::MulAssign
        | Punctuation::AddAssign
        | Punctuation::SubAssign
        | Punctuation::Arrow
        | Punctuation::DotDot // TODO: will this be an operation?
        | Punctuation::DivAssign
        | Punctuation::PathSep
        | Punctuation::ColonEq
        | Punctuation::FatArrow
        | Punctuation::XorAssign
        | Punctuation::OrAssign
        | Punctuation::ExpAssign
        | Punctuation::ShlAssign
        | Punctuation::ShrAssign
        | Punctuation::NandAssign
        | Punctuation::NorAssign
        | Punctuation::XnorAssign => {
            unimplemented!("not an operator")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TypeResolutionMsg {
    ty: ValueType,
}

impl std::fmt::Display for TypeResolutionMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type resolves to {}", self.ty)
    }
}

impl std::fmt::Display for RenderedContextError<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = InlineErrMsg(&self.0.err);
        LineRefs {
            source: self.0.source,
            items: {
                let mut refs = vec![LineRef::new(
                    self.0.source,
                    RefStyleKind::Error,
                    self.0.range,
                    Box::new(err),
                )];

                if let Some(macro_range) = self.0.macro_range {
                    refs.push(LineRef::new(
                        self.0.source,
                        RefStyleKind::Info,
                        macro_range,
                        Box::new("within this macro expansion"),
                    ));
                }

                self.0.info_line(&mut refs);

                refs.sort_by_key(|item| (item.span.start, item.span.end));
                refs
            }
            .as_slice(),
        }
        .fmt(f)
    }
}
