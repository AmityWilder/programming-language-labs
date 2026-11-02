use crate::{
    scanner::token::{punc::Punctuation, value::Keyword},
    src_code::span::{RelSpan, uspan},
};

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
pub enum TokenKind {
    /// An error
    #[default]
    Invalid,

    /// A single newline
    Newline,
    /// An entire chunk of whitespace, not just one character
    WSpace,
    /// A comment (either block or line)
    Comment,

    /// Boolean literal
    BoolLit,
    /// Any number literal
    NumberLit,
    /// Character literal
    CharLit,
    /// String literal
    TextLit,

    /// An identifier - its value is the lexeme itself
    Ident(IdentKind),
    /// An identifier prefixed with `\`
    Macro,
    /// An identifier prefixed with `$`
    MParam,

    /// A language keyword
    Keyword(Keyword, KeywordKind),
    /// Operators and other non-alphanumeric tokens
    Punc(Punctuation, PuncKind),
}

#[derive(Debug, Clone)]
pub struct TokenSpans<'a> {
    acc: uspan,
    iter: std::slice::Iter<'a, (uspan, TokenKind)>,
}

impl<'a> TokenSpans<'a> {
    fn new(iter: &'a [(uspan, TokenKind)]) -> Self {
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct TokenTable {
    /// (length, kind)
    pub(super) tokens: Vec<(uspan, TokenKind)>,
}

impl TokenTable {
    pub const fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    pub fn spans(&self) -> TokenSpans<'_> {
        TokenSpans::new(&self.tokens)
    }
}
