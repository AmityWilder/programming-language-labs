use crate::{
    error::{Error, ErrorKind, ExpectedToken},
    lex::token::{
        Operator, TokenKind,
        keyword::Keyword,
        punc::{BadBracketCombo, Bracket, Punctuation},
    },
    src_code::span::Span,
};
use ast::{Expectable, ExpectableExt};
use std::iter::{Filter, Peekable};

pub mod ast;

pub mod util {
    pub trait MapInner<T, E>: Sized {
        fn map_inner<U, F>(self, f: F) -> Option<Result<U, E>>
        where
            F: FnOnce(T) -> U;
    }

    impl<T, E> MapInner<T, E> for Option<Result<T, E>> {
        fn map_inner<U, F>(self, f: F) -> Option<Result<U, E>>
        where
            F: FnOnce(T) -> U,
        {
            match self {
                Some(Ok(t)) => Some(Ok(f(t))),
                Some(Err(e)) => Some(Err(e)),
                None => None,
            }
        }
    }
}
use util::MapInner;

pub type Result<T> = std::result::Result<T, Error>;

pub trait ParserExpect {
    type Item;

    fn ok_or_unexpected<I>(self, parser: &mut Parser<I>) -> Result<Self::Item>
    where
        I: Iterator<Item = (Span, TokenKind)>;
}

impl<T: ExpectableExt> ParserExpect for T {
    type Item = T::Item;

    fn ok_or_unexpected<I>(self, parser: &mut Parser<I>) -> Result<Self::Item>
    where
        I: Iterator<Item = (Span, TokenKind)>,
    {
        self.ok_or_expected().map_err(|e| parser.error(e))
    }
}

impl<T: Expectable> ParserExpect for Option<Result<T>> {
    type Item = T;

    fn ok_or_unexpected<I>(self, parser: &mut Parser<I>) -> Result<T>
    where
        I: Iterator<Item = (Span, TokenKind)>,
    {
        self.ok_or(T::EXPECTED)
            .map_err(|e| parser.error(e))
            .flatten()
    }
}

#[derive(Debug, Clone)]
pub struct Parser<I: Iterator> {
    tokens: Peekable<Filter<I, fn(&I::Item) -> bool>>,
    eof: Span,
}

impl<I: Iterator<Item = (Span, TokenKind)>> Parser<I> {
    pub fn new<J>(tokens: J, eof: Span) -> Self
    where
        J: IntoIterator<IntoIter = I>,
    {
        let filter: fn(&I::Item) -> bool = |&(_, kind)| {
            !matches!(
                kind,
                TokenKind::Newline | TokenKind::WSpace | TokenKind::Comment
            )
        };
        Self {
            tokens: tokens.into_iter().filter(filter).peekable(),
            eof,
        }
    }

    fn peek(&mut self) -> Option<&I::Item> {
        self.tokens.peek()
    }

    fn is_next<P>(&mut self, p: P) -> bool
    where
        P: FnOnce(&I::Item) -> bool,
    {
        self.peek().is_some_and(p)
    }

    fn is_next_eq<Q>(&mut self, kind: Q) -> bool
    where
        Q: Into<TokenKind>,
    {
        self.is_next(|(_, k)| *k == kind.into())
    }

    /// Returns `true` if the next token is `)`, `]`, or `}`.
    ///
    /// See [`Self::is_next_angle_close`] if you are specifically expecting an angle bracket (`>`).
    fn is_next_close(&mut self) -> bool {
        self.is_next(|(_, k)| {
            matches!(
                k,
                TokenKind::Punc(Punctuation::RBrace | Punctuation::RBrack | Punctuation::RParen)
            )
        })
    }

    /// Similar to [`Self::is_next_close`], but **also** returns `true` for `>`.
    fn is_next_angle_close(&mut self) -> bool {
        self.is_next(|(_, k)| {
            matches!(
                k,
                TokenKind::Punc(
                    Punctuation::RBrace
                        | Punctuation::RBrack
                        | Punctuation::RParen
                        | Punctuation::Gt
                )
            )
        })
    }

    fn pull(&mut self) -> Option<I::Item> {
        self.tokens.next()
    }

    fn pull_if<P>(&mut self, p: P) -> Option<I::Item>
    where
        P: FnOnce(&I::Item) -> bool,
    {
        self.tokens.next_if(p)
    }

    fn pull_if_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(I::Item) -> Option<T>,
    {
        self.tokens.next_if_map(|item| f(item).ok_or(item))
    }

    fn pull_if_eq<Q>(&mut self, kind: Q) -> Option<Span>
    where
        Q: Into<TokenKind>,
    {
        // if the kind can only be one thing, we don't need to output it.
        self.pull_if(|(_, k)| *k == kind.into())
            .map(|(span, _)| span)
    }

    fn pull_into_if<T, P>(&mut self, p: P) -> Option<(Span, T)>
    where
        TokenKind: TryInto<T>,
        P: FnOnce(&T) -> bool,
    {
        self.pull_if_map(|(span, kind)| kind.try_into().ok().filter(p).map(|t| (span, t)))
    }

    fn pull_into<T>(&mut self) -> Option<(Span, T)>
    where
        TokenKind: TryInto<T>,
    {
        self.pull_into_if(|_| true)
    }

    fn try_pull<P>(&mut self, p: P, expected: ExpectedToken) -> Result<I::Item>
    where
        P: FnOnce(&I::Item) -> bool,
    {
        self.pull_if(p).ok_or_else(|| self.error(expected))
    }

    fn try_pull_map<T, F>(&mut self, f: F, expected: ExpectedToken) -> Result<T>
    where
        F: FnOnce(I::Item) -> Option<T>,
    {
        self.pull_if_map(f).ok_or_else(|| self.error(expected))
    }

    fn try_pull_eq<Q>(&mut self, kind: Q) -> Result<Span>
    where
        Q: Into<TokenKind>,
    {
        let kind = kind.into();
        // if the kind can only be one thing, we don't need to output it.
        self.pull_if_eq(kind)
            .ok_or_else(|| self.error(ExpectedToken::Verbatim(kind)))
    }

    fn try_pull_into_if<T, P>(&mut self, expected: ExpectedToken, p: P) -> Result<(Span, T)>
    where
        TokenKind: TryInto<T>,
        P: FnOnce(&T) -> bool,
    {
        self.try_pull_map(
            |(span, kind)| kind.try_into().ok().filter(p).map(|t| (span, t)),
            expected,
        )
    }

    fn try_pull_into<T>(&mut self, expected: ExpectedToken) -> Result<(Span, T)>
    where
        TokenKind: TryInto<T>,
    {
        self.try_pull_map(
            |(span, kind)| kind.try_into().ok().map(|t| (span, t)),
            expected,
        )
    }

    fn rip_rangle(&mut self) -> Option<Span> {
        #[expect(clippy::as_conversions, reason = "char.len_utf8() is at most 4")]
        const RANGLE_LEN: u32 = '>'.len_utf8() as u32;
        if let x @ Some(_) = self.pull_if_eq(Punctuation::Gt) {
            x
        } else if let Some((span, kind)) = self.tokens.peek_mut() {
            *kind = match kind {
                TokenKind::Punc(Punctuation::Shr) => TokenKind::Punc(Punctuation::Gt),
                TokenKind::Punc(Punctuation::ShrAssign) => TokenKind::Punc(Punctuation::Ge),
                _ => return None,
            };
            let start = span.rel.start;
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the span contains a '>' so because file length is not allowed to exceed u32::MAX, \
                the end position of that '>' cannot exceed u32::MAX."
            )]
            let mid = start + RANGLE_LEN;
            span.rel.start = mid;
            Some(Span {
                file: span.file,
                rel: (start..mid).into(),
            })
        } else {
            None
        }
    }

    // I know it's a big function, but `brack` should be a literal, so inlining should allow the compiler to cut down on some of the branches
    #[inline]
    fn pull_if_delim<R, F>(&mut self, brack: Bracket, inner: F) -> Option<Result<ast::Delimited<R>>>
    where
        F: FnOnce(&mut Self) -> Result<R>,
    {
        self.pull_if_eq(brack.open_punc()).map(|open| {
            let inner = inner(self)?;
            let close = if matches!(brack, Bracket::Angle) {
                self.rip_rangle()
            } else {
                self.pull_if_eq(brack.close_punc())
            }
            .ok_or_else(|| {
                if let Some(&(span, kind)) = self.peek() {
                    let kind = 'err_kind: {
                        let close = match kind {
                            TokenKind::Punc(Punctuation::RBrack) => Bracket::Brack,
                            TokenKind::Punc(Punctuation::RParen) => Bracket::Paren,
                            TokenKind::Punc(Punctuation::RBrace) => Bracket::Brace,
                            // '>' should not be reported as an unexpected close bracket, only an unexpected token.
                            _ => {
                                break 'err_kind ErrorKind::UnexpectedToken {
                                    expected: ExpectedToken::Verbatim(brack.close_punc().into()),
                                };
                            }
                        };
                        ErrorKind::IncorrectCloseBracket {
                            open_range: open,
                            failure: BadBracketCombo::new(brack, close)
                                .expect("matching brackets should not produce an error"),
                        }
                    };
                    Error::new(span, kind)
                } else {
                    Error::new(
                        self.eof,
                        ErrorKind::MissingCloseBracket {
                            open_range: open,
                            expect: brack,
                        },
                    )
                }
            })?;
            Ok(ast::Delimited { open, inner, close })
        })
    }

    /// If a trailing separator is **allowed**, `trailing` should be [`None`]. If a trailing separator is **not allowed**,
    /// `trailing` should be the error that occurs if a separator is not followed by another item.
    // I know it's a big function, but `trailing` should be a literal, so inlining should allow the compiler to cut down on some of the branches
    #[inline]
    fn pull_if_separated<Q, T, F>(
        &mut self,
        trailing: Option<ExpectedToken>,
        sep: Q,
        mut f: F,
    ) -> Result<Vec<T>>
    where
        Q: Into<TokenKind>,
        F: FnMut(&mut Self) -> Option<Result<T>>,
    {
        let sep = sep.into();
        let mut items = Vec::new();
        if let Some(first) = f(self) {
            items.push(first?);
            while self.pull_if_eq(sep).is_some() {
                let item = if let Some(expected) = trailing {
                    f(self).ok_or_else(|| self.error(expected)).flatten()
                } else if let Some(item) = f(self) {
                    item
                } else {
                    break;
                };
                items.push(item?);
            }
        }
        Ok(items)
    }

    /// Does not consume the next token
    fn error(&mut self, expected: ExpectedToken) -> Error {
        if let Some(&(span, _)) = self.peek() {
            Error::new(span, ErrorKind::UnexpectedToken { expected })
        } else {
            Error::new(self.eof, ErrorKind::MissingToken { expected })
        }
    }

    fn sync(&mut self) {
        use Keyword::{
            Alt, Cat, Cord, Def, Done, Emit, Fail, For, Fun, Give, If, Let, Loop, Mem, Or, Pick,
            Prnt, Pvt, Rec, Rep, Skip, Sub, Sup, Uni,
        };
        while let Some((_, kind)) = self.pull() {
            // end of current statement
            if kind == TokenKind::Punc(Punctuation::Semi) {
                break;
            }

            // start of new statement/definition
            if self.peek().is_some_and(|(_, kind)| {
                matches!(
                    kind,
                    TokenKind::Keyword(
                        Rec | Sup
                            | Cat
                            | Alt
                            | Sub
                            | Def
                            | Fun
                            | Mem
                            | Let
                            | Uni
                            | Pvt
                            | If
                            | Or
                            | Pick
                            | Rep
                            | For
                            | Loop
                            | Cord
                            | Done
                            | Skip
                            | Give
                            | Fail
                            | Emit
                            | Prnt
                    )
                )
            }) {
                break;
            }
        }
    }
}

pub fn parse<I>(tokens: I, eof: Span) -> std::result::Result<ast::Program, Vec<Error>>
where
    I: IntoIterator<Item = (Span, TokenKind)>,
{
    Parser::new(tokens, eof).program().map_err(Vec::from_iter)
}

impl<I: Iterator<Item = (Span, TokenKind)>> Parser<I> {
    /// `program -> ( statement )*`
    pub fn program(
        &mut self,
    ) -> std::result::Result<ast::Program, impl IntoIterator<Item = Error>> {
        let mut iter = std::iter::from_fn(|| {
            self.tokens
                .peek()
                .is_some()
                .then(|| self.declaration().inspect_err(|_| self.sync()))
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
        Ok(ast::Program { decls })
    }

    /// `declaration -> var_declaration | defer_declaration | statement`
    pub fn declaration(&mut self) -> Result<ast::Declaration> {
        self.var_declaration()
            .map_inner(ast::Declaration::Var)
            .or_else(|| self.defer_declaration().map_inner(ast::Declaration::Defer))
            .unwrap_or_else(|| self.statement().map(ast::Declaration::Stmt))
    }

    /// `assignment -> "=" expression`
    pub fn assignment(&mut self) -> Option<Result<ast::Assignment>> {
        self.pull_if_eq(Punctuation::Assign).map(|op| {
            Ok(ast::Assignment {
                op,
                expr: self.expression()?,
            })
        })
    }

    /// `var_declaration -> ( "uni" | "pvt" | "let" ) pattern ( type_annotation )? ( assignment )? ";"`
    pub fn var_declaration(&mut self) -> Option<Result<ast::DeclVar>> {
        self.pull_into().map(|(kw_span, kw)| {
            Ok(ast::DeclVar {
                kw,
                kw_span,
                binding: self.bind_pattern().ok_or_unexpected(self)?,
                ty: self.type_annotation().transpose()?,
                assignment: self.assignment().transpose()?,
                semi: self.try_pull_eq(TokenKind::Punc(Punctuation::Semi))?,
            })
        })
    }

    /// `type_annotation -> ":" type_expression`
    pub fn type_annotation(&mut self) -> Option<Result<ast::TypeAnnotation>> {
        self.pull_if_eq(Punctuation::Colon).map(|colon| {
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
    pub fn statement(&mut self) -> Result<ast::Statement> {
        todo!()
    }

    /// `expression_statement -> assign ";"`
    pub fn expression_statement(&mut self) -> Result<ast::ExprStmt> {
        Ok(ast::ExprStmt {
            expr: self.assign()?,
            semi: self.try_pull_eq(Punctuation::Semi)?,
        })
    }

    /// `print_statement -> "prnt" expression ";"`
    pub fn print_statement(&mut self) -> Option<Result<ast::PrintStmt>> {
        self.pull_if_eq(Keyword::Prnt).map(|kw| {
            Ok(ast::PrintStmt {
                kw,
                group: self.group().ok_or_unexpected(self)?,
                semi: self.try_pull_eq(Punctuation::Semi)?,
            })
        })
    }

    /// `defer_declaration -> "defer" ( defer_param )? block_statement`
    pub fn defer_declaration(&mut self) -> Option<Result<ast::DeclDefer>> {
        self.pull_if_eq(Keyword::Defer).map(|kw| {
            Ok(ast::DeclDefer {
                kw,
                param: self.defer_param().transpose()?,
                on_pop: self.block_statement().ok_or_unexpected(self)?,
            })
        })
    }

    /// `defer_param -> "(" bind_pattern ")"`
    pub fn defer_param(&mut self) -> Option<Result<ast::DeferParam>> {
        self.pull_if_delim(Bracket::Paren, |parser| {
            parser.bind_pattern().ok_or_unexpected(parser)
        })
        .map_inner(|x| ast::DeferParam {
            open: x.open,
            param: x.inner,
            close: x.close,
        })
    }

    /// `if_statement -> "if" expression block_statement ( or_statement )?`
    pub fn if_statement(&mut self) -> Option<Result<ast::IfStmt>> {
        self.pull_if_eq(Keyword::If).map(|kw| {
            Ok(ast::IfStmt {
                kw,
                pred: self.expression()?,
                then: self.block_statement().ok_or_unexpected(self)?,
                or: self.or_statement().transpose()?.map(Box::new),
            })
        })
    }

    /// `or_statement -> "or" or_body`
    pub fn or_statement(&mut self) -> Option<Result<ast::OrStmt>> {
        self.pull_if_eq(Keyword::Or).map(|kw| {
            Ok(ast::OrStmt {
                kw,
                body: self.or_body()?,
            })
        })
    }

    /// `or_body -> if_statement | block_statement`
    pub fn or_body(&mut self) -> Result<ast::OrBody> {
        self.if_statement()
            .map_inner(ast::OrBody::More)
            .or_else(|| self.block_statement().map_inner(ast::OrBody::Last))
            .ok_or_unexpected(self)
    }

    /// `loop_block -> ( scope_label )? loop_block1`
    pub fn loop_block(&mut self) -> Option<Result<ast::LoopBlock>> {
        // this may consume the label if there's no block...
        match self.scope_label().transpose() {
            Ok(lbl) => self.loop_block1(lbl),
            Err(e) => Some(Err(e)),
        }
    }

    /// `label -> "." IDENTIFIER`
    pub fn label(&mut self) -> Option<Result<ast::Label>> {
        self.pull_if_eq(Punctuation::Dot).map(|dot| {
            Ok(ast::Label {
                dot,
                name: self.try_pull_eq(TokenKind::Ident)?,
            })
        })
    }

    /// `scope_label -> label ":"`
    pub fn scope_label(&mut self) -> Option<Result<ast::ScopeLabel>> {
        self.label().map(|label| {
            Ok(ast::ScopeLabel {
                label: label?,
                colon: self.try_pull_eq(Punctuation::Colon)?,
            })
        })
    }

    /// `loop_block1 -> ( "loop" | "cord" | "for" bind_pattern "in" expression | "rep" expression ) block_statement`
    pub fn loop_block1(
        &mut self,
        label: Option<ast::ScopeLabel>,
    ) -> Option<Result<ast::LoopBlock>> {
        let res = self
            .pull_if_eq(Keyword::Loop)
            .map(|kw| Result::Ok((kw, ast::LoopKind::Loop)))
            .or_else(|| {
                self.pull_if_eq(Keyword::Cord)
                    .map(|kw| Ok((kw, ast::LoopKind::Cord)))
            })
            .or_else(|| {
                self.pull_if_eq(Keyword::For).map(|kw| {
                    Ok((
                        kw,
                        ast::LoopKind::For {
                            bind: self.bind_pattern().ok_or_unexpected(self)?,
                            in_kw: self.try_pull_eq(Keyword::In)?,
                            iter: self.expression()?,
                        },
                    ))
                })
            })
            .or_else(|| {
                self.pull_if_eq(Keyword::Rep).map(|kw| {
                    Ok((
                        kw,
                        ast::LoopKind::Rep {
                            pred: self.expression()?,
                        },
                    ))
                })
            });

        Some(
            if label.is_none() {
                res?
            } else {
                res.ok_or_else(|| self.error(ExpectedToken::LoopBlock))
                    .flatten()
            }
            .and_then(|(kw, kind)| {
                Ok(ast::LoopBlock {
                    label,
                    kw,
                    kind,
                    block: self.block_statement().ok_or_unexpected(self)?,
                })
            }),
        )
    }

    /// `done_statement -> "done" ( label )? expression ";"`
    pub fn done_statement(&mut self) -> Option<Result<ast::DoneStmt>> {
        self.pull_if_eq(Keyword::Done).map(|kw| {
            let label = self.label().transpose()?;
            let semi_upcoming = self.is_next_eq(Punctuation::Semi);
            Ok(ast::DoneStmt {
                kw,
                label,
                value: (!semi_upcoming).then(|| self.expression()).transpose()?,
                semi: self.try_pull_eq(Punctuation::Semi)?,
            })
        })
    }

    /// `skip_statement -> "skip" ( label )? ";"`
    pub fn skip_statement(&mut self) -> Option<Result<ast::SkipStmt>> {
        self.pull_if_eq(Keyword::Skip).map(|kw| {
            Ok(ast::SkipStmt {
                kw,
                label: self.label().transpose()?,
                semi: self.try_pull_eq(Punctuation::Semi)?,
            })
        })
    }

    /// `block_statement -> "{" ( declaration )* "}"`
    pub fn block_statement(&mut self) -> Option<Result<ast::BlockStmt>> {
        self.pull_if_delim(Bracket::Brace, |parser| {
            std::iter::from_fn(|| parser.is_next_close().then(|| parser.declaration())).collect()
        })
        .map_inner(|x| ast::BlockStmt {
            open: x.open,
            body: x.inner,
            close: x.close,
        })
    }

    /// `assign -> expression (
    ///       "="
    ///     | "+="
    ///     | "-="
    ///     | "*="
    ///     | "/="
    ///     | "%="
    ///     | "**="
    ///     | "<<="
    ///     | ">>="
    ///     | "[<<]="
    ///     | "[>>]="
    ///     | "|="
    ///     | "&="
    ///     | "^="
    ///     | "!|="
    ///     | "!&="
    ///     | "!^="
    /// ) expression | expression`
    fn assign(&mut self) -> Result<ast::Expr> {
        let expr = self.expression()?;
        if let Some((op_span, op)) = self.pull_into_if(|op| {
            matches!(
                op,
                Operator::Assign
                    | Operator::AddAssign
                    | Operator::SubAssign
                    | Operator::MulAssign
                    | Operator::DivAssign
                    | Operator::RemAssign
                    | Operator::PowAssign
                    | Operator::ShlAssign
                    | Operator::ShrAssign
                    | Operator::RotlAssign
                    | Operator::RotrAssign
                    | Operator::OrAssign
                    | Operator::AndAssign
                    | Operator::XorAssign
                    | Operator::NorAssign
                    | Operator::NandAssign
                    | Operator::XnorAssign
            )
        }) {
            Ok(ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs: self.expression()?,
            })))
        } else {
            Ok(expr)
        }
    }

    /// `expression -> if_statement | loop_block | block_statement | or`
    pub fn expression(&mut self) -> Result<ast::Expr> {
        self.if_statement()
            .map_inner(|x| ast::BlockExpr::If(Box::new(x)))
            .or_else(|| {
                self.loop_block()
                    .map_inner(|x| ast::BlockExpr::Loop(Box::new(x)))
            })
            .or_else(|| self.block_statement().map_inner(ast::BlockExpr::Block))
            .map_inner(ast::Expr::Block)
            .unwrap_or_else(|| self.or())
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<ast::Expr> {
        let mut expr = self.xor()?;
        while let Some((op_span, op)) =
            self.pull_into_if(|kind| matches!(kind, Operator::Or | Operator::Nor))
        {
            let rhs = self.xor()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `xor -> and ( ("^" | "!^") and )*`
    fn xor(&mut self) -> Result<ast::Expr> {
        let mut expr = self.and()?;
        while let Some((op_span, op)) =
            self.pull_into_if(|kind| matches!(kind, Operator::Xor | Operator::Xnor))
        {
            let rhs = self.and()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<ast::Expr> {
        let mut expr = self.equality()?;
        while let Some((op_span, op)) =
            self.pull_into_if(|kind| matches!(kind, Operator::And | Operator::Nand))
        {
            let rhs = self.equality()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<ast::Expr> {
        let mut expr = self.comparison()?;
        while let Some((op_span, op)) =
            self.pull_into_if(|kind| matches!(kind, Operator::Ne | Operator::Eq))
        {
            let rhs = self.comparison()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<ast::Expr> {
        let mut expr = self.shift()?;
        while let Some((op_span, op)) = self.pull_into_if(|kind| {
            matches!(
                kind,
                Operator::Gt | Operator::Ge | Operator::Lt | Operator::Le
            )
        }) {
            let rhs = self.shift()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<ast::Expr> {
        let mut expr = self.term()?;
        while let Some((op_span, op)) = self.pull_into_if(|kind| {
            matches!(
                kind,
                Operator::Shl | Operator::Shr | Operator::Rotl | Operator::Rotr
            )
        }) {
            let rhs = self.term()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<ast::Expr> {
        let mut expr = self.factor()?;
        while let Some((op_span, op)) =
            self.pull_into_if(|kind| matches!(kind, Operator::Add | Operator::SubNeg))
        {
            let rhs = self.factor()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `factor -> unary_prefix ( ( "*" | "/" | "%" ) unary_prefix )*`
    fn factor(&mut self) -> Result<ast::Expr> {
        let mut expr = self.unary_prefix()?;
        while let Some((op_span, op)) =
            self.pull_into_if(|kind| matches!(kind, Operator::Mul | Operator::Div | Operator::Rem))
        {
            let rhs = self.unary_prefix()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    // TODO: `szof uint 3 ** 2` shouldn't be valid syntax
    /// `unary_prefix -> ( ( ( "!" | "!!" | "-" ) | "szof" ( type_expression )? ) )* exponent`
    fn unary_prefix(&mut self) -> Result<ast::Expr> {
        todo!()
    }

    /// `exponent -> conversion ( "**" conversion )*`
    fn exponent(&mut self) -> Result<ast::Expr> {
        let mut expr = self.conversion()?;
        while let Some((op_span, op)) = self.pull_into_if(|kind| matches!(kind, Operator::Pow)) {
            let rhs = self.conversion()?;
            expr = ast::Expr::Binary(Box::new(ast::Binary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `conversion -> unary_postfix ( "-:>" | "=:>" ) type_expression`
    fn conversion(&mut self) -> Result<ast::Expr> {
        todo!()
    }

    /// `unary_postfix -> primary ( "?" | "++" | "--" )*`
    fn unary_postfix(&mut self) -> Result<ast::Expr> {
        todo!()
    }

    /// `primary -> literal | group | variable | type_expression`
    ///
    /// `type_expression`/`type_bound` are included as a variant of `primary` in order
    /// to allow type errors instead of just "wrong token" errors.
    fn primary(&mut self) -> Result<ast::Expr> {
        todo!()
    }

    /// `directive -> "inpt"`
    fn directive(&mut self) -> Option<ast::Directive> {
        self.pull_into()
            .map(|(kw_span, kw)| ast::Directive { kw, kw_span })
    }

    /// `pattern -> IDENTIFIER`
    fn bind_pattern(&mut self) -> Option<ast::BindingPattern> {
        self.pull_if_eq(TokenKind::Ident)
            .map(|ident| ast::BindingPattern { ident })
    }

    /// `variable -> IDENTIFIER`
    fn variable(&mut self) -> Option<ast::Variable> {
        self.pull_if_eq(TokenKind::Ident)
            .map(|ident| ast::Variable { ident })
    }

    /// `literal -> "true" | "fals" | "none" | BYTE | UINT | SINT | FLPT | CHAR | TEXT | MACRO`
    fn literal(&mut self) -> Option<ast::Literal> {
        self.pull_if(|(_, kind)| {
            matches!(
                kind,
                TokenKind::BoolLit
                    | TokenKind::Keyword(Keyword::None)
                    | TokenKind::NumberLit
                    | TokenKind::CharLit
                    | TokenKind::TextLit
                    | TokenKind::Macro
            )
        })
        .map(|(span, kind)| ast::Literal { kind, span })
    }

    /// `sequence -> "[" expression "]"`
    fn sequence(&mut self) -> Option<Result<ast::Grouping>> {
        self.pull_if_delim(Bracket::Brack, Self::expression)
            .map_inner(|x| ast::Grouping {
                open: x.open,
                expr: x.inner,
                close: x.close,
            })
    }

    /// `group -> "(" expression ")"`
    fn group(&mut self) -> Option<Result<ast::Grouping>> {
        self.pull_if_delim(Bracket::Paren, Self::expression)
            .map_inner(|x| ast::Grouping {
                open: x.open,
                expr: x.inner,
                close: x.close,
            })
    }

    /// `generic_group -> "<" ( type_expression ( "," type_expression )* ( "," )? )? ">"`
    fn generic_group(&mut self) -> Option<Result<ast::GenericGroup>> {
        self.pull_if_delim(Bracket::Angle, |parser| {
            parser.pull_if_separated(None, Punctuation::Comma, |parser| {
                Some(parser.type_expression())
            })
        })
        .map_inner(|x| ast::GenericGroup {
            open: x.open,
            list: x.inner,
            close: x.close,
        })
    }

    /// `type_expression -> ( ( "nevr" | "bool" | "uint" | "sint" | "flpt" | "char" | "text" | "fail" | IDENTIFIER ) "::" )*
    ///     ( "nevr" | "bool" | "uint" | "sint" | "flpt" | "char" | "text" | "fail" | IDENTIFIER )
    ///     ( generic_group )?
    ///     ( "|" ( "fail" | "none" | "nevr" ) )?`
    fn type_expression(&mut self) -> Result<ast::TypeExpr> {
        todo!()
    }

    /// `iff_clause -> "<=>" type_bound`
    fn iff_clause(&mut self) -> Option<Result<ast::IffClause>> {
        self.pull_if_eq(Punctuation::Iff).map(|iff| {
            Ok(ast::IffClause {
                iff,
                bound: self.type_bound()?,
            })
        })
    }

    /// `type_bound -> exact_bound | member_bound | shape_bound | expr_bound | grouped_bound | or_bound`
    fn type_bound(&mut self) -> Result<ast::TypeBound> {
        todo!()
    }

    /// `exact_bound -> "only" type_expression`
    fn exact_bound(&mut self) -> Option<Result<ast::ExactTypeBound>> {
        self.pull_if_eq(Keyword::Only).map(|kw| {
            Ok(ast::ExactTypeBound {
                kw,
                ty: self.type_expression()?,
            })
        })
    }

    /// `member_bound -> "has" IDENTIFIER ( colon_bound )?`
    fn member_bound(&mut self) -> Option<Result<ast::MemberBound>> {
        self.pull_if_eq(Keyword::Has).map(|kw| {
            Ok(ast::MemberBound {
                kw,
                member: self.try_pull_eq(TokenKind::Ident)?,
                req: self.colon_bound().transpose()?,
            })
        })
    }

    /// `type_bound -> ":" type_bound`
    fn colon_bound(&mut self) -> Option<Result<ast::ColonBound>> {
        self.pull_if_eq(Punctuation::Colon).map(|colon| {
            Ok(ast::ColonBound {
                colon,
                bound: self.type_bound()?,
            })
        })
    }

    /// `shape_bound -> "fun" "(" ( type_expression ( "," type_expression )* ( "," )? )? ")" "->" type_expression`
    fn shape_bound(&mut self) -> Option<Result<ast::ShapeBound>> {
        todo!()
    }

    /// `expr_bound -> "for" IDENTIFIER ":" expression`
    fn expr_bound(&mut self) -> Option<Result<ast::ExprBound>> {
        self.pull_if_eq(Keyword::For).map(|kw| {
            Ok(ast::ExprBound {
                kw,
                name: self.try_pull_eq(TokenKind::Ident)?,
                colon: self.try_pull_eq(Punctuation::Colon)?,
                expr: self.expression()?,
            })
        })
    }

    /// `grouped_bound -> "(" type_bound ")"`
    fn grouped_bound(&mut self) -> Option<Result<ast::GroupedBound>> {
        self.pull_if_delim(Bracket::Paren, |parser| parser.type_bound())
            .map_inner(|x| ast::GroupedBound {
                open: x.open,
                inner: x.inner,
                close: x.close,
            })
    }

    /// `or_bound -> xor_bound ( "|" xor_bound )*`
    fn or_bound(&mut self) -> Result<ast::TypeBound> {
        todo!()
    }

    /// `xor_bound -> and_bound ( "/" and_bound )*`
    fn xor_bound(&mut self) -> Result<ast::TypeBound> {
        todo!()
    }

    /// `and_bound -> not_bound ( "+" not_bound )*`
    fn and_bound(&mut self) -> Result<ast::TypeBound> {
        todo!()
    }

    /// `not_bound -> ( "!" )* type_bound`
    fn not_bound(&mut self) -> Result<ast::TypeBound> {
        todo!()
    }
}
