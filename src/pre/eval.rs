use crate::{
    analyzer::types::PrimitiveType,
    error::Error,
    lex::token::TokenKind,
    src_code::{map::SourceMap, span::Span},
    types::{byte, sint, text, uint},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreVal<'src> {
    Typename(PrimitiveType),
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    Char(char),
    Text(text),
    Macro(&'src str),
}

pub fn eval<'src>(
    tokens: &[(Span, TokenKind)],
    source: &'src SourceMap,
) -> Result<PreVal<'src>, Error> {
    todo!("need to expand content before evaluating predicate")
}
