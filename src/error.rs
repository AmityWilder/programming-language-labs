//! Errors regarding code validity

use crate::{
    eval::ValueType,
    highlight::style::{Color, Style, StyleWrapper},
    scanner::{
        BadBracketCombo, Bracket,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntValue {
    UInt(usize),
    SInt(isize),
}

impl Default for IntValue {
    fn default() -> Self {
        Self::UInt(Default::default())
    }
}

impl std::fmt::Display for IntValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UInt(x) => x.fmt(f),
            Self::SInt(x) => x.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct OverflowErrorInfoMsg {
    value: IntValue,
}

impl std::fmt::Display for OverflowErrorInfoMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { value } = self;
        write!(f, "this expression evaluated to {value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowError {
    Binary {
        l_range: Range<usize>,
        r_range: Range<usize>,
        l_value: IntValue,
        r_value: IntValue,
    },
    Unary {
        r_range: Range<usize>,
        r_value: IntValue,
    },
}

impl IntoIterator for OverflowError {
    type Item = (Range<usize>, OverflowErrorInfoMsg);
    type IntoIter = std::array::IntoIter<(Range<usize>, OverflowErrorInfoMsg), 2>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            OverflowError::Binary {
                l_range,
                r_range,
                l_value,
                r_value,
            } => [
                (l_range, OverflowErrorInfoMsg { value: l_value }),
                (r_range, OverflowErrorInfoMsg { value: r_value }),
            ]
            .into_iter(),
            OverflowError::Unary { r_range, r_value } => {
                let mut iter = [
                    Default::default(),
                    (r_range, OverflowErrorInfoMsg { value: r_value }),
                ]
                .into_iter();
                _ = iter.next(); // skip first
                iter
            }
        }
    }
}

macro_rules! expected_token {
    (
        $(#[$meta:meta])*
        $vis:vis enum $Enum:ident {$(
            $Variant:ident = $desc:expr
        ),* $(,)?}
    ) => {
        $(#[$meta])*
        $vis enum $Enum {$(
            $Variant
        ),*}

        impl $Enum {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $desc),*
                }
            }
        }
    };
}

expected_token! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ExpectedToken {
        // preproc
        MacroIdent = "a macro identifier",
        LParen = "a `(`",
        CommaOrRParen = "a `,` or `)`",
        MacroParamOrRParen = "a macro parameter or `)`",
        MacroDefLBrace = "a `{` for macro definition",
        MacroArgLBrace = "a `{` for macro argument",

        // grammar
        Literal = "a literal",
        ParenExpr = "a parenthesized expression",
        ExprOrRParen = "an expression or `)`",
        Expr = "an expression",
    }
}

impl std::fmt::Display for ExpectedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
                info$({$(..)? $( $info_s_ident:ident$(: $info_s_pat:pat)? ),* $(, ..)?})?$(($( $info_t_pat:pat ),*))? => $info:expr)?$(,)?
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
                        vec.extend($info.into_iter().map(|(range, msg)| LineRef::new(self.source, RefStyleKind::Info, range, Box::new(msg))))
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
            err (esc) => write!(f, "unknown character escape: {esc:?}"),
            inlay (_) => write!(f, "has an invalid escape sequence"),
            help (esc) => {
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
            err (e) => write!(f, "invalid number literal: {e}"),
            inlay (_) => write!(f, "not a valid number"),
            help (e) => {
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
            open_range: Range<usize>,
            failure: BadBracketCombo,
        } GRA 31 {
            err { failure } => {
                let (expect, actual) = failure.decompose();
                write!(f,
                    "incorrect close bracket: expected `{}`, found `{}`",
                    expect.close(),
                    actual.close(),
                )
            },
            inlay { .. } => write!(f, "incorrect partner"),
            help { failure } => {
                let (expect, actual) = failure.decompose();
                write!(
                    f,
                    "try inserting a `{}` before the `{}`, add a `{}` before it and after the `{}`, \
                    or remove either the `{}` or the `{}`",
                    expect.close(),
                    actual.close(),
                    actual.open(),
                    expect.open(),
                    expect.open(),
                    actual.close(),
                )
            },
            info { open_range } => [(*open_range, "bracket type introduced here")],
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
            open_range: Range<usize>,
            expect: Bracket,
        } GRA 33 {
            err { expect } => write!(f, "missing close bracket: expected `{}`, found none", expect.close()),
            inlay { .. } => write!(f, "missing close bracket"),
            help { expect } => write!(
                f,
                "try inserting a `{}` or remove the `{}`",
                expect.close(),
                expect.open(),
            ),
            info { open_range } => [(*open_range, "missing a partner")],
        },
        /// A token was expected, but instead found EOF
        MissingToken {
            /// The token pattern expected - should start with the proper article
            /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
            expect: ExpectedToken,
        } GRA 34 {
            err { expect } => write!(f, "missing {expect}"),
            inlay { .. } => write!(f, "missing token"),
            help { expect } => write!(f, "try inserting {expect}"),
        },
        /// A token was expected, but instead found `actual`
        UnexpectedToken {
            /// The token pattern expected - should start with the proper article
            /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
            expect: ExpectedToken,
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
            info { zero } => [(*zero, "this expression evaluates to 0")],
        },
        /// The operands in a binary operation are of incompatible type
        Incompatible {
            /// The binary operator
            op: Punctuation,
            l_range: Range<usize>,
            r_range: Range<usize>,
            l_ty: ValueType,
            r_ty: ValueType,
        } RUN 42 {
            err { op, l_ty, r_ty, .. } => write!(f, "{l_ty} is not compatible with {r_ty} for `{op}`"),
            inlay { op, l_ty, r_ty, .. } => write!(f, "{} is not supported between operands of these types", op.op_description(&[*l_ty, *r_ty])),
            help { op, l_ty, r_ty, .. } => {
                write!(f, "try changing operators, converting types, or wrapping something in parentheses")?;
                match op {
                    Punctuation::And |
                    Punctuation::Xor |
                    Punctuation::Or |
                    Punctuation::Nand |
                    Punctuation::Nor |
                    Punctuation::Xnor if matches!(l_ty, ValueType::Bool) != matches!(r_ty, ValueType::Bool)
                        // TODO: the user might have used a boolean literal/variable, not a comparison; this message might not be helpful in that case
                        => write!(f, "\n\nit looks like you may have been trying to perform a bitwise operation, but one side resolves to a boolean.\n\
                            bitwise/logic operations have lower precedence than comparisons, so you may need to wrap the bitwise operation in parentheses."),

                    _ => Ok(())
                }
            },
            info { l_ty, l_range, r_ty, r_range, .. } => [
                (*l_range, TypeResolutionMsg { side: OpSide::Left, ty: *l_ty }),
                (*r_range, TypeResolutionMsg { side: OpSide::Right, ty: *r_ty }),
            ],
        },
        /// The operand in a unary operation is of an unsupported type
        Unsupported {
            /// The unary operator
            op: Punctuation,
            /// The type of the value on the right side of the operator
            r_ty: ValueType,
            r_range: Range<usize>,
        } RUN 43 {
            err { op, r_ty, .. } => write!(f, "`{op}` is not supported for {r_ty}"),
            inlay { op, r_ty, .. } => write!(f, "{} is not supported for operands of this type", op.op_description(&[*r_ty])),
            help { .. } => write!(f, "try a different operator or convert the type"),
            info { r_ty, r_range, .. } => [(*r_range, TypeResolutionMsg { side: OpSide::Right, ty: *r_ty })],
        },
        /// Unsigned cannot be negated
        UnsignedNeg RUN 44 {
            err => write!(f, "unsigned integer cannot be negated"),
            inlay => write!(f, "uint can't be negated"),
            help => write!(f, "remove the `-` or convert the integer to signed"),
        },
        /// An operation resulted in overflow
        Overflow(OverflowError) RUN 45 {
            err (_) => write!(f, "arithmetic overflow"),
            inlay (_) => write!(f, "unhandled integer overflow"),
            help (_) => write!(f, "ensure the result will fit in an integer"),
            info (e) => *e,
        },
        /// Failed to convert between integer types
        FailedConvert {
            op: Punctuation,
            is_binary: bool,
            op_range: Range<usize>,
            failure: IntConversionFailure,
        } RUN 46 {
            err { failure, .. } => write!(f, "failed conversion: {failure}"),
            inlay { .. } => write!(f, "integer conversion failed"),
            help { .. } => write!(f, "ensure the expression fits in the target type"),
            info { op, op_range, failure, is_binary } => [
                (*op_range, FailedConversionMsg::new(*op, OpSide::Right, *failure, *is_binary)),
            ],
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
    pub fn unexpected(token: Token<'src>, source: &'src str, expect: ExpectedToken) -> Self {
        Self::token_error(
            source,
            Some(token),
            ErrorType::UnexpectedToken {
                expect,
                actual: token.lex,
            },
        )
    }

    /// No token was found despite expecting one
    pub const fn missing(source: &'src str, expect: ExpectedToken) -> Self {
        Self::error(
            source,
            None,
            None, // TODO: is there a case where macro expansion can have a missing token?
            ErrorType::MissingToken { expect },
        )
    }

    /// A token is expected but wasn't found; determine from its existence if it's unexpected or missing
    pub fn missing_or_unexpected(
        token: Option<Token<'src>>,
        source: &'src str,
        expect: ExpectedToken,
    ) -> Self {
        match token {
            Some(token) => Self::unexpected(token, source, expect),
            None => Self::missing(source, expect),
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

pub fn lines_in(s: &str) -> usize {
    s.matches('\n').count().strict_add(1) // +1 to convert from 0-based to 1-based
}

pub fn last_line_cols(s: &str) -> usize {
    s.rsplit_once('\n').map_or(s, |(_, tail)| tail).len()
}

/// The line of `position` within `s`
#[expect(
    dead_code,
    reason = "optimized alternative to line_col for when only line part is needed"
)]
pub fn line_of(s: &str, position: usize) -> Option<usize> {
    s.get(..position).map(last_line_cols)
}

/// The column of `position` within `s`
#[expect(
    dead_code,
    reason = "optimized alternative to line_col for when only col part is needed"
)]
pub fn col_of(s: &str, position: usize) -> Option<usize> {
    s.get(..position).map(last_line_cols)
}

/// The line and column of `position` within `s`
///
/// It is slightly cheaper to call this function if you are doing both, since they use the same substring
pub fn line_col(s: &str, position: usize) -> Option<LineCol> {
    s.get(..position).map(|s: &str| LineCol {
        line: lines_in(s),
        col: last_line_cols(s),
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
    #[expect(dead_code, reason = "reserved for use in static analysis")]
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

impl std::fmt::Debug for LineRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LineRef")
            .field("style", &self.style)
            .field("range", &self.range)
            .field("block", &self.block)
            .field("span", &self.span)
            .field_with("msg", |f| write!(f, "{:?}", self.msg.to_string()))
            .finish()
    }
}

impl PartialEq for LineRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.style == other.style
            && self.range == other.range
            && std::ptr::eq(&raw const self.msg, &raw const other.msg)
    }
}
impl Eq for LineRef<'_> {}

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

/// User must ensure slice is in order of range
#[derive(Debug)]
struct LineRefs<'src, 'arr, 'msg> {
    pub source: &'src str,
    pub items: &'arr [LineRef<'msg>],
}

impl<'src, 'arr, 'msg> LineRefs<'src, 'arr, 'msg> {
    const EDGE_STYLE: Style = Style::new().foreground(Color::BrightBlue);
    /// Displayed in place of the line numbers between non-contiguous lines
    const ELLIPSES: &str = "...";
    /// Currently, no inline error message has multiple lines.
    /// If one does in the future, enable this.
    /// Having this disabled saves from allocating a string to iterate over its lines,
    /// but messes up rendering if an error message has multiple lines.
    const SUPPORT_MULTILINE_MSG: bool = true;

    const fn new(source: &'src str, items: &'arr [LineRef<'msg>]) -> Self {
        Self { source, items }
    }

    fn write_line_start<T>(
        f: &mut std::fmt::Formatter<'_>,
        line_num_width: usize,
        line_number: T,
    ) -> std::fmt::Result
    where
        T: std::fmt::Display,
    {
        write!(
            f,
            " {}{:>line_num_width$} |{}  ",
            LineRefs::EDGE_STYLE.begin(),
            line_number,
            LineRefs::EDGE_STYLE.end(),
        )
    }

    fn line_num_width(&self) -> Option<usize> {
        self.items
            .iter()
            .map(|item| item.span.end.line)
            .max()
            // this map is being applied to an Option, not an iterator,
            // so only the biggest number calls `to_string()`
            .map(|n| n.to_string().len().max(Self::ELLIPSES.len()))
    }

    fn span_chunks(
        &self,
    ) -> std::slice::ChunkBy<'arr, LineRef<'msg>, fn(&LineRef<'_>, &LineRef<'_>) -> bool> {
        const fn p(a: &LineRef<'_>, b: &LineRef<'_>) -> bool {
            a.span.start.line == b.span.start.line
                && a.span.end.line == b.span.end.line
                // overlapping items not supported - they go in separate chunks
                && a.span.start.line == a.span.end.line
        }
        self.items.chunk_by(p)
    }

    fn underlines(
        f: &mut std::fmt::Formatter<'_>,
        start_line: usize,
        end_line: usize,
        line_num_width: usize,
        block: &str,
        line_items: &[LineRef<'_>],
    ) -> std::fmt::Result {
        // will only have multiple lines if there are multiple lines in a single line_item
        let lines = block
            .lines()
            .enumerate()
            .map(|(n, line)| (n.strict_add(start_line), line));
        for (i, line) in lines {
            // print the line content
            Self::write_line_start(f, line_num_width, i)?;
            writeln!(f, "{line}")?;
            // per-line
            Self::write_line_start(f, line_num_width, "")?;
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
                std::fmt::Display::fmt(&style.color.begin(), f)?;
                for _ in start_col..end_col {
                    write!(f, "{}", style.underline)?;
                }
                std::fmt::Display::fmt(&style.color.end(), f)?;
                prev_end = end_col;
            }
            if i != end_line {
                writeln!(f)?;
            }
        }
        Ok(())
    }

    /// Enables [`Self::SUPPORT_MULTILINE_MSG`] to be either enabled or disabled.
    fn print_message<T>(
        f: &mut std::fmt::Formatter<'_>,
        line_num_width: usize,
        n: usize,
        line_items: &[LineRef<'_>],
        item: &LineRef<'_>,
        msg_line: &T,
    ) -> std::fmt::Result
    where
        T: ?Sized + std::fmt::Display,
    {
        writeln!(f)?;
        Self::write_line_start(f, line_num_width, "")?;
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
        write!(f, "{}", item.style.style().color.style(msg_line))
    }

    fn inline_messages(
        f: &mut std::fmt::Formatter<'_>,
        line_items: &[LineRef<'_>],
        line_num_width: usize,
    ) -> std::fmt::Result {
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
            Self::write_line_start(f, line_num_width, "")?;
            // this loop is forward, because it prints the bar annotating the underline
            let mut prev_end = 0;
            for item in line_items.iter().take(n) {
                for _ in prev_end..item.span.start.col {
                    write!(f, " ")?;
                }
                write!(f, "{}", item.style.style().color.style('|'))?;
                prev_end = item.span.start.col.strict_add(1);
            }

            if Self::SUPPORT_MULTILINE_MSG {
                for msg_line in item
                    .msg
                    .to_string() // <- what we are avoiding by disabling SUPPORT_MULTILINE_MSG
                    .lines()
                {
                    Self::print_message(f, line_num_width, n, line_items, item, msg_line)?;
                }
            } else {
                debug_assert!(
                    !item.msg.to_string().contains('\n'),
                    "multiline inline messages not supported, implementor promised no inline error message would be multiple lines"
                );
                Self::print_message(f, line_num_width, n, line_items, item, &*item.msg)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Renders a line reference with one or more messages.
///
/// Example:
/// ```not_code
///    |
///  1 |  let foo = 5;
///    |      ^^^ message
/// ```
///
/// Multiple items in one line:
/// ```not_code
///    |
///  1 |  let foo = 5x;
///    |  ^^^ ^^^   ^^ message 3
///    |  |   |
///    |  |   message 2
///    |  |
///    |  message 1
/// ```
impl std::fmt::Display for LineRefs<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        debug_assert!(
            self.items
                .is_sorted_by_key(|item| (item.range.start, item.range.end)),
            "LineRefs expects references to be sorted by range"
        );

        let Some((very_first, line_num_width)) = self.line_num_width().map(|n| {
            (
                self.items
                    .first()
                    .expect("guaranteed at least one element if a max element exists"),
                n,
            )
        }) else {
            // no items to display
            return Ok(());
        };

        let mut prev_line = very_first
            .span
            .start
            .line
            .checked_sub(1)
            .expect("line_col promises the line number will never be below 1");

        for line_items in self.span_chunks() {
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
            // separator line
            Self::write_line_start(f, line_num_width, {
                if std::mem::replace(&mut prev_line, start_line)
                    == start_line
                        .checked_sub(1)
                        .expect("line_col promises the line number will never be below 1")
                {
                    ""
                } else {
                    Self::ELLIPSES
                }
            })?;
            writeln!(f)?;
            // draw the underlines
            Self::underlines(f, start_line, end_line, line_num_width, block, line_items)?;
            Self::inline_messages(f, line_items, line_num_width)?;
        }
        writeln!(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct InlineErrMsg<'src, 'err>(&'err ErrorType<'src>);

impl Punctuation {
    fn op_description(self, operands: &[ValueType]) -> &'static str {
        #[allow(clippy::enum_glob_use, reason = "we're using all of them")]
        use Punctuation::*;
        match (self, operands) {
            (Not, [ValueType::Bool]) => "logical 'not'",
            (Not, [_]) => "bitflip",
            (MacroStringify, [_]) => "token stringification",
            (Rem, [_, _]) => "remainder",
            (And, [ValueType::Bool, ValueType::Bool]) => "logical 'and'",
            (And, [ValueType::Bool, _] | [_, ValueType::Bool]) => "logical/bitwise 'and'",
            (And, [_, _]) => "bitwise 'and'",
            (Mul, [_, _]) => "multiplication",
            (Add, [ValueType::Str, _] | [_, ValueType::Str]) => "string concatenation",
            (Add, [_, _]) => "addition",
            (SubNeg, [_, _]) => "subtraction",
            (SubNeg, [_]) => "arithmetic negation",
            (Div, [_, _]) => "division",
            (Ref, [_, _]) => "referencing",
            (Xor, [ValueType::Bool, ValueType::Bool]) => "logical 'xor'",
            (Xor, [ValueType::Bool, _] | [_, ValueType::Bool]) => "logical/bitwise 'xor'",
            (Xor, [_, _]) => "bitwise 'xor'",
            (Or, [ValueType::Bool, ValueType::Bool]) => "logical 'or'",
            (Or, [ValueType::Bool, _] | [_, ValueType::Bool]) => "logical/bitwise 'or'",
            (Or, [_, _]) => "bitwise 'or'",
            (Nand, [ValueType::Bool, ValueType::Bool]) => "logical 'nand'",
            (Nand, [ValueType::Bool, _] | [_, ValueType::Bool]) => "logical/bitwise 'nand'",
            (Nand, [_, _]) => "bitwise 'nand'",
            (Nor, [ValueType::Bool, ValueType::Bool]) => "logical 'nor'",
            (Nor, [ValueType::Bool, _] | [_, ValueType::Bool]) => "logical/bitwise 'nor'",
            (Nor, [_, _]) => "bitwise 'nor'",
            (Xnor, [ValueType::Bool, ValueType::Bool]) => "logical 'xnor'",
            (Xnor, [ValueType::Bool, _] | [_, ValueType::Bool]) => "logical/bitwise 'xnor'",
            (Xnor, [_, _]) => "bitwise 'xnor'",
            (MacroConcat, [_, _]) => "token concatenation",
            (Pow, [_, _]) => "arithmetic power",
            (Shl, [_, _]) => "left bitshift",
            (Shr, [_, _]) => "right bitshift",
            (Rotl, [_, _]) => "left bitwise rotate",
            (Rotr, [_, _]) => "right bitwise rotate",

            (Ne | Eq, [_, _]) => "equality",

            (Lt | Gt | Le | Ge, [_, _]) => "comparison",

            (LParen
            | RParen
            | Comma
            | Dot
            | Colon
            | Semi
            | Assign
            | QMark // TODO: will this be an operation?
            | LBrack
            | RBrack
            | LBrace
            | RBrace
            | RemAssign
            | AndAssign
            | MulAssign
            | AddAssign
            | SubAssign
            | Arrow
            | DotDot // TODO: will this be an operation?
            | DivAssign
            | PathSep
            | ColonEq
            | FatArrow
            | XorAssign
            | OrAssign
            | PowAssign
            | ShlAssign
            | ShrAssign
            | NandAssign
            | NorAssign
            | XnorAssign
            | RotlAssign
            | RotrAssign, _) => {
                unimplemented!("not an operator")
            }

            _ => unimplemented!("invalid combination of operands"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpSide {
    Left,
    Right,
}

impl std::fmt::Display for OpSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Left => "lhs",
            Self::Right => "rhs",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TypeResolutionMsg {
    side: OpSide,
    ty: ValueType,
}

impl std::fmt::Display for TypeResolutionMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { side, ty } = self;
        write!(f, "{side}: type resolves to {ty}")
    }
}

/// Combines [`IntValue`], [`TargetTy`], and [`std::num::TryFromIntError`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntConversionFailure {
    UIntToSInt(usize),
    UIntToU32(usize),
    UIntToS32(usize),
    SIntToUInt(isize),
    SIntToU32(isize),
    SIntToS32(isize),
}

impl IntConversionFailure {
    pub const fn new(value: IntValue, target_ty: TargetTy) -> Option<Self> {
        match (value, target_ty) {
            (IntValue::UInt(x), TargetTy::SInt) => Some(Self::UIntToSInt(x)),
            (IntValue::UInt(x), TargetTy::U32) => Some(Self::UIntToU32(x)),
            (IntValue::UInt(x), TargetTy::S32) => Some(Self::UIntToS32(x)),
            (IntValue::SInt(x), TargetTy::UInt) => Some(Self::SIntToUInt(x)),
            (IntValue::SInt(x), TargetTy::U32) => Some(Self::SIntToU32(x)),
            (IntValue::SInt(x), TargetTy::S32) => Some(Self::SIntToS32(x)),

            // Infallible
            (IntValue::UInt(_), TargetTy::UInt) | (IntValue::SInt(_), TargetTy::SInt) => None,
        }
    }

    pub const fn reason(&self) -> &'static str {
        match self {
            Self::UIntToSInt(_)
            | Self::UIntToU32(_)
            | Self::UIntToS32(_)
            | Self::SIntToU32(0..)
            | Self::SIntToS32(0..) => "number too large to fit in target type",

            Self::SIntToUInt(_) | Self::SIntToU32(..0) | Self::SIntToS32(..0) => {
                "number too small to fit in target type"
            }
        }
    }

    pub const fn target_ty(self) -> TargetTy {
        match self {
            Self::UIntToSInt(_) => TargetTy::SInt,
            Self::UIntToU32(_) | Self::SIntToU32(_) => TargetTy::U32,
            Self::UIntToS32(_) | Self::SIntToS32(_) => TargetTy::S32,
            Self::SIntToUInt(_) => TargetTy::UInt,
        }
    }

    pub const fn value(self) -> IntValue {
        match self {
            Self::UIntToSInt(x) | Self::UIntToU32(x) | Self::UIntToS32(x) => IntValue::UInt(x),
            Self::SIntToUInt(x) | Self::SIntToU32(x) | Self::SIntToS32(x) => IntValue::SInt(x),
        }
    }
}

impl std::fmt::Display for IntConversionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reason().fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetTy {
    UInt,
    SInt,
    U32,
    S32,
}

impl TargetTy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UInt => "an unsigned integer",
            Self::SInt => "a signed integer",
            Self::U32 => "an unsigned 32-bit integer",
            Self::S32 => "a signed 32-bit integer",
        }
    }

    #[allow(clippy::as_conversions)]
    const fn bounds(self) -> (u32, i128, i128) {
        match self {
            Self::UInt => (usize::BITS, usize::MIN as i128, usize::MAX as i128),
            Self::SInt => (isize::BITS, isize::MIN as i128, isize::MAX as i128),
            Self::U32 => (u32::BITS, u32::MIN as i128, u32::MAX as i128),
            Self::S32 => (i32::BITS, i32::MIN as i128, i32::MAX as i128),
        }
    }
}

impl std::fmt::Display for TargetTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct FailedConversionMsg {
    op: Punctuation,
    side: OpSide,
    failure: IntConversionFailure,
    is_binary: bool,
}

impl FailedConversionMsg {
    const fn new(
        op: Punctuation,
        side: OpSide,
        failure: IntConversionFailure,
        is_binary: bool,
    ) -> Self {
        Self {
            op,
            side,
            failure,
            is_binary,
        }
    }
}

impl std::fmt::Display for FailedConversionMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::cmp::Ordering;
        let Self {
            op,
            side,
            failure,
            is_binary,
        } = self;
        let target_ty = failure.target_ty();
        let (_bits, min, max) = target_ty.bounds();
        let value = failure.value();
        #[expect(
            clippy::as_conversions,
            reason = "i128 can fit any usize/isize with 64 bits or fewer, but doesn't implement From<usize>/From<isize>"
        )]
        let n = match value {
            IntValue::UInt(val) => cfg_select! {
                any(
                    target_pointer_width = "16",
                    target_pointer_width = "32",
                    target_pointer_width = "64"
                ) => val as i128,
            },
            IntValue::SInt(val) => cfg_select! {
                any(
                    target_pointer_width = "16",
                    target_pointer_width = "32",
                    target_pointer_width = "64"
                ) => val as i128,
            },
        };
        let (broken_bound, ord) = if n < min {
            (min, Ordering::Less)
        } else {
            debug_assert!(n > max, "conversion should not have failed");
            (max, Ordering::Greater)
        };
        write!(
            f,
            "expression evaluated to {value:?}.\n\
            {} requires {side} to be {target_ty}, which must be between {min} and {max}\n\
            ({n} {} {broken_bound})",
            // HACK: not the actual value types, but correct quantity. should be fine as long as uint vs sint doesn't change the description
            op.op_description(if *is_binary {
                [ValueType::SInt, ValueType::SInt].as_slice()
            } else {
                [ValueType::SInt].as_slice()
            }),
            match ord {
                Ordering::Less => '<',
                Ordering::Greater => '>',
                Ordering::Equal => unreachable!(),
            }
        )
    }
}

impl std::fmt::Display for RenderedContextError<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = InlineErrMsg(&self.0.err);
        LineRefs::new(
            self.0.source,
            {
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
        )
        .fmt(f)
    }
}
