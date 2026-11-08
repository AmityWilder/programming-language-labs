use super::StreamSpan;
use crate::{
    lex::token::TokenKind,
    src_code::span::{FileId, Span, uspan},
};

/// `Macro`
pub type MacroName = StreamSpan;

/// `MParam`
pub type Param = StreamSpan;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Document {
    pub items: Vec<DocumentItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum DocumentItem {
    Def(Definition),
    Call(Call),
    Cond(Conditional),
    Const(MacroConst),
    Other(Tokens),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct MacroConst {
    pub tilde: StreamSpan,
    pub name: MacroName,
}

impl MacroConst {
    pub const fn first_span(&self) -> &StreamSpan {
        &self.tilde
    }

    pub const fn last_span(&self) -> &StreamSpan {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Definition {
    pub kw: StreamSpan,
    pub name: MacroName,
    pub params: Option<ParamGroup>,
    pub body: Replacer,

    /// The total lenth of the item in bytes, including whitespace and comments
    pub len: uspan,
}

impl Definition {
    pub const fn first_span(&self) -> &StreamSpan {
        &self.kw
    }

    pub const fn last_span(&self) -> &StreamSpan {
        self.body.last_span()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct ParamGroup {
    /// `(`
    pub open: StreamSpan,
    pub params: Vec<Param>,
    /// `)`
    pub close: StreamSpan,
}

impl ParamGroup {
    pub const fn first_span(&self) -> &StreamSpan {
        &self.open
    }

    pub const fn last_span(&self) -> &StreamSpan {
        &self.close
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Replacer {
    /// `{`
    pub open: StreamSpan,
    pub items: Tokens,
    /// `}`
    pub close: StreamSpan,
}

impl Replacer {
    pub const fn first_span(&self) -> &StreamSpan {
        &self.open
    }

    pub const fn last_span(&self) -> &StreamSpan {
        &self.close
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Tokens {
    pub list: Vec<(StreamSpan, TokenKind)>,
}

impl Tokens {
    pub const fn first_span(&self) -> Option<&StreamSpan> {
        if let Some((span, _)) = self.list.first() {
            Some(span)
        } else {
            None
        }
    }

    pub const fn last_span(&self) -> Option<&StreamSpan> {
        if let Some((span, _)) = self.list.last() {
            Some(span)
        } else {
            None
        }
    }

    pub fn max_def(&self, file: FileId) -> Option<Span> {
        self.list
            .iter()
            .map(|(x, _)| x.into_span(file))
            .try_reduce(Span::try_super_union)
            .flatten()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Call {
    pub name: MacroName,
    pub args: Vec<Replacer>,

    /// The total lenth of the item in bytes, including whitespace and comments
    pub len: uspan,
}

impl Call {
    pub const fn first_span(&self) -> &StreamSpan {
        &self.name
    }

    pub const fn last_span(&self) -> &StreamSpan {
        if let Some(arg) = self.args.last() {
            arg.last_span()
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Conditional {
    /// `rot`
    pub kw: StreamSpan,
    /// Macros and conditions should already be expanded
    pub pred: Replacer,
    pub then: Replacer,
    pub alt: Option<Alternative>,
}

impl Conditional {
    pub const fn first_span(&self) -> &StreamSpan {
        &self.kw
    }

    pub const fn last_span(&self) -> &StreamSpan {
        if let Some(alt) = &self.alt {
            alt.last_span()
        } else {
            self.then.last_span()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Alternative {
    /// `tor`
    pub kw: StreamSpan,
    pub body: AlternativeBody,
}

impl Alternative {
    pub const fn first_span(&self) -> &StreamSpan {
        &self.kw
    }

    pub const fn last_span(&self) -> &StreamSpan {
        self.body.last_span()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum AlternativeBody {
    More(Box<Conditional>),
    Last(Replacer),
}

impl AlternativeBody {
    pub const fn first_span(&self) -> &StreamSpan {
        match self {
            Self::More(x) => (*x).first_span(),
            Self::Last(x) => x.first_span(),
        }
    }

    pub const fn last_span(&self) -> &StreamSpan {
        match self {
            Self::More(x) => (*x).last_span(),
            Self::Last(x) => x.last_span(),
        }
    }
}
