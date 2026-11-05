use crate::{Todo, lex::token::TokenKind, src_code::span::Span};

/// `Macro`
pub type MacroName = Span;

/// `MParam`
pub type Param = Span;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Document {
    pub items: Vec<DocumentItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum DocumentItem {
    Def(Definition),
    Call(Call),
    Cond(Conditional),
    Other(Tokens),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Definition {
    pub kw: Span,
    pub name: MacroName,
    pub params: Option<ParamGroup>,
    pub body: Replacer,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct ParamGroup {
    /// `(`
    pub open: Span,
    pub params: Vec<Param>,
    /// `)`
    pub close: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Replacer {
    /// `{`
    pub open: Span,
    pub items: Tokens,
    /// `}`
    pub close: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Tokens {
    pub list: Vec<(Span, TokenKind)>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Call {
    pub name: MacroName,
    pub args: Vec<Replacer>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Conditional {
    /// `rot`
    pub kw: Span,
    pub pred: Replacer,
    pub then: Replacer,
    pub alt: Option<Alternative>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Alternative {
    /// `tor`
    pub kw: Span,
    pub body: AlternativeBody,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum AlternativeBody {
    More(Box<Conditional>),
    Last(Replacer),
}
