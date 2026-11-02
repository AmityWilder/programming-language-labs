//! Context-free grammar

#![deprecated = "use crate::parse instead"]

use crate::{
    analyzer::types::bounds::Combinator,
    grammar::ast::{
        Assignment, BlockExpr, BlockStmt, DeclVar, Declaration, DeferParam, DoneStmt, ExprStmt,
        IfStmt, OrBody, OrStmt, PrintStmt, Program, Statement, Variable,
    },
    lex::token::Operator,
    old_error::{ContextError, ExpectedToken},
    scanner::token::{
        Lexeme, SpecToken, Token,
        punc::{Bracket, OpSide, Punctuation},
        value::{Keyword, LexValue},
    },
};
use Operator::*;

pub mod ast;
// pub mod ast_iter; // TODO: FIX THIS
pub mod fmt;

#[cfg(test)]
pub mod tests;

macro_rules! token_matches {
    ($token:expr, $($Variant:ident$(($pattern:pat))?$(::$SubVariant:ident)?$(::{$($SubVariants:ident)|*})?)|+) => {{
        #[allow(unused_imports, reason = "only here to allow proper semantic highlighting")] {
            $($({ use $crate::scanner::token::value::$Variant::$SubVariant; })?)*
            $($({ use $crate::scanner::token::value::$Variant::{$($SubVariants),*}; })?)*
        }
        matches!($token.val, $($crate::scanner::token::value::LexValue::$Variant
            $(($pattern))?
            $(($crate::scanner::token::value::$Variant::$SubVariant))?
            $(($($crate::scanner::token::value::$Variant::$SubVariants)|*))?
        )|+)
    }};
}
pub(crate) use token_matches;

macro_rules! match_token {
    ($($Variant:ident$(($pattern:pat))?$(::$SubVariant:ident)?$(::{$($SubVariants:ident)|*})?)|+) => {
        |token| -> bool { $crate::grammar::token_matches!(token, $($Variant$(($pattern))?$(::$SubVariant)?$(::{$($SubVariants)|*})?)|+) }
    };
}
pub(crate) use match_token;

#[derive(Debug, Clone)]
pub struct Parser<'src, I: Iterator<Item = Token<'src>>> {
    source: &'src str,
    tokens: std::iter::Peekable<I>,
}

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    fn new(source: &'src str, tokens: I) -> Self {
        Self {
            source,
            tokens: tokens.peekable(),
        }
    }
}

impl<'src, I> Iterator for Parser<'src, I>
where
    I: Iterator<Item = Token<'src>>,
{
    type Item = Result<ast::Expr<'src>, ContextError<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens
            .peek()
            .is_some()
            .then(|| self.expression().inspect_err(|_| self.synchronize()))
    }
}

pub fn parse<'src, A>(
    source: &'src str,
    tokens: A,
) -> Parser<'src, std::iter::Filter<<A as IntoIterator>::IntoIter, impl FnMut(&Token<'src>) -> bool>>
where
    A: IntoIterator<IntoIter: 'src, Item = Token<'src>>,
{
    Parser::new(
        source,
        tokens
            .into_iter()
            .filter(|token| !token_matches!(token, Whitespace | Comment)),
    )
}

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    /// Returns the next token if it is a `<`, possibly tearing the
    /// leading `<` off of another token (such as `<<` or `<=`) to do so.
    ///
    /// Special case: `<=>` is left unharmed, as it is more likely to be a bound on a generic type than the argument.
    ///
    /// Used in generic types
    fn rip_langle(&mut self) -> Option<Lexeme<'src>> {
        use Punctuation::{Assign, Iff, Le, Lt, Shl, ShlAssign};
        const LANGLE: char = '<';
        self.pull_if_eq(LexValue::Punctuation(Lt)).or_else(|| {
            self.tokens
                .peek_mut()
                .filter(|next| next.lex.starts_with(LANGLE))
                .and_then(|next| {
                    next.val = match next.val {
                        LexValue::Punctuation(Lt) => {
                            unreachable!("`<` should have been handled by pull_if_eq")
                        }
                        LexValue::Punctuation(Iff) => return None,

                        LexValue::Punctuation(Le) => LexValue::Punctuation(Assign),
                        LexValue::Punctuation(Shl) => LexValue::Punctuation(Lt),
                        LexValue::Punctuation(ShlAssign) => LexValue::Punctuation(Le),

                        x => {
                            if cfg!(debug_assertions) {
                                todo!("unhandled: `{x:?}` starts with `<`")
                            } else {
                                return None;
                            }
                        }
                    };
                    let torn;
                    (torn, next.lex.src) = next.lex.src.split_at(LANGLE.len_utf8());
                    Some(Lexeme {
                        src: torn,
                        mac: next.mac,
                    })
                })
        })
    }

    /// Returns the next token if it is a `>`, possibly tearing the
    /// leading `>` off of another token (such as `>>` or `>=`) to do so.
    ///
    /// Used in generic types
    fn rip_rangle(&mut self) -> Option<Lexeme<'src>> {
        use Punctuation::{Assign, Ge, Gt, Shr, ShrAssign};
        const RANGLE: char = '>';
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Gt))
            .or_else(|| {
                self.tokens
                    .peek_mut()
                    .filter(|next| next.lex.starts_with(RANGLE))
                    .and_then(|next| {
                        next.val = match next.val {
                            LexValue::Punctuation(Gt) => {
                                unreachable!("`>` should have been handled by pull_if_eq")
                            }

                            LexValue::Punctuation(Ge) => LexValue::Punctuation(Assign),
                            LexValue::Punctuation(Shr) => LexValue::Punctuation(Gt),
                            LexValue::Punctuation(ShrAssign) => LexValue::Punctuation(Ge),

                            x => {
                                if cfg!(debug_assertions) {
                                    todo!("unhandled: `{x:?}` starts with `>`")
                                } else {
                                    return None;
                                }
                            }
                        };
                        let torn;
                        (torn, next.lex.src) = next.lex.src.split_at(RANGLE.len_utf8());
                        Some(Lexeme {
                            src: torn,
                            mac: next.mac,
                        })
                    })
            })
    }

    /// Optional token
    fn pull_if<P>(&mut self, p: P) -> Option<Token<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.tokens.next_if(p)
    }

    /// Optional token
    fn pull_if_eq(&mut self, val: LexValue<'src>) -> Option<Lexeme<'src>> {
        self.tokens
            .next_if(|token| token.val == val)
            .map(Lexeme::from)
    }

    /// Optional token
    fn pull_into_if<T, P>(&mut self, p: P) -> Option<SpecToken<'src, T>>
    where
        Token<'src>: TryInto<SpecToken<'src, T>>,
        P: FnOnce(&SpecToken<'src, T>) -> bool,
    {
        self.tokens
            .next_if_map(|token| Token::try_into(token).ok().filter(p).ok_or(token))
    }

    /// Optional token
    fn pull_into<T>(&mut self) -> Option<SpecToken<'src, T>>
    where
        Token<'src>: TryInto<SpecToken<'src, T>>,
    {
        self.pull_into_if(|_| true)
    }

    /// Optional operator-token
    fn pull_into_if_in<T>(&mut self, matching: &[T]) -> Option<SpecToken<'src, T>>
    where
        Token<'src>: TryInto<SpecToken<'src, T>>,
        T: PartialEq,
    {
        self.pull_into_if(|token| matching.contains(&token.val))
    }

    /// Required token
    fn try_pull<P>(
        &mut self,
        p: P,
        expected: ExpectedToken,
    ) -> Result<Token<'src>, ContextError<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.pull_if(p).ok_or_else(|| self.error(expected))
    }

    /// Required token
    fn try_pull_eq(
        &mut self,
        val: LexValue<'src>,
        expected: ExpectedToken,
    ) -> Result<Lexeme<'src>, ContextError<'src>> {
        self.pull_if_eq(val).ok_or_else(|| self.error(expected))
    }

    /// Required token
    fn try_pull_into_if<T, P>(
        &mut self,
        p: P,
        expected: ExpectedToken,
    ) -> Result<SpecToken<'src, T>, ContextError<'src>>
    where
        Token<'src>: TryInto<SpecToken<'src, T>>,
        P: FnOnce(&SpecToken<'src, T>) -> bool,
    {
        self.pull_into_if(p).ok_or_else(|| self.error(expected))
    }

    /// Required token
    fn try_pull_into<T>(
        &mut self,
        expected: ExpectedToken,
    ) -> Result<SpecToken<'src, T>, ContextError<'src>>
    where
        Token<'src>: TryInto<SpecToken<'src, T>>,
    {
        self.try_pull_into_if(|_| true, expected)
    }

    /// Required token
    fn try_pull_into_if_in<T>(
        &mut self,
        matching: &[T],
        expected: ExpectedToken,
    ) -> Result<SpecToken<'src, T>, ContextError<'src>>
    where
        Token<'src>: TryInto<SpecToken<'src, T>>,
        T: PartialEq,
    {
        self.try_pull_into_if(|token| matching.contains(&token.val), expected)
    }

    pub fn error(&mut self, expected: ExpectedToken) -> ContextError<'src> {
        match self.tokens.peek() {
            Some(token) => ContextError::unexpected(token, self.source, expected),
            None => ContextError::missing(self.source, expected),
        }
    }

    fn synchronize(&mut self) {
        while let Some(token) = self.tokens.next() {
            // end of current statement
            if token_matches!(token, Punctuation::Semi) {
                break;
            }

            // start of new statement/definition
            if self.tokens.peek().is_some_and(match_token!(Keyword::{
                Rec | Sup | Cat | Alt | Sub | Def | Fun | Mem | Let | Uni | Pvt | If |
                Or | Pick | Rep | For | Loop | Cord | Done | Skip | Give | Fail | Emit | Prnt
            })) {
                break;
            }
        }
    }

    /// `program -> ( statement )*`
    // TODO: should this return multiple errors?
    pub fn program(
        &mut self,
    ) -> Result<Program<'src>, impl IntoIterator<Item = ContextError<'src>>> {
        let mut iter = std::iter::from_fn(|| {
            self.tokens
                .peek()
                .is_some()
                .then(|| self.declaration().inspect_err(|_| self.synchronize()))
        });
        let (lo, hi) = iter.size_hint();
        let mut decls = Vec::with_capacity(hi.unwrap_or(lo));
        while let Some(res) = iter.next() {
            match res {
                Ok(stmt) => decls.push(stmt),
                Err(first) => {
                    return Err(std::iter::once(first).chain(iter.filter_map(Result::err)));
                }
            }
        }
        Ok(Program { decls })
    }

    /// `declaration -> var_declaration | defer_declaration | statement`
    pub fn declaration(&mut self) -> Result<Declaration<'src>, ContextError<'src>> {
        self.var_declaration()
            .map(|res| res.map(Declaration::var))
            .or_else(|| {
                self.defer_declaration()
                    .map(|res| res.map(Declaration::defer))
            })
            .or_else(|| Some(self.statement().map(Declaration::stmt)))
            .ok_or_else(|| self.error(ExpectedToken::Declaration))
            .flatten()
    }

    /// `assignment -> "=" expression`
    pub fn assignment(&mut self) -> Option<Result<Assignment<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Assign))
            .map(|op| {
                Ok(Assignment {
                    op,
                    expr: self.expression()?,
                })
            })
    }

    /// `var_declaration -> ( "uni" | "pvt" | "let" ) pattern ( type_annotation )? ( assignment )? ";"`
    pub fn var_declaration(&mut self) -> Option<Result<DeclVar<'src>, ContextError<'src>>> {
        self.pull_into().map(|var_kw| {
            Ok(DeclVar {
                var_kw,
                binding: self
                    .bind_pattern()
                    .ok_or_else(|| self.error(ExpectedToken::Pattern))?,
                ty: self.type_annotation().transpose()?,
                assignment: self.assignment().transpose()?,
                semi: self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::Semi),
                    ExpectedToken::Semi,
                )?,
            })
        })
    }

    /// `type_annotation -> ":" type_expression`
    pub fn type_annotation(
        &mut self,
    ) -> Option<Result<ast::TypeAnnotation<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Colon))
            .map(|colon| {
                Ok(ast::TypeAnnotation {
                    colon,
                    ty: self.type_expression()?,
                })
            })
    }

    /// `statement -> block_statement
    ///     | print_statement
    ///     | done_statement
    ///     | skip_statement
    ///     | if_statement
    ///     | loop_statement
    ///     | expression_statement`
    pub fn statement(&mut self) -> Result<Statement<'src>, ContextError<'src>> {
        self.block_statement()
            .map(|res| res.map(Statement::Block))
            .or_else(|| self.print_statement().map(|res| res.map(Statement::Print)))
            .or_else(|| self.done_statement().map(|res| res.map(Statement::Done)))
            .or_else(|| self.skip_statement().map(|res| res.map(Statement::Skip)))
            .or_else(|| self.if_statement().map(|res| res.map(Statement::If)))
            .or_else(|| self.loop_block().map(|res| res.map(Statement::Loop)))
            .or_else(|| Some(self.expression_statement().map(Statement::Expr)))
            .ok_or_else(|| self.error(ExpectedToken::Statement))
            .flatten()
    }

    /// `expression_statement -> assign ";"`
    pub fn expression_statement(&mut self) -> Result<ExprStmt<'src>, ContextError<'src>> {
        Ok(ExprStmt {
            expr: self.assign()?,
            semi: self.try_pull_eq(
                LexValue::Punctuation(Punctuation::Semi),
                ExpectedToken::Semi,
            )?,
        })
    }

    /// `print_statement -> "prnt" expression ";"`
    pub fn print_statement(&mut self) -> Option<Result<PrintStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Prnt)).map(|kw| {
            Ok(PrintStmt {
                kw,
                group: self
                    .group()
                    .ok_or_else(|| self.error(ExpectedToken::LParen))
                    .flatten()?,
                semi: self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::Semi),
                    ExpectedToken::Semi,
                )?,
            })
        })
    }

    /// `defer_declaration -> "defer" ( defer_param )? block_statement`
    pub fn defer_declaration(
        &mut self,
    ) -> Option<Result<ast::DeclDefer<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Defer))
            .map(|kw| {
                Ok(ast::DeclDefer {
                    kw,
                    param: self.defer_param().transpose()?,
                    on_pop: self
                        .block_statement()
                        .ok_or_else(|| self.error(ExpectedToken::BlockStmt))
                        .flatten()?,
                })
            })
    }

    /// `defer_param -> "(" bind_pattern ")"`
    pub fn defer_param(&mut self) -> Option<Result<ast::DeferParam<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::LParen))
            .map(|open| {
                Ok(ast::DeferParam {
                    open,
                    param: self
                        .bind_pattern()
                        .ok_or_else(|| self.error(ExpectedToken::Pattern))?,
                    close: self
                        .try_pull_eq(
                            LexValue::Punctuation(Punctuation::RParen),
                            ExpectedToken::ExprOrRParen,
                        )
                        .map_err(|e| {
                            e.into_bracket_mismatch(Bracket::Paren, open.lex_range(self.source))
                        })?,
                })
            })
    }

    /// `if_statement -> "if" expression block_statement ( or_statement )?`
    pub fn if_statement(&mut self) -> Option<Result<IfStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::If))
            .map(|if_kw| {
                Ok(IfStmt {
                    if_kw,
                    pred: self.expression()?,
                    then: self
                        .block_statement()
                        .ok_or_else(|| self.error(ExpectedToken::BlockStmt))
                        .flatten()?,
                    or: self.or_statement().transpose()?.map(Box::new),
                })
            })
    }

    /// `or_statement -> "or" or_body`
    pub fn or_statement(&mut self) -> Option<Result<OrStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Or))
            .map(|or_kw| {
                Ok(OrStmt {
                    or_kw,
                    body: self.or_body()?,
                })
            })
    }

    /// `or_body -> if_statement | block_statement`
    pub fn or_body(&mut self) -> Result<OrBody<'src>, ContextError<'src>> {
        self.if_statement()
            .map(|res| res.map(OrBody::More))
            .or_else(|| self.block_statement().map(|res| res.map(OrBody::Last)))
            .ok_or_else(|| self.error(ExpectedToken::BlockOrIf))
            .flatten()
    }

    /// `loop_block -> ( scope_label )? loop_block1`
    pub fn loop_block(&mut self) -> Option<Result<ast::LoopBlock<'src>, ContextError<'src>>> {
        match self.scope_label().transpose() {
            Ok(label) => self.loop_block1(label),
            Err(e) => Some(Err(e)),
        }
    }

    /// `label -> "." IDENTIFIER`
    pub fn label(&mut self) -> Option<Result<ast::Label<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Dot))
            .map(|dot| {
                self.try_pull_eq(LexValue::Identifier, ExpectedToken::Ident)
                    .map(|name| ast::Label { dot, name })
            })
    }

    /// `scope_label -> label ":"`
    pub fn scope_label(&mut self) -> Option<Result<ast::ScopeLabel<'src>, ContextError<'src>>> {
        self.label().map(|label| {
            label.and_then(|label| {
                self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::Colon),
                    ExpectedToken::Colon,
                )
                .map(|colon| ast::ScopeLabel { label, colon })
            })
        })
    }

    /// `loop_block1 -> ( "loop" | "cord" | "for" bind_pattern "in" expression | "rep" expression ) block_statement`
    pub fn loop_block1(
        &mut self,
        label: Option<ast::ScopeLabel<'src>>,
    ) -> Option<Result<ast::LoopBlock<'src>, ContextError<'src>>> {
        let body = self
            .pull_if_eq(LexValue::Keyword(Keyword::Loop))
            .map(|kw| Ok((kw, ast::LoopKind::Loop)))
            .or_else(|| {
                self.pull_if_eq(LexValue::Keyword(Keyword::Cord))
                    .map(|kw| Ok((kw, ast::LoopKind::Cord)))
            })
            .or_else(|| {
                self.pull_if_eq(LexValue::Keyword(Keyword::For)).map(|kw| {
                    let bind = self
                        .bind_pattern()
                        .ok_or_else(|| self.error(ExpectedToken::Pattern))?;
                    let in_kw =
                        self.try_pull_eq(LexValue::Keyword(Keyword::In), ExpectedToken::InKeyword)?;
                    let iter = self.expression()?;
                    Ok((kw, ast::LoopKind::For { bind, in_kw, iter }))
                })
            })
            .or_else(|| {
                self.pull_if_eq(LexValue::Keyword(Keyword::Rep)).map(|kw| {
                    let pred = self.expression()?;
                    Ok((kw, ast::LoopKind::Rep { pred }))
                })
            });
        Some(
            if label.is_some() {
                // loop is mandatory
                body.ok_or_else(|| self.error(ExpectedToken::LoopBlock))
                    .flatten()
            } else {
                // loop is optional
                body?
            }
            .and_then(|(kw, kind)| {
                self.block_statement()
                    .ok_or_else(|| self.error(ExpectedToken::BlockStmt))
                    .flatten()
                    .map(|block| ast::LoopBlock {
                        label,
                        kw,
                        kind,
                        block,
                    })
            }),
        )
    }

    /// `done_statement -> "done" ( label )? expression ";"`
    pub fn done_statement(&mut self) -> Option<Result<ast::DoneStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Done)).map(|kw| {
            let label = self.label().transpose()?;
            self.pull_if_eq(LexValue::Punctuation(Punctuation::Semi))
                .map_or_else(
                    || {
                        self.expression().and_then(|value| {
                            self.try_pull_eq(
                                LexValue::Punctuation(Punctuation::Semi),
                                ExpectedToken::Semi,
                            )
                            .map(|semi| (Some(value), semi))
                        })
                    },
                    |semi| Ok((None, semi)),
                )
                .map(|(value, semi)| ast::DoneStmt {
                    kw,
                    label,
                    value,
                    semi,
                })
        })
    }

    /// `skip_statement -> "skip" ( label )? ";"`
    pub fn skip_statement(&mut self) -> Option<Result<ast::SkipStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Skip))
            .map(|skip_kw| {
                Ok(ast::SkipStmt {
                    kw: skip_kw,
                    label: self.label().transpose()?,
                    semi: self.try_pull_eq(
                        LexValue::Punctuation(Punctuation::Semi),
                        ExpectedToken::Semi,
                    )?,
                })
            })
    }

    /// `block_statement -> "{" ( declaration )* "}"`
    pub fn block_statement(&mut self) -> Option<Result<BlockStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::LBrace))
            .map(|open| {
                let mut body = Vec::new();
                let close = loop {
                    if let Some(close) = self.pull_if_eq(LexValue::Punctuation(Punctuation::RBrace))
                    {
                        break close;
                    }
                    body.push(self.declaration()?);
                };
                Ok(BlockStmt { open, body, close })
            })
    }

    /// `assign -> expression ( "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | "<<=" | ">>=" | "[<<]=" | "[>>]=" | "|=" | "&=" | "^=" | "!|=" | "!&=" | "!^=" ) expression | expression`
    fn assign(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let lhs = self.expression()?; // we'll check if this can be assigned to in static analysis
        if let Some(op) = self.pull_into_if_in(&[
            Assign, AddAssign, SubAssign, MulAssign, DivAssign, RemAssign, PowAssign, ShlAssign,
            ShrAssign, RotlAssign, RotrAssign, OrAssign, AndAssign, XorAssign, NorAssign,
            NandAssign, XnorAssign,
        ]) {
            let rhs = self.expression()?;
            Ok(ast::Expr::binary(ast::Binary { lhs, op, rhs }))
        } else {
            Ok(lhs)
        }
    }

    /// `expression -> if_block | loop_block | block_statement | or`
    pub fn expression(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        self.if_statement()
            .map(|res| res.map(BlockExpr::if_expr))
            .or_else(|| self.loop_block().map(|res| res.map(BlockExpr::loop_expr)))
            .or_else(|| self.block_statement().map(|res| res.map(BlockExpr::block)))
            .map(|res| res.map(ast::Expr::Block))
            .unwrap_or_else(|| self.or())
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.xor()?;
        while let Some(op) = self.pull_into_if_in(&[Or, Nor]) {
            let rhs = self.xor()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `xor -> and ( ("^" | "!^") and )*`
    fn xor(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.and()?;
        while let Some(op) = self.pull_into_if_in(&[Xor, Xnor]) {
            let rhs = self.and()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.equality()?;
        while let Some(op) = self.pull_into_if_in(&[And, Nand]) {
            let rhs = self.equality()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.comparison()?;
        while let Some(op) = self.pull_into_if_in(&[Ne, Eq]) {
            let rhs = self.comparison()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.shift()?;
        while let Some(op) = self.pull_into_if_in(&[Gt, Ge, Lt, Le]) {
            let rhs = self.shift()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.term()?;
        while let Some(op) = self.pull_into_if_in(&[Shl, Shr, Rotl, Rotr]) {
            let rhs = self.term()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.factor()?;
        while let Some(op) = self.pull_into_if_in(&[Add, SubNeg]) {
            let rhs = self.factor()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `factor -> unary_prefix ( ( "*" | "/" | "%" ) unary_prefix )*`
    fn factor(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_prefix()?;
        while let Some(op) = self.pull_into_if_in(&[Mul, Div, Rem]) {
            let rhs = self.unary_prefix()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    // TODO: `szof uint 3 ** 2` shouldn't be valid syntax
    /// `unary_prefix -> ( ( ( "!" | "!!" | "-" ) | "szof" ( type_expression )? ) )* exponent`
    fn unary_prefix(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        if let Some(op) = self.pull_into_if_in(&[Szof]) {
            let operand = self
                .type_expression() // TODO: combo error?
                .map(ast::Expr::type_expr)
                .or_else(|_| self.unary_prefix())?;
            Ok(ast::Expr::unary(ast::Unary {
                op,
                operand,
                side: OpSide::Prefix,
            }))
        } else if let Some(op) = self.pull_into_if_in(&[Not, Exists, SubNeg]) {
            let operand = self.unary_prefix()?;
            Ok(ast::Expr::unary(ast::Unary {
                op,
                operand,
                side: OpSide::Prefix,
            }))
        } else {
            self.exponent()
        }
    }

    /// `exponent -> conversion ( "**" conversion )*`
    fn exponent(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.conversion()?;
        while let Some(op) = self.pull_into_if_in(&[Pow]) {
            let rhs = self.conversion()?;
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `conversion -> unary_postfix ( "-:>" | "=:>" ) type_expression`
    fn conversion(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_postfix()?;
        while let Some(op) = self.pull_into_if_in(&[Convert, Transmute]) {
            let rhs = ast::Expr::type_expr(self.type_expression()?);
            expr = ast::Expr::binary(ast::Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `unary_postfix -> primary ( "?" | "++" | "--" )*`
    fn unary_postfix(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        let mut expr = self.primary()?;
        while let Some(op) = self.pull_into_if_in(&[Coalesce, Incr, Decr]) {
            expr = ast::Expr::unary(ast::Unary {
                operand: expr,
                op,
                side: OpSide::Postfix,
            });
        }
        Ok(expr)
    }

    /// `primary -> literal | group | variable | type_expression`
    ///
    /// `type_expression`/`type_bound` are included as a variant of `primary` in order
    /// to allow type errors instead of just "wrong token" errors.
    fn primary(&mut self) -> Result<ast::Expr<'src>, ContextError<'src>> {
        self.literal()
            .map(|literal| Ok(ast::Expr::literal(literal)))
            .or_else(|| self.group().map(|res| res.map(ast::Expr::grouping)))
            .or_else(|| self.directive().map(|var| Ok(ast::Expr::directive(var))))
            .or_else(|| self.variable().map(|var| Ok(ast::Expr::variable(var))))
            .or_else(|| {
                self.type_expression()
                    .map(ast::Expr::type_expr)
                    .ok()
                    .map(Ok)
            })
            .ok_or_else(|| self.error(ExpectedToken::Expr))
            .flatten()
    }

    /// `directive -> "inpt"`
    fn directive(&mut self) -> Option<ast::Directive<'src>> {
        self.pull_into::<ast::DirectiveKw>()
            .map(|kw| ast::Directive { kw })
    }

    /// `pattern -> IDENTIFIER`
    fn bind_pattern(&mut self) -> Option<ast::BindingPattern<'src>> {
        self.pull_if_eq(LexValue::Identifier)
            .map(|ident| ast::BindingPattern { ident })
    }

    /// `variable -> IDENTIFIER`
    fn variable(&mut self) -> Option<ast::Variable<'src>> {
        self.pull_if_eq(LexValue::Identifier)
            .map(|ident| ast::Variable { ident })
    }

    /// `literal -> "true" | "fals" | "none" | BYTE | UINT | SINT | FLPT | CHAR | TEXT | MACRO`
    fn literal(&mut self) -> Option<ast::Literal<'src>> {
        self.pull_if(match_token!(
            BoolLiteral(_)
                | Keyword::None
                | ByteLiteral(_)
                | UIntLiteral(_)
                | SIntLiteral(_)
                | FlptLiteral(_)
                | CharLiteral(_)
                | TextLiteral(_)
                | Macro
        ))
        .map(|value| ast::Literal { value })
    }

    /// `sequence -> "[" expression "]"`
    fn sequence(&mut self) -> Option<Result<ast::Grouping<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::LBrack))
            .map(|open| {
                let expr = self.expression()?;
                self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::RBrack),
                    ExpectedToken::ExprOrRBrack,
                )
                .map(move |close| ast::Grouping { open, expr, close })
                .map_err(|e| e.into_bracket_mismatch(Bracket::Brack, open.lex_range(self.source)))
            })
    }

    /// `group -> "(" expression ")"`
    fn group(&mut self) -> Option<Result<ast::Grouping<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::LParen))
            .map(|open| {
                Ok(ast::Grouping {
                    open,
                    expr: self.expression()?,
                    close: self
                        .try_pull_eq(
                            LexValue::Punctuation(Punctuation::RParen),
                            ExpectedToken::ExprOrRParen,
                        )
                        .map_err(|e| {
                            e.into_bracket_mismatch(Bracket::Paren, open.lex_range(self.source))
                        })?,
                })
            })
    }

    /// `generic_group -> "<" ( type_expression ( "," type_expression )* ( "," )? )? ">"`
    fn generic_group(&mut self) -> Option<Result<ast::GenericGroup<'src>, ContextError<'src>>> {
        // TBD: should `<<` be allowed? can `<...>` be used without a generic type?
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Lt))
            .map(|open| {
                let mut list = Vec::new();
                let close = loop {
                    if let Some(close) = self.rip_rangle() {
                        break close;
                    }
                    let item = self.type_expression()?;
                    let comma = self.pull_if_eq(LexValue::Punctuation(Punctuation::Comma));
                    list.push((item, comma));
                    if comma.is_none() {
                        break self
                            .rip_rangle()
                            .ok_or_else(|| self.error(ExpectedToken::TypeOrRAngle))
                            .map_err(|e| {
                                e.into_bracket_mismatch(Bracket::Paren, open.lex_range(self.source))
                            })?;
                    }
                };
                Ok(ast::GenericGroup { open, list, close })
            })
    }

    /// `type_expression -> ( ( "nevr" | "bool" | "uint" | "sint" | "flpt" | "char" | "text" | "fail" | IDENTIFIER ) "::" )*
    ///     ( "nevr" | "bool" | "uint" | "sint" | "flpt" | "char" | "text" | "fail" | IDENTIFIER )
    ///     ( generic_group )?
    ///     ( "|" ( "fail" | "none" | "nevr" ) )?`
    fn type_expression(&mut self) -> Result<ast::TypeExpr<'src>, ContextError<'src>> {
        let mut path = Vec::new();
        let mut name = self.try_pull_into(ExpectedToken::TypeExpr)?;
        while let Some(sep) = self.pull_if_eq(LexValue::Punctuation(Punctuation::PathSep)) {
            let next = self.try_pull_into(ExpectedToken::TypeExpr)?;
            let prev = std::mem::replace(&mut name, next);
            path.push(ast::NamespacePathPart {
                name: prev.lex,
                sep,
            });
        }
        let or_ty = self
            .pull_if_eq(LexValue::Punctuation(Punctuation::Or))
            .map(|pipe| {
                Ok(ast::OrType {
                    pipe,
                    ty: self.try_pull_into(ExpectedToken::OrType)?,
                })
            });
        Ok(ast::TypeExpr {
            name: ast::Typename {
                path: Vec::new(), // TODO
                name,
                args: None, // TODO
            },
            or_ty: or_ty.transpose()?,
        })
    }

    /// `iff_clause -> "<=>" type_bound`
    fn iff_clause(&mut self) -> Option<Result<ast::IffClause<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Iff))
            .map(|sym| {
                Ok(ast::IffClause {
                    iff: sym,
                    bound: self.type_bound()?,
                })
            })
    }

    /// `type_bound -> exact_bound | member_bound | shape_bound | expr_bound | grouped_bound | or_bound`
    fn type_bound(&mut self) -> Result<ast::TypeBound<'src>, ContextError<'src>> {
        self.member_bound()
            .map(|res| res.map(ast::TypeBound::has))
            .or_else(|| self.exact_bound().map(|res| res.map(ast::TypeBound::exact)))
            .or_else(|| self.shape_bound().map(|res| res.map(ast::TypeBound::shape)))
            .or_else(|| {
                self.expr_bound()
                    .map(|res| res.map(ast::TypeBound::satisfies))
            })
            .or_else(|| {
                self.grouped_bound()
                    .map(|res| res.map(ast::TypeBound::group))
            })
            .unwrap_or_else(|| self.or_bound())
    }

    /// `exact_bound -> "only" type_expression`
    fn exact_bound(&mut self) -> Option<Result<ast::ExactTypeBound<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Only)).map(|kw| {
            Ok(ast::ExactTypeBound {
                kw,
                ty: self.type_expression()?,
            })
        })
    }

    /// `member_bound -> "has" IDENTIFIER ( colon_bound )?`
    fn member_bound(&mut self) -> Option<Result<ast::MemberBound<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Has)).map(|kw| {
            Ok(ast::MemberBound {
                kw,
                member: self.try_pull_eq(LexValue::Identifier, ExpectedToken::Ident)?,
                req: self.colon_bound().transpose()?,
            })
        })
    }

    /// `type_bound -> ":" type_bound`
    fn colon_bound(&mut self) -> Option<Result<ast::ColonBound<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Colon))
            .map(|colon| {
                Ok(ast::ColonBound {
                    colon,
                    bound: self.type_bound()?,
                })
            })
    }

    /// `shape_bound -> "fun" "(" ( type_expression ( "," type_expression )* ( "," )? )? ")" "->" type_expression`
    fn shape_bound(&mut self) -> Option<Result<ast::ShapeBound<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Fun)).map(|kw| {
            let open = self.try_pull_eq(
                LexValue::Punctuation(Punctuation::LParen),
                ExpectedToken::LParen,
            )?;

            let mut args = Vec::new();
            let close = loop {
                if let Some(close) = self.pull_if_eq(LexValue::Punctuation(Punctuation::RParen)) {
                    break close;
                }
                let item = self.type_expression()?;
                let comma = self.pull_if_eq(LexValue::Punctuation(Punctuation::Comma));
                args.push((item, comma));
                if comma.is_none() {
                    break self
                        .try_pull_eq(
                            LexValue::Punctuation(Punctuation::RParen),
                            ExpectedToken::BoundOrRParen,
                        )
                        .map_err(|e| {
                            e.into_bracket_mismatch(Bracket::Paren, open.lex_range(self.source))
                        })?;
                }
            };

            Ok(ast::ShapeBound {
                kw,
                open,
                args,
                close,
                // TBD: optional?
                arrow: self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::Arrow),
                    ExpectedToken::Arrow,
                )?,
                ret: self.type_expression()?,
            })
        })
    }

    /// `expr_bound -> "for" IDENTIFIER ":" expression`
    fn expr_bound(&mut self) -> Option<Result<ast::ExprBound<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::For)).map(|kw| {
            Ok(ast::ExprBound {
                kw,
                name: self.try_pull_eq(LexValue::Identifier, ExpectedToken::Ident)?,
                colon: self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::Colon),
                    ExpectedToken::Colon,
                )?,
                expr: self.expression()?,
            })
        })
    }

    /// `grouped_bound -> "(" type_bound ")"`
    fn grouped_bound(&mut self) -> Option<Result<ast::GroupedBound<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::LParen))
            .map(|open| {
                Ok(ast::GroupedBound {
                    open,
                    inner: self.type_bound()?,
                    close: self
                        .try_pull_eq(
                            LexValue::Punctuation(Punctuation::RParen),
                            ExpectedToken::BoundOrRParen,
                        )
                        .map_err(|e| {
                            e.into_bracket_mismatch(Bracket::Paren, open.lex_range(self.source))
                        })?,
                })
            })
    }

    /// `or_bound -> xor_bound ( "|" xor_bound )*`
    fn or_bound(&mut self) -> Result<ast::TypeBound<'src>, ContextError<'src>> {
        let mut lhs = self.xor_bound()?;
        while let Some(sep) = self.pull_into_if_in(&[Combinator::Any]) {
            let rhs = self.xor_bound()?;
            lhs = ast::TypeBound::combo(ast::ComboBound { lhs, sep, rhs });
        }
        Ok(lhs)
    }

    /// `xor_bound -> and_bound ( "/" and_bound )*`
    fn xor_bound(&mut self) -> Result<ast::TypeBound<'src>, ContextError<'src>> {
        let mut lhs = self.or_bound()?;
        while let Some(sep) = self.pull_into_if_in(&[Combinator::One]) {
            let rhs = self.or_bound()?;
            lhs = ast::TypeBound::combo(ast::ComboBound { lhs, sep, rhs });
        }
        Ok(lhs)
    }

    /// `and_bound -> not_bound ( "+" not_bound )*`
    fn and_bound(&mut self) -> Result<ast::TypeBound<'src>, ContextError<'src>> {
        let mut lhs = self.not_bound()?;
        while let Some(sep) = self.pull_into_if_in(&[Combinator::All]) {
            let rhs = self.not_bound()?;
            lhs = ast::TypeBound::combo(ast::ComboBound { lhs, sep, rhs });
        }
        Ok(lhs)
    }

    /// `not_bound -> ( "!" )* type_bound`
    fn not_bound(&mut self) -> Result<ast::TypeBound<'src>, ContextError<'src>> {
        if let Some(not) = self.pull_if_eq(LexValue::Punctuation(Punctuation::Not)) {
            let rhs = self.not_bound()?;
            Ok(ast::TypeBound::not(ast::NegativeBound { not, rhs }))
        } else {
            self.type_bound()
        }
    }
}
