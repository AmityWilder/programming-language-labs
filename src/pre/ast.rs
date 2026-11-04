use crate::src_code::span::RelSpan;

/// `Macro`
pub type MacroName = RelSpan;

/// `MParam`
pub type Param = RelSpan;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Document {
    pub items: Vec<DocumentItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum DocumentItem {
    Def(Definition),
    Other(Tokens),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Definition {
    pub kw: RelSpan,
    pub name: MacroName,
    pub params: Option<ParamGroup>,
    pub body: Replacer,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct ParamGroup {
    /// `(`
    pub open: RelSpan,
    pub params: Vec<Param>,
    /// `)`
    pub close: RelSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Replacer {
    /// `{`
    pub open: RelSpan,
    pub items: Tokens,
    /// `}`
    pub close: RelSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Tokens {
    pub spans: Vec<RelSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Call {
    pub name: MacroName,
    pub body: Vec<Replacer>,
}
