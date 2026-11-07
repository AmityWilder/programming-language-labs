use crate::{
    lex::token::{keyword::Keyword, punc::Punctuation},
    parse::ast::ExprPrecedence,
    src_code::{
        code::{LineContentSpans, SourceCode},
        span::{RelSpan, usize_to_uspan, uspan},
    },
};
use std::range::Range;

/// Helper macro for preventing issues with missed variants when adding new ones
///
/// Variants should be in the order they should be tested
macro_rules! define_token_eq {
    (
        $(#[$em:meta])*
        $vis:vis enum $Enum:ident = $name:ident {$(
            $(#[doc = $doc:expr])*
            $Variant:ident = $value:expr
        ),+ $(,)?}
    ) => {
        $(#[$em])*
        $vis enum $Enum {$(
            #[doc = concat!("`", $value, "`\n")]
            $(#[doc = $doc])*
            $Variant,
        )+}

        #[allow(dead_code, reason = "not always used in all expressions of this macro")]
        impl $Enum {
            /// Matches the prefix of `s` to a variant of [`Self`], finding the longest match possible.
            pub fn from_prefix(s: &str) -> Option<Self> {
                $name::OPTIONS
                    .into_iter()
                    .filter(|(pat, _)| s.starts_with(pat))
                    .max_by_key(|(pat, _)| pat.len()) // maximal munch
                    .map(|(_, punc)| punc)
            }

            /// Like [`Self::from_prefix`], but matches the full string
            pub const fn try_from_str(s: &str) -> Option<Self> {
                match s {
                    $($value => Some(Self::$Variant),)+
                    _ => None,
                }
            }

            /// The constant string name of the token
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $value),+
                }
            }

            pub const fn expectation_name(self) -> &'static str {
                match self {
                    $(Self::$Variant => concat!("`", $value, "`")),+
                }
            }
        }

        impl std::fmt::Display for $Enum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        pub mod $name {
            use super::*;

            pub static OPTIONS: std::sync::LazyLock<[(&str, $Enum); [$($Enum::$Variant),+].len()]> = std::sync::LazyLock::new(||{
                let mut list: [(&'static str, $Enum); _] = [
                    $(($value, $Enum::$Variant),)+
                ];
                list.sort_by_key(|(name, _)| *name);
                list
            });

            #[cfg(test)]
            #[expect(non_snake_case)]
            mod tests {
                use super::*;

                $(#[test]
                fn $Variant() {
                    assert_eq!($Enum::from_prefix($value), Some($Enum::$Variant));
                })+
            }
        }
    };
}

pub mod keyword;
pub mod punc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum IdentKind {
    #[default]
    Variable,
    Constant,
    Function,
    Typename,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum KeywordKind {
    #[default]
    Normal,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PuncKind {
    #[default]
    Normal,
    /// Negative separated from its number
    NumberLit,
    Keyword(KeywordKind),
    Bracket(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum NumberSuffix {
    #[default]
    Invalid,
    /// `s`
    Signed,
    /// `u`
    Unsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum IdentCase {
    #[default]
    Other,
    Snake,
    Pascal,
    Screaming,
}

impl From<&str> for IdentCase {
    fn from(value: &str) -> Self {
        // PascalCase is allowed to contain underscores, as long as they aren't infix
        let value = value.trim_matches('_');
        let mut chars = value.chars();
        if chars
            .find(|ch| ch.is_alphabetic())
            .is_some_and(char::is_uppercase)
        {
            if chars.any(char::is_lowercase) {
                if value.contains('_') {
                    Self::Other
                } else {
                    debug_assert!(
                        !value.contains('_')
                            && value
                                .trim_start_matches(char::is_numeric)
                                .starts_with(char::is_uppercase)
                            && value.contains(char::is_lowercase),
                        "expected PascalCase"
                    );
                    Self::Pascal
                }
            } else {
                debug_assert!(
                    value
                        .chars()
                        .all(|ch| ch == '_' || ch.is_uppercase() || ch.is_numeric()),
                    "expected SCREAMING_SNAKE_CASE"
                );
                Self::Screaming
            }
        } else {
            debug_assert!(
                value
                    .chars()
                    .all(|ch| ch == '_' || ch.is_lowercase() || ch.is_numeric()),
                "expected snake_case"
            );
            Self::Snake
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TokenKind {
    /// An error
    #[default]
    Invalid,

    /// An entire chunk of whitespace, not just one character
    WSpace,
    /// A comment (either block or line)
    Comment,

    /// Boolean literal
    BoolLit(bool),
    /// Any number literal
    NumberLit,
    /// Character literal
    CharLit,
    /// String literal
    TextLit,

    /// An identifier - its value is the lexeme itself
    Ident,
    /// An identifier prefixed with `\`
    Macro,
    /// An identifier prefixed with `$`
    MParam,

    /// A language keyword
    Keyword(Keyword),
    /// Operators and other non-alphanumeric tokens
    Punc(Punctuation),
}

impl TokenKind {
    pub const fn is_ignored(self) -> bool {
        matches!(self, Self::WSpace | Self::Comment)
    }
}

impl From<Keyword> for TokenKind {
    fn from(value: Keyword) -> Self {
        Self::Keyword(value)
    }
}

impl From<Punctuation> for TokenKind {
    fn from(value: Punctuation) -> Self {
        Self::Punc(value)
    }
}

impl TokenKind {
    pub const fn expectation_name(self) -> &'static str {
        match self {
            Self::Invalid => panic!("not implemented: should not expect an invalid token"),
            Self::WSpace => panic!("not implemented: should not expect whitespace"),
            Self::Comment => panic!("not implemented: should not expect a comment"),

            Self::BoolLit(_) => "a boolean literal",
            Self::NumberLit => "a number literal",
            Self::CharLit => "a character literal",
            Self::TextLit => "a text literal",
            Self::Ident => "an identifier",
            Self::Macro => "a macro name",
            Self::MParam => "a macro parameter",
            Self::Keyword(keyword) => keyword.expectation_name(),
            Self::Punc(punc) => punc.expectation_name(),
        }
    }

    pub const fn to_token_eq(self) -> Option<TokenEq> {
        match self {
            Self::Keyword(kw) => Some(TokenEq::Keyword(kw)),
            Self::Punc(punc) => Some(TokenEq::Punctuation(punc)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenSpans<'tbl> {
    acc: uspan,
    iter: std::slice::Iter<'tbl, (uspan, TokenKind)>,
}

impl<'tbl> TokenSpans<'tbl> {
    fn new(iter: &'tbl [(uspan, TokenKind)]) -> Self {
        Self {
            acc: 0,
            iter: iter.iter(),
        }
    }
}

impl Iterator for TokenSpans<'_> {
    type Item = (RelSpan, TokenKind);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&(len, kind)) = self.iter.next() {
            let start = self.acc;
            let end = start.strict_add(len);
            self.acc = end;
            return Some((RelSpan::from(start..end), kind));
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash, Default)]
enum TokenLinesWithinState {
    Ahead {
        start: uspan,
        end: uspan,
    },
    Within {
        end: uspan,
    },
    #[default]
    After,
}

#[derive(Debug, Clone)]
pub struct TokenLinesWithin<'src, 'tbl> {
    iter: TokenLines<'src, 'tbl>,
    state: TokenLinesWithinState,
}

impl<'src, 'tbl> TokenLinesWithin<'src, 'tbl> {
    const fn new(iter: TokenLines<'src, 'tbl>, RelSpan(Range { start, end }): RelSpan) -> Self {
        Self {
            iter,
            state: TokenLinesWithinState::Ahead { start, end },
        }
    }
}

impl Iterator for TokenLinesWithin<'_, '_> {
    type Item = (RelSpan, TokenKind);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                TokenLinesWithinState::Ahead { start, end } => {
                    if let item @ Some((span, _)) = self.iter.next()
                        && span.end > start
                    {
                        self.state = TokenLinesWithinState::Within { end };
                        break item;
                    }
                }

                TokenLinesWithinState::Within { end } => {
                    if let item @ Some((span, _)) = self.iter.next()
                        && span.start < end
                    {
                        break item;
                    }
                    self.state = TokenLinesWithinState::After;
                }

                TokenLinesWithinState::After => break None,
            }
        }
    }
}

/// **Note:** Not ever Item output from this iterator is a true token.
/// This iterator may break tokens apart around newlines.
#[derive(Debug, Clone)]
pub struct TokenLines<'src, 'tbl> {
    lines: std::iter::Peekable<LineContentSpans<'src>>,
    tokens: std::iter::Peekable<TokenSpans<'tbl>>,
}

impl<'src, 'tbl> TokenLines<'src, 'tbl> {
    fn new(line_content_spans: LineContentSpans<'src>, tokens: TokenSpans<'tbl>) -> Self {
        Self {
            lines: line_content_spans.peekable(),
            tokens: tokens.peekable(),
        }
    }

    pub const fn overlapping(self, span: RelSpan) -> TokenLinesWithin<'src, 'tbl> {
        TokenLinesWithin::new(self, span)
    }
}

impl Iterator for TokenLines<'_, '_> {
    type Item = (RelSpan, TokenKind);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&mut (ref mut span, kind)) = self.tokens.peek_mut() {
            debug_assert!(
                !self.lines.peek().is_some_and(|span| span.is_empty()),
                "`lines` should iterate when emptied"
            );
            let line_span = self
                .lines
                .peek_mut()
                .expect("ran out of lines before visiting every token");

            match span.end.cmp(&line_span.end) {
                // line is incomplete
                std::cmp::Ordering::Less => {
                    debug_assert!(
                        line_span.start <= span.end,
                        "line should not iterate while it still contains tokens"
                    );
                    line_span.start = span.end;
                    self.tokens.next()
                }

                // token completes line
                std::cmp::Ordering::Equal => {
                    _ = self.lines.next();
                    self.tokens.next()
                }

                // token contains linebreak(s)
                std::cmp::Ordering::Greater => {
                    let Some(line_span) = self.lines.next() else {
                        unreachable!("already peeked");
                    };
                    let item = (
                        RelSpan(Range {
                            start: span.start,
                            end: line_span.end,
                        }),
                        kind,
                    );
                    if let Some(next_line_span) = self.lines.peek() {
                        span.start = next_line_span.start;
                    } else {
                        // TODO: this assertion doesn't cover the case where the span includes an extra character
                        // the size of a newline while `lines` *doesn't* have a trailing newline
                        debug_assert_eq!(
                            (span.len().checked_sub(line_span.len()), kind),
                            (Some(usize_to_uspan('\n'.len_utf8())), TokenKind::WSpace),
                            "the absence of an upcoming line in the presence of a token containing a newline should \
                            imply the token ends with the newline (which is only possible for whitespace tokens)"
                        );
                        _ = self.tokens.next();
                    }
                    Some(item)
                }
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash, Default)]
pub struct TokenTable {
    /// (length, kind)
    pub tokens: Vec<(uspan, TokenKind)>,
}

impl TokenTable {
    pub const fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    pub fn iter(&self) -> TokenSpans<'_> {
        TokenSpans::new(&self.tokens)
    }

    /// Splits tokens into chunks around newline characters
    pub fn lines<'src, 'tbl>(&'tbl self, code: &'src SourceCode) -> TokenLines<'src, 'tbl> {
        TokenLines::new(code.line_content_spans(), self.iter())
    }
}

impl<'tbl> IntoIterator for &'tbl TokenTable {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = TokenSpans<'tbl>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A [`LexValue`] where the lexeme can be represented by a static string
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum TokenEq {
    Keyword(Keyword),
    Punctuation(Punctuation),
}

impl TokenEq {
    pub const fn as_str(self) -> &'static str {
        match self {
            TokenEq::Keyword(kw) => kw.as_str(),
            TokenEq::Punctuation(punc) => punc.as_str(),
        }
    }

    // pub fn op_description(self, operands: &[ValueType]) -> &'static str {
    //     match self {
    //         TokenEq::Keyword(kw) => kw.op_description(operands),
    //         TokenEq::Punctuation(punc) => punc.op_description(operands),
    //     }
    // }
}

impl std::fmt::Display for TokenEq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

macro_rules! define_operators {
    (
        $(#[$em:meta])*
        $vis:vis enum $Enum:ident : $Base:ident {$(
            $(#[$vm:meta])*
            $BaseVariant:ident::$Variant:ident
        ),* $(,)?}
    ) => {
        $(#[$em])*
        $vis enum $Enum {$(
            $(#[$vm])*
            $Variant
        ),*}

        impl Operator {
            pub const fn from_token_eq(value: $Base) -> Option<Self> {
                $(#[allow(unused_imports)] use $BaseVariant::$Variant;)*
                Some(match value {
                    $($Base::$BaseVariant($BaseVariant::$Variant) => Self::$Variant,)*
                    _ => return None,
                })
            }

            pub const fn to_token_eq(self) -> $Base {
                $(use $BaseVariant::$Variant;)*
                match self {
                    $(Self::$Variant => $Base::$BaseVariant($Variant),)*
                }
            }
        }
    };
}

define_operators! {
/// Subset of [`TokenEq`] specific to operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Operator : TokenEq {
    #[default]
    Punctuation::Assign,

    Punctuation::And,
    Punctuation::Or,
    Punctuation::Nand,
    Punctuation::Nor,
    Punctuation::Xor,
    Punctuation::Xnor,

    Punctuation::AndAssign,
    Punctuation::OrAssign,
    Punctuation::XorAssign,
    Punctuation::NandAssign,
    Punctuation::NorAssign,
    Punctuation::XnorAssign,

    Punctuation::Not,
    Punctuation::Exists,

    Punctuation::Add,
    Punctuation::Incr,
    Punctuation::SubNeg,
    Punctuation::Decr,
    Punctuation::Mul,
    Punctuation::Pow,
    Punctuation::Div,
    Punctuation::Rem,

    Punctuation::AddAssign,
    Punctuation::SubAssign,
    Punctuation::MulAssign,
    Punctuation::PowAssign,
    Punctuation::DivAssign,
    Punctuation::RemAssign,

    Punctuation::Shl,
    Punctuation::Shr,
    Punctuation::Rotl,
    Punctuation::Rotr,

    Punctuation::ShlAssign,
    Punctuation::ShrAssign,
    Punctuation::RotlAssign,
    Punctuation::RotrAssign,

    Punctuation::Lt,
    Punctuation::Gt,
    Punctuation::Ne,
    Punctuation::Le,
    Punctuation::Eq,
    Punctuation::Ge,

    Punctuation::Coalesce,
    Punctuation::Ref,

    Punctuation::Convert,
    Punctuation::Transmute,

    Keyword::Szof,
}
}

impl TryFrom<TokenEq> for Operator {
    type Error = ();

    fn try_from(value: TokenEq) -> Result<Self, Self::Error> {
        value.to_operator().ok_or(())
    }
}

impl TryFrom<TokenKind> for Operator {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        value.to_token_eq().and_then(TokenEq::to_operator).ok_or(())
    }
}

impl Operator {
    /// Alias for [`Self::SubNeg`] to better imply what the code is doing
    #[expect(non_upper_case_globals)]
    pub const Sub: Self = Self::SubNeg;

    /// Alias for [`Self::SubNeg`] to better imply what the code is doing
    #[expect(non_upper_case_globals)]
    pub const Neg: Self = Self::SubNeg;

    pub const fn as_str(self) -> &'static str {
        self.to_token_eq().as_str()
    }

    // pub fn op_description(self, operands: &[ValueType]) -> &'static str {
    //     self.to_token_eq().op_description(operands)
    // }

    pub const fn precedence(&self) -> ExprPrecedence {
        match self {
            Self::Assign
            | Self::AndAssign
            | Self::OrAssign
            | Self::XorAssign
            | Self::NandAssign
            | Self::NorAssign
            | Self::XnorAssign
            | Self::AddAssign
            | Self::SubAssign
            | Self::MulAssign
            | Self::PowAssign
            | Self::DivAssign
            | Self::RemAssign
            | Self::ShlAssign
            | Self::ShrAssign
            | Self::RotlAssign
            | Self::RotrAssign => ExprPrecedence::Assignment,
            Self::And | Self::Nand => ExprPrecedence::And,
            Self::Or | Self::Nor => ExprPrecedence::Or,
            Self::Xor | Self::Xnor => ExprPrecedence::Xor,
            Self::Not | Self::Exists | Self::Ref | Self::Szof => ExprPrecedence::UnaryPre,
            Self::Add => ExprPrecedence::Term,
            Self::Incr | Self::Decr | Self::Coalesce => ExprPrecedence::UnaryPost,
            Self::SubNeg => unimplemented!(), // depends on unary vs binary
            Self::Mul | Self::Div | Self::Rem => ExprPrecedence::Factor,
            Self::Pow => ExprPrecedence::Exponent,
            Self::Shl | Self::Shr | Self::Rotl | Self::Rotr => ExprPrecedence::Shift,
            Self::Lt | Self::Gt | Self::Le | Self::Ge => ExprPrecedence::Comparison,
            Self::Ne | Self::Eq => ExprPrecedence::Equality,
            Self::Convert | Self::Transmute => ExprPrecedence::Conversion,
        }
    }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl TokenEq {
    pub const fn to_operator(self) -> Option<Operator> {
        Operator::from_token_eq(self)
    }
}

impl From<Operator> for TokenEq {
    fn from(value: Operator) -> Self {
        value.to_token_eq()
    }
}
