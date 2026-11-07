use crate::{
    analyzer::types::PrimitiveType,
    error::Error,
    pre::ast,
    src_code::map::SourceMap,
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

pub fn eval<'src>(tokens: &ast::Tokens, source: &'src SourceMap) -> Result<PreVal<'src>, Error> {
    todo!("need to expand content before evaluating predicate")
}
