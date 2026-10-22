//! Context-free grammar

use crate::{
    error::{ContextError, ExpectedToken, OpSide},
    grammar::ast::{
        Assignment, BlockStmt, DeclVar, Declaration, ExprStmt, HaltStmt, IfStmt, OrBody, OrStmt,
        PrintStmt, Program, Statement, Variable,
    },
    scanner::{
        Bracket,
        token::{
            Lexeme, Lexical, SpecToken, Token,
            punc::Punctuation,
            value::{Keyword, LexValue, Operator},
        },
    },
};
use Operator::*;
use ast::{
    Binary, BindingPattern, Directive, DirectiveKw, Expr, Grouping, Label, Literal, LoopBlock,
    LoopKind, OrType, ScopeLabel, SkipStmt, TypeAnnotation, TypeExpr, Unary,
};

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
    type Item = Result<Expr<'src>, ContextError<'src>>;

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
                Or | Pick | Rep | For | Loop | Cord | Halt | Skip | Give | Fail | Emit
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

    /// `declaration -> var_declaration | statement`
    pub fn declaration(&mut self) -> Result<Declaration<'src>, ContextError<'src>> {
        self.var_declaration()
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
    pub fn var_declaration(&mut self) -> Option<Result<Declaration<'src>, ContextError<'src>>> {
        self.pull_into().map(|var_kw| {
            Ok(Declaration::var(DeclVar {
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
            }))
        })
    }

    /// `type_annotation -> ":" type_expression`
    pub fn type_annotation(&mut self) -> Option<Result<TypeAnnotation<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Colon))
            .map(|colon| {
                Ok(TypeAnnotation {
                    colon,
                    ty: self.type_expression()?,
                })
            })
    }

    /// `statement -> block_statement | print_statement | halt_statement | skip_statement | if_statement | loop_statement | expression_statement`
    pub fn statement(&mut self) -> Result<Statement<'src>, ContextError<'src>> {
        self.block_statement()
            .map(|res| res.map(Statement::Block))
            .or_else(|| self.print_statement().map(|res| res.map(Statement::Print)))
            .or_else(|| self.halt_statement().map(|res| res.map(Statement::Halt)))
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
    pub fn loop_block(&mut self) -> Option<Result<LoopBlock<'src>, ContextError<'src>>> {
        match self.scope_label().transpose() {
            Ok(label) => self.loop_block1(label),
            Err(e) => Some(Err(e)),
        }
    }

    /// `label -> "." IDENTIFIER`
    pub fn label(&mut self) -> Option<Result<Label<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::Dot))
            .map(|dot| {
                self.try_pull_eq(LexValue::Identifier, ExpectedToken::Ident)
                    .map(|name| Label { dot, name })
            })
    }

    /// `scope_label -> label ":"`
    pub fn scope_label(&mut self) -> Option<Result<ScopeLabel<'src>, ContextError<'src>>> {
        self.label().map(|label| {
            label.and_then(|label| {
                self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::Colon),
                    ExpectedToken::Colon,
                )
                .map(|colon| ScopeLabel { label, colon })
            })
        })
    }

    /// `loop_block1 -> ( "loop" | "cord" | "for" bind_pattern "in" expression | "rep" expression ) block_statement`
    pub fn loop_block1(
        &mut self,
        label: Option<ScopeLabel<'src>>,
    ) -> Option<Result<LoopBlock<'src>, ContextError<'src>>> {
        let body = self
            .pull_if_eq(LexValue::Keyword(Keyword::Loop))
            .map(|kw| Ok((kw, LoopKind::Loop)))
            .or_else(|| {
                self.pull_if_eq(LexValue::Keyword(Keyword::Cord))
                    .map(|kw| Ok((kw, LoopKind::Cord)))
            })
            .or_else(|| {
                self.pull_if_eq(LexValue::Keyword(Keyword::For)).map(|kw| {
                    let bind = self
                        .bind_pattern()
                        .ok_or_else(|| self.error(ExpectedToken::Pattern))?;
                    let in_kw =
                        self.try_pull_eq(LexValue::Keyword(Keyword::In), ExpectedToken::InKeyword)?;
                    let iter = self.expression()?;
                    Ok((kw, LoopKind::For { bind, in_kw, iter }))
                })
            })
            .or_else(|| {
                self.pull_if_eq(LexValue::Keyword(Keyword::Rep)).map(|kw| {
                    let pred = self.expression()?;
                    Ok((kw, LoopKind::Rep { pred }))
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
                    .map(|block| LoopBlock {
                        label,
                        kw,
                        kind,
                        block,
                    })
            }),
        )
    }

    /// `halt_statement -> "halt" ( label )? ";"`
    pub fn halt_statement(&mut self) -> Option<Result<HaltStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Halt))
            .map(|halt_kw| {
                Ok(HaltStmt {
                    halt_kw,
                    label: self.label().transpose()?,
                    semi: self.try_pull_eq(
                        LexValue::Punctuation(Punctuation::Semi),
                        ExpectedToken::Semi,
                    )?,
                })
            })
    }

    /// `skip_statement -> "skip" ( label )? ";"`
    pub fn skip_statement(&mut self) -> Option<Result<SkipStmt<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Keyword(Keyword::Skip))
            .map(|skip_kw| {
                Ok(SkipStmt {
                    skip_kw,
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
    fn assign(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let lhs = self.expression()?; // we'll check if this can be assigned to in static analysis
        if let Some(op) = self.pull_into_if_in(&[
            Assign, AddAssign, SubAssign, MulAssign, DivAssign, RemAssign, PowAssign, ShlAssign,
            ShrAssign, RotlAssign, RotrAssign, OrAssign, AndAssign, XorAssign, NorAssign,
            NandAssign, XnorAssign,
        ]) {
            let rhs = self.expression()?;
            Ok(Expr::binary(Binary { lhs, op, rhs }))
        } else {
            Ok(lhs)
        }
    }

    /// `expression -> or`
    pub fn expression(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.or()
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.xor()?;
        while let Some(op) = self.pull_into_if_in(&[Or, Nor]) {
            let rhs = self.xor()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `xor -> and ( ("^" | "!^") and )*`
    fn xor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.and()?;
        while let Some(op) = self.pull_into_if_in(&[Xor, Xnor]) {
            let rhs = self.and()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.equality()?;
        while let Some(op) = self.pull_into_if_in(&[And, Nand]) {
            let rhs = self.equality()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.comparison()?;
        while let Some(op) = self.pull_into_if_in(&[Ne, Eq]) {
            let rhs = self.comparison()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.shift()?;
        while let Some(op) = self.pull_into_if_in(&[Gt, Ge, Lt, Le]) {
            let rhs = self.shift()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.term()?;
        while let Some(op) = self.pull_into_if_in(&[Shl, Shr, Rotl, Rotr]) {
            let rhs = self.term()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.factor()?;
        while let Some(op) = self.pull_into_if_in(&[Add, SubNeg]) {
            let rhs = self.factor()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `factor -> unary_prefix ( ( "*" | "/" | "%" ) unary_prefix )*`
    fn factor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_prefix()?;
        while let Some(op) = self.pull_into_if_in(&[Mul, Div, Rem]) {
            let rhs = self.unary_prefix()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    // TODO: `szof uint 3 ** 2` shouldn't be valid syntax
    /// `unary_prefix -> ( ( ( "!" | "!!" | "-" ) | "szof" ( type_expression )? ) )* exponent`
    fn unary_prefix(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(op) = self.pull_into_if_in(&[Szof]) {
            let operand = self
                .type_expression() // TODO: combo error?
                .map(Expr::type_expr)
                .or_else(|_| self.unary_prefix())?;
            Ok(Expr::unary(Unary {
                op,
                operand,
                side: OpSide::Prefix,
            }))
        } else if let Some(op) = self.pull_into_if_in(&[Not, Exists, SubNeg]) {
            let operand = self.unary_prefix()?;
            Ok(Expr::unary(Unary {
                op,
                operand,
                side: OpSide::Prefix,
            }))
        } else {
            self.exponent()
        }
    }

    /// `exponent -> conversion ( "**" conversion )*`
    fn exponent(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.conversion()?;
        while let Some(op) = self.pull_into_if_in(&[Pow]) {
            let rhs = self.conversion()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `conversion -> unary_postfix ( "-:>" | "=:>" ) type_expression`
    fn conversion(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_postfix()?;
        while let Some(op) = self.pull_into_if_in(&[Convert, Transmute]) {
            let rhs = Expr::type_expr(self.type_expression()?);
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `unary_postfix -> primary ( "?" | "++" | "--" )*`
    fn unary_postfix(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.primary()?;
        while let Some(op) = self.pull_into_if_in(&[Coalesce, Incr, Decr]) {
            expr = Expr::unary(Unary {
                operand: expr,
                op,
                side: OpSide::Postfix,
            });
        }
        Ok(expr)
    }

    /// `primary -> literal | group | variable | type_expression`
    ///
    /// `type_expression` is included as a variant of `primary` in order
    /// to allow type errors instead of just "wrong token" errors.
    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.literal()
            .map(|literal| Ok(Expr::literal(literal)))
            .or_else(|| self.group().map(|res| res.map(Expr::grouping)))
            .or_else(|| self.directive().map(|var| Ok(Expr::directive(var))))
            .or_else(|| self.variable().map(|var| Ok(Expr::variable(var))))
            .or_else(|| self.type_expression().map(Expr::type_expr).ok().map(Ok))
            .ok_or_else(|| self.error(ExpectedToken::Expr))
            .flatten()
    }

    /// `directive -> "inpt"`
    fn directive(&mut self) -> Option<Directive<'src>> {
        self.pull_into::<DirectiveKw>().map(|kw| Directive { kw })
    }

    /// `pattern -> IDENTIFIER`
    fn bind_pattern(&mut self) -> Option<BindingPattern<'src>> {
        self.pull_if_eq(LexValue::Identifier)
            .map(|ident| BindingPattern { ident })
    }

    /// `variable -> IDENTIFIER`
    fn variable(&mut self) -> Option<Variable<'src>> {
        self.pull_if_eq(LexValue::Identifier)
            .map(|ident| Variable { ident })
    }

    /// `literal -> "true" | "fals" | "none" | UINT | SINT | FLPT | CHAR | TEXT | MACRO`
    fn literal(&mut self) -> Option<Literal<'src>> {
        self.pull_if(match_token!(
            BoolLiteral(_)
                | Keyword::None
                | UIntLiteral(_)
                | SIntLiteral(_)
                | FlptLiteral(_)
                | CharLiteral(_)
                | TextLiteral(_)
                | Macro
        ))
        .map(|value| Literal { value })
    }

    /// `sequence -> "[" expression "]"`
    fn sequence(&mut self) -> Option<Result<Grouping<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::LBrack))
            .map(|open| {
                let expr = self.expression()?;
                self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::RBrack),
                    ExpectedToken::ExprOrRBrack,
                )
                .map(move |close| Grouping { open, expr, close })
                .map_err(|e| e.into_bracket_mismatch(Bracket::Brack, open.lex_range(self.source)))
            })
    }

    /// `group -> "(" expression ")"`
    fn group(&mut self) -> Option<Result<Grouping<'src>, ContextError<'src>>> {
        self.pull_if_eq(LexValue::Punctuation(Punctuation::LParen))
            .map(|open| {
                let expr = self.expression()?;
                self.try_pull_eq(
                    LexValue::Punctuation(Punctuation::RParen),
                    ExpectedToken::ExprOrRParen,
                )
                .map(move |close| Grouping { open, expr, close })
                .map_err(|e| e.into_bracket_mismatch(Bracket::Paren, open.lex_range(self.source)))
            })
    }

    /// `type_expression -> ( "nevr" | "bool" | "uint" | "sint" | "flpt" | "char" | "text" | "fail" | IDENTIFIER ) ( "|" ( "fail" | "none" | "nevr" ) )?`
    fn type_expression(&mut self) -> Result<TypeExpr<'src>, ContextError<'src>> {
        let name = self.try_pull_into(ExpectedToken::TypeExpr)?;
        let or_ty = self
            .pull_if_eq(LexValue::Punctuation(Punctuation::Or))
            .map(|pipe| {
                Ok(OrType {
                    pipe,
                    ty: self.try_pull_into(ExpectedToken::OrType)?,
                })
            });
        Ok(TypeExpr {
            name,
            or_ty: or_ty.transpose()?,
        })
    }
}
