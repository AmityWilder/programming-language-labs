use crate::{
    analyzer::types::bounds::Combinator,
    error::{CheckDiagnostics, Diagnostic, DiagnosticKind, ExpectedToken},
    lex::token::{
        Operator, TokenKind,
        keyword::Keyword,
        punc::{BadBracketCombo, Bracket, OpSide, Punctuation},
    },
    src_code::{
        code::SourceCode,
        span::{FileId, RelSpan, usize_to_uspan, uspan},
    },
    util::OrAllErrsExt,
};
use ast::{Expectable, ExpectableExt};

pub mod ast;
pub mod fmt;

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

    pub trait OkOrFlatten<T, E> {
        fn ok_or_flat(self, error: E) -> Result<T, E>;

        fn ok_or_else_flat<F>(self, error: F) -> Result<T, E>
        where
            F: FnOnce() -> E;
    }

    impl<T, E> OkOrFlatten<T, E> for Option<Result<T, E>> {
        fn ok_or_flat(self, error: E) -> Result<T, E> {
            self.unwrap_or(Err(error))
        }

        fn ok_or_else_flat<F>(self, error: F) -> Result<T, E>
        where
            F: FnOnce() -> E,
        {
            self.unwrap_or_else(move || Err(error()))
        }
    }

    pub trait InspectAlways: Sized {
        fn inspect_always(self, f: impl FnOnce(&Self)) -> Self {
            f(&self);
            self
        }
    }

    impl<T> InspectAlways for Option<T> {}
    impl<T, E> InspectAlways for Result<T, E> {}
}
use util::{InspectAlways, MapInner, OkOrFlatten};

pub type Result<T> = std::result::Result<T, CheckDiagnostics>;

pub trait ParserExpect {
    type Item;

    fn ok_or_unexpected<I>(self, parser: &mut Parser<I>) -> Result<Self::Item>
    where
        I: Iterator<Item = (RelSpan, TokenKind)>;
}

impl<T: ExpectableExt> ParserExpect for T {
    type Item = T::Item;

    fn ok_or_unexpected<I>(self, parser: &mut Parser<I>) -> Result<Self::Item>
    where
        I: Iterator<Item = (RelSpan, TokenKind)>,
    {
        self.ok_or_expected().map_err(|e| parser.error(e))
    }
}

impl<T: Expectable> ParserExpect for Option<Result<T>> {
    type Item = T;

    fn ok_or_unexpected<I>(self, parser: &mut Parser<I>) -> Result<T>
    where
        I: Iterator<Item = (RelSpan, TokenKind)>,
    {
        self.ok_or(T::EXPECTED)
            .map_err(|e| parser.error(e))
            .flatten()
    }
}

pub fn parse<'src, I>(
    iter: I,
    diagnostics: &mut Vec<Diagnostic>,
) -> std::result::Result<ast::Program, CheckDiagnostics>
where
    I: IntoIterator<Item = (FileId, &'src SourceCode)>,
{
    iter.into_iter()
        .map(|(id, code)| {
            parse_unit(
                id,
                usize_to_uspan(code.content().len()),
                code.tokens().iter(),
                diagnostics,
            )
        })
        .collect_or_drop_all_errs()
        .map(|units| ast::Program { units })
        .ok_or(CheckDiagnostics)
}

pub fn parse_unit<I>(
    file: FileId,
    file_len: uspan,
    tokens: I,
    diagnostics: &mut Vec<Diagnostic>,
) -> std::result::Result<ast::ProgramUnit, CheckDiagnostics>
where
    I: IntoIterator<Item = (RelSpan, TokenKind)>,
{
    Parser::new(file, file_len, tokens, diagnostics).program()
}

#[derive(Debug)]
pub struct Parser<'err, I: Iterator> {
    file: FileId,
    file_len: uspan,
    #[expect(clippy::type_complexity, reason = "type alias messes up derives")]
    tokens: std::iter::Peekable<std::iter::Filter<I, fn(&I::Item) -> bool>>,
    diagnostics: &'err mut Vec<Diagnostic>,

    /// The number of tokens "pulled" from the stream.
    #[cfg(debug_assertions)]
    consumed: usize,
    /// The list of parse methods used which have not yet returned, and the value of [`Self::consumed`] when they were called.
    /// If a method already in this gets called again, and [`Self::consumed`] has not changed since the last time it was called, there is a cycle.
    #[cfg(debug_assertions)]
    call_stack: Vec<(&'static str, usize)>,
}

macro_rules! push_parse_call {
    ($parser:ident, $fn_name:expr) => {{
        #[cfg(debug_assertions)]
        {
            let fn_name: &'static str = $fn_name;
            let prev_call = $parser
                .call_stack
                .iter()
                .copied()
                .rfind(|(name, _)| *name == fn_name);
            if false {
                println!(
                    "{}. {fn_name}: {}; previously {:?}",
                    $parser.call_stack.len(),
                    $parser.consumed,
                    prev_call.map(|(_, n)| n)
                );
            }
            debug_assert!(
                !prev_call.is_some_and(|(_, prev_consumed)| $parser.consumed == prev_consumed),
                "parser cycle detected\
                \n call stack: {:?}",
                $parser.call_stack,
            );
            $parser.call_stack.push((fn_name, $parser.consumed));
        }
    }};
}

macro_rules! pop_parse_call {
    ($parser:ident, $fn_name:expr, $val:expr) => {{
        #[cfg(debug_assertions)]
        {
            let fn_name: &'static str = $fn_name;
            if false {
                println!("  finished with {fn_name}, found: {:?}", $val);
            }
            let popped = $parser.call_stack.pop();
            debug_assert_eq!(
                popped.map(|(name, _)| name),
                Some(fn_name),
                "push and pop should be balanced"
            );
        }
    }};
}

impl<'err, I: Iterator<Item = (RelSpan, TokenKind)>> Parser<'err, I> {
    pub fn new<J>(
        file: FileId,
        file_len: uspan,
        tokens: J,
        diagnostics: &'err mut Vec<Diagnostic>,
    ) -> Self
    where
        J: IntoIterator<IntoIter = I>,
    {
        let filter: fn(&I::Item) -> bool = |&(_, kind)| !kind.is_ignored();
        Self {
            file,
            file_len,
            tokens: tokens.into_iter().filter(filter).peekable(),
            diagnostics,
            consumed: 0,
            call_stack: Vec::new(),
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
    fn is_next_not_close(&mut self) -> bool {
        self.is_next(|(_, k)| {
            !matches!(
                k,
                TokenKind::Punc(Punctuation::RBrace | Punctuation::RBrack | Punctuation::RParen)
            )
        })
    }

    /// Similar to [`Self::is_next_close`], but **also** returns `true` for `>`.
    fn is_next_not_angle_close(&mut self) -> bool {
        self.is_next(|(_, k)| {
            !matches!(
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
        self.tokens
            .next()
            .inspect(|_| self.consumed = self.consumed.strict_add(1))
    }

    fn pull_if<P>(&mut self, p: P) -> Option<I::Item>
    where
        P: FnOnce(&I::Item) -> bool,
    {
        self.tokens
            .next_if(p)
            .inspect(|_| self.consumed = self.consumed.strict_add(1))
    }

    fn pull_if_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(I::Item) -> Option<T>,
    {
        self.tokens
            .next_if_map(|item| f(item).ok_or(item))
            .inspect(|_| self.consumed = self.consumed.strict_add(1))
    }

    fn pull_if_eq<Q>(&mut self, kind: Q) -> Option<RelSpan>
    where
        Q: Into<TokenKind>,
    {
        // if the kind can only be one thing, we don't need to output it.
        self.pull_if(|(_, k)| *k == kind.into())
            .map(|(span, _)| span)
    }

    fn pull_into_if<T, P>(&mut self, p: P) -> Option<(RelSpan, T)>
    where
        TokenKind: TryInto<T>,
        P: FnOnce(&T) -> bool,
    {
        self.pull_if_map(|(span, kind)| kind.try_into().ok().filter(p).map(|t| (span, t)))
    }

    fn pull_into<T>(&mut self) -> Option<(RelSpan, T)>
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

    fn try_pull_eq<Q>(&mut self, kind: Q) -> Result<RelSpan>
    where
        Q: Into<TokenKind>,
    {
        let kind = kind.into();
        // if the kind can only be one thing, we don't need to output it.
        self.pull_if_eq(kind)
            .ok_or_else(|| self.error(ExpectedToken::Verbatim(kind)))
    }

    fn try_pull_into_if<T, P>(&mut self, expected: ExpectedToken, p: P) -> Result<(RelSpan, T)>
    where
        TokenKind: TryInto<T>,
        P: FnOnce(&T) -> bool,
    {
        self.try_pull_map(
            |(span, kind)| kind.try_into().ok().filter(p).map(|t| (span, t)),
            expected,
        )
    }

    fn try_pull_into<T>(&mut self, expected: ExpectedToken) -> Result<(RelSpan, T)>
    where
        TokenKind: TryInto<T>,
    {
        self.try_pull_map(
            |(span, kind)| kind.try_into().ok().map(|t| (span, t)),
            expected,
        )
    }

    fn rip_rangle(&mut self) -> Option<RelSpan> {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "char.len_utf8() is at most 4"
        )]
        const RANGLE_LEN: u32 = '>'.len_utf8() as u32;
        if let trivial @ Some(_) = self.pull_if_eq(Punctuation::Gt) {
            trivial
        } else if let Some((span, kind)) = self.tokens.peek_mut() {
            *kind = match kind {
                TokenKind::Punc(Punctuation::Shr) => TokenKind::Punc(Punctuation::Gt),
                TokenKind::Punc(Punctuation::ShrAssign) => TokenKind::Punc(Punctuation::Ge),
                _ => return None,
            };
            let start = span.start;
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the span contains a '>' so because file length is not allowed to exceed u32::MAX, \
                the end position of that '>' cannot exceed u32::MAX."
            )]
            let mid = start + RANGLE_LEN;
            span.start = mid;
            Some((start..mid).into())
        } else {
            None
        }
    }

    /// `pull_if_delim -> brack.open inner brack.close`
    // I know it's a big function, but `brack` should be a literal, so inlining should allow the compiler to cut down on some of the branches
    #[inline]
    fn pull_if_delim<R, F>(
        &mut self,
        brack: Bracket,
        inner: F,
    ) -> Option<Result<(RelSpan, R, RelSpan)>>
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
                let e = if let Some(&(span, kind)) = self.peek() {
                    let kind = 'err_kind: {
                        let close = match kind {
                            TokenKind::Punc(Punctuation::RBrack) => Bracket::Brack,
                            TokenKind::Punc(Punctuation::RParen) => Bracket::Paren,
                            TokenKind::Punc(Punctuation::RBrace) => Bracket::Brace,
                            // '>' should not be reported as an unexpected close bracket, only an unexpected token.
                            _ => {
                                break 'err_kind DiagnosticKind::UnexpectedToken {
                                    expected: ExpectedToken::Verbatim(brack.close_punc().into()),
                                };
                            }
                        };
                        DiagnosticKind::IncorrectCloseBracket {
                            open_range: open.within_file(self.file),
                            failure: BadBracketCombo::new(brack, close)
                                .expect("matching brackets should not produce an error"),
                        }
                    };
                    Diagnostic::new(span.within_file(self.file), kind)
                } else {
                    Diagnostic::new(
                        RelSpan::from(self.file_len..self.file_len).within_file(self.file),
                        DiagnosticKind::MissingCloseBracket {
                            open_range: open.within_file(self.file),
                            expect: brack,
                        },
                    )
                };
                self.diagnostics.push(e);
                CheckDiagnostics
            })?;
            Ok((open, inner, close))
        })
    }

    /// `pull_terminated -> ( f ( "," f )* ( "," )? )?`
    fn pull_terminated<Q, T, F>(&mut self, sep: Q, mut f: F) -> Result<Vec<T>>
    where
        Q: Into<TokenKind>,
        F: FnMut(&mut Self) -> Option<Result<T>>,
    {
        let mut any_err = false;
        let sep = sep.into();
        let mut items = Vec::new();
        while let Some(item) = f(self) {
            if let Ok(item) = item {
                items.push(item);
                if self.pull_if_eq(sep).is_none() {
                    break;
                }
            } else {
                any_err = true;
                self.sync();
            }
        }
        if any_err {
            Err(CheckDiagnostics)
        } else {
            Ok(items)
        }
    }

    /// Does not consume the next token
    fn error(&mut self, expected: ExpectedToken) -> CheckDiagnostics {
        let err = if let Some(&(span, _)) = self.peek() {
            Diagnostic::new(
                span.within_file(self.file),
                DiagnosticKind::UnexpectedToken { expected },
            )
        } else {
            Diagnostic::new(
                RelSpan::from(self.file_len..self.file_len).within_file(self.file),
                DiagnosticKind::MissingToken { expected },
            )
        };
        self.diagnostics.push(err);
        CheckDiagnostics
    }

    fn sync(&mut self) {
        use Keyword::{
            Alt, Cat, Cord, Def, Done, Emit, Fail, For, Fun, Give, If, Let, Loop, On, Pick, Prnt,
            Pvt, Rec, Rep, Skip, Sub, Sup, Uni,
        };
        while let Some((_, kind)) = self.pull() {
            // end of current statement
            if matches!(kind, TokenKind::Punc(Punctuation::Semi)) {
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
                            | On
                            | Let
                            | Uni
                            | Pvt
                            | If
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

impl<I: Iterator<Item = (RelSpan, TokenKind)>> Parser<'_, I> {
    /// `program -> ( statement )*`
    pub fn program(&mut self) -> Result<ast::ProgramUnit> {
        push_parse_call!(self, "program");
        std::iter::from_fn(|| {
            self.peek()
                .is_some()
                .then(|| self.declaration().inspect_err(|_| self.sync()))
        })
        .collect_or_drop_all_errs()
        .map(|decls| ast::ProgramUnit {
            file: self.file,
            decls,
        })
        .ok_or(CheckDiagnostics)
        .inspect_always(|x| pop_parse_call!(self, "program", x))
    }

    /// `declaration -> rec_declaration | cat_declaration | var_declaration | dfer_declaration | statement`
    pub fn declaration(&mut self) -> Result<ast::Decl> {
        push_parse_call!(self, "declaration");
        self.rec_declaration()
            .map_inner(ast::DeclType::Rec)
            .or_else(|| self.cat_declaration().map_inner(ast::DeclType::Cat))
            .map_inner(ast::Decl::Type)
            .or_else(|| self.var_declaration().map_inner(ast::Decl::Var))
            .or_else(|| self.dfer_declaration().map_inner(ast::Decl::Dfer))
            .unwrap_or_else(|| self.statement().map(ast::Decl::Stmt))
            .inspect_always(|x| pop_parse_call!(self, "declaration", x))
    }

    /// `assignment -> "=" expression`
    pub fn assignment(&mut self) -> Option<Result<ast::Assignment>> {
        push_parse_call!(self, "assignment");
        self.pull_if_eq(Punctuation::Assign)
            .map(|op| {
                Ok(ast::Assignment {
                    op,
                    expr: self.expression()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "assignment", x))
    }

    /// `var_declaration -> ( "uni" | "pvt" | "let" ) pattern ( type_annotation )? ( assignment )? ";"`
    pub fn var_declaration(&mut self) -> Option<Result<ast::DeclVar>> {
        push_parse_call!(self, "var_declaration");
        self.pull_into()
            .map(|(kw_span, kw)| {
                Ok(ast::DeclVar {
                    kw,
                    kw_span,
                    binding: self.bind_pattern().ok_or_unexpected(self)?,
                    ty: self.type_annotation().transpose()?,
                    assignment: self.assignment().transpose()?,
                    semi: self.try_pull_eq(TokenKind::Punc(Punctuation::Semi))?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "var_declaration", x))
    }

    /// `rec_declaration -> "rec" IDENT ( generic_group )? "{" ( field ( "," field )* ( "," )? )? "}"`
    pub fn rec_declaration(&mut self) -> Option<Result<ast::DeclRec>> {
        push_parse_call!(self, "rec_declaration");
        self.pull_if_eq(Keyword::Rec)
            .map(|kw| {
                Ok(ast::DeclRec {
                    kw,
                    name: self.try_pull_eq(TokenKind::Ident)?,
                    generics: self.generic_group().transpose()?,
                    body: self
                        .field_set()
                        .ok_or_else_flat(|| self.error(ExpectedToken::RecBody))?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "rec_declaration", x))
    }

    pub fn field_set(&mut self) -> Option<Result<ast::FieldSet>> {
        push_parse_call!(self, "field_set");
        self.pull_if_delim(Bracket::Brace, |parser| {
            parser.pull_terminated(Punctuation::Comma, Self::field)
        })
        .map_inner(|(open, fields, close)| ast::FieldSet {
            open,
            fields,
            close,
        })
        .inspect_always(|x| pop_parse_call!(self, "field_set", x))
    }

    pub fn field(&mut self) -> Option<Result<ast::Field>> {
        push_parse_call!(self, "field");
        self.pull_if_eq(TokenKind::Ident)
            .map(|name| {
                Ok(ast::Field {
                    name,
                    ty: self.type_annotation().ok_or_else_flat(|| {
                        self.error(ExpectedToken::Verbatim(TokenKind::Punc(Punctuation::Colon)))
                    })?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "field", x))
    }

    /// `cat_declaration -> "cat" name ( generic_group )? cat_disc_set`
    pub fn cat_declaration(&mut self) -> Option<Result<ast::DeclCat>> {
        push_parse_call!(self, "cat_declaration");
        self.pull_if_eq(Keyword::Cat)
            .map(|kw| {
                Ok(ast::DeclCat {
                    kw,
                    name: self.try_pull_eq(TokenKind::Ident)?,
                    discriminants: self.cat_disc_set().ok_or_else_flat(|| {
                        self.error(ExpectedToken::Verbatim(TokenKind::Punc(
                            Punctuation::LBrace,
                        )))
                    })?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "cat_declaration", x))
    }

    /// `cat_disc_set -> "{" ( cat_disc ( "," cat_disc )* ( "," )? )? "}"`
    pub fn cat_disc_set(&mut self) -> Option<Result<ast::CatDiscSet>> {
        push_parse_call!(self, "cat_disc_set");
        self.pull_if_delim(Bracket::Brace, |parser| {
            parser.pull_terminated(Punctuation::Comma, Self::cat_disc)
        })
        .map_inner(|(open, items, close)| ast::CatDiscSet { open, items, close })
        .inspect_always(|x| pop_parse_call!(self, "cat_disc_set", x))
    }

    /// `cat_disc -> IDENT ( assignment )?`
    pub fn cat_disc(&mut self) -> Option<Result<ast::CatDisc>> {
        push_parse_call!(self, "cat_disc");
        self.pull_if_eq(TokenKind::Ident)
            .map(|name| {
                Ok(ast::CatDisc {
                    name,
                    val: self.assignment().transpose()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "cat_disc", x))
    }

    /// `type_annotation -> ":" type_expression`
    pub fn type_annotation(&mut self) -> Option<Result<ast::TypeAnnotation>> {
        push_parse_call!(self, "type_annotation");
        self.pull_if_eq(Punctuation::Colon)
            .map(|colon| {
                Ok(ast::TypeAnnotation {
                    colon,
                    ty: self.type_expression()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "type_annotation", x))
    }

    /// `statement -> block_statement
    ///     | print_statement
    ///     | done_statement
    ///     | skip_statement
    ///     | if_statement
    ///     | loop_block
    ///     | expression_statement`
    pub fn statement(&mut self) -> Result<ast::Stmt> {
        push_parse_call!(self, "statement");
        self.block_statement()
            .map_inner(ast::Stmt::Block)
            .or_else(|| self.print_statement().map_inner(ast::Stmt::Print))
            .or_else(|| self.done_statement().map_inner(ast::Stmt::Done))
            .or_else(|| self.skip_statement().map_inner(ast::Stmt::Skip))
            .or_else(|| self.if_statement().map_inner(ast::Stmt::If))
            .or_else(|| self.loop_block().map_inner(ast::Stmt::Loop))
            .unwrap_or_else(|| self.expression_statement().map(ast::Stmt::Expr))
            .inspect_always(|x| pop_parse_call!(self, "statement", x))
    }

    /// `expression_statement -> assign ";"`
    pub fn expression_statement(&mut self) -> Result<ast::ExprStmt> {
        push_parse_call!(self, "expression_statement");
        self.assign()
            .and_then(|expr| {
                Ok(ast::ExprStmt {
                    expr,
                    semi: self.try_pull_eq(Punctuation::Semi)?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "expression_statement", x))
    }

    /// `print_statement -> "prnt" expression ";"`
    pub fn print_statement(&mut self) -> Option<Result<ast::PrintStmt>> {
        push_parse_call!(self, "print_statement");
        self.pull_if_eq(Keyword::Prnt)
            .map(|kw| {
                Ok(ast::PrintStmt {
                    kw,
                    group: self.group().ok_or_unexpected(self)?,
                    semi: self.try_pull_eq(Punctuation::Semi)?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "print_statement", x))
    }

    /// `dfer_declaration -> "dfer" ( dfer_param )? block_statement`
    pub fn dfer_declaration(&mut self) -> Option<Result<ast::DeclDfer>> {
        push_parse_call!(self, "dfer_declaration");
        self.pull_if_eq(Keyword::Dfer)
            .map(|kw| {
                Ok(ast::DeclDfer {
                    kw,
                    param: self.dfer_param().transpose()?,
                    on_pop: self.block_statement().ok_or_unexpected(self)?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "dfer_declaration", x))
    }

    /// `dfer_param -> "(" bind_pattern ")"`
    pub fn dfer_param(&mut self) -> Option<Result<ast::DferParam>> {
        push_parse_call!(self, "dfer_param");
        self.pull_if_delim(Bracket::Paren, |parser| {
            parser.bind_pattern().ok_or_unexpected(parser)
        })
        .map_inner(|(open, param, close)| ast::DferParam { open, param, close })
        .inspect_always(|x| pop_parse_call!(self, "dfer_param", x))
    }

    /// `pick_statement -> "pick" expression "{" ( pick_arm ( "," pick_arm )* ( "," )? )? "}"`
    pub fn pick_statement(&mut self) -> Option<Result<ast::PickStmt>> {
        push_parse_call!(self, "pick_statement");
        self.pull_if_eq(Keyword::Pick)
            .map(|kw| {
                let expr = self.expression()?;
                let (open, arms, close) = self
                    .pull_if_delim(Bracket::Brace, |parser| {
                        parser.pull_terminated(Punctuation::Comma, |parser| {
                            parser.is_next_not_close().then(|| parser.pick_arm())
                        })
                    })
                    .ok_or_else_flat(|| {
                        self.error(ExpectedToken::Verbatim(TokenKind::Punc(
                            Punctuation::LBrace,
                        )))
                    })?;
                Ok(ast::PickStmt {
                    kw,
                    expr,
                    open,
                    arms,
                    close,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "pick_statement", x))
    }

    /// `pick_arm -> pick_pattern "=>" expression`
    pub fn pick_arm(&mut self) -> Result<ast::PickArm> {
        push_parse_call!(self, "pick_arm");
        Ok(ast::PickArm {
            pattern: self.pick_pattern()?,
            arrow: self.try_pull_eq(Punctuation::FatArrow)?,
            then: self.expression()?,
        })
        .inspect_always(|x| pop_parse_call!(self, "pick_arm", x))
    }

    /// `pick_pattern -> ?`
    pub fn pick_pattern(&mut self) -> Result<ast::PickPattern> {
        push_parse_call!(self, "pick_pattern");
        todo!();
        Ok(ast::PickPattern { /* TODO */ })
            .inspect_always(|x| pop_parse_call!(self, "pick_pattern", x))
    }

    /// `if_statement -> "if" expression block_statement ( or_statement )?`
    pub fn if_statement(&mut self) -> Option<Result<ast::IfStmt>> {
        push_parse_call!(self, "if_statement");
        self.pull_if_eq(Keyword::If)
            .map(|kw| {
                Ok(ast::IfStmt {
                    kw,
                    pred: self.expression()?,
                    then: self.block_statement().ok_or_unexpected(self)?,
                    or: self.or_statement().transpose()?.map(Box::new),
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "if_statement", x))
    }

    /// `or_statement -> "or" or_body`
    pub fn or_statement(&mut self) -> Option<Result<ast::OrStmt>> {
        push_parse_call!(self, "or_statement");
        self.pull_if_eq(Keyword::Or)
            .map(|kw| {
                Ok(ast::OrStmt {
                    kw,
                    body: self.or_body()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "or_statement", x))
    }

    /// `or_body -> if_statement | block_statement`
    pub fn or_body(&mut self) -> Result<ast::OrBody> {
        push_parse_call!(self, "or_body");
        self.if_statement()
            .map_inner(ast::OrBody::More)
            .or_else(|| self.block_statement().map_inner(ast::OrBody::Last))
            .ok_or_unexpected(self)
            .inspect_always(|x| pop_parse_call!(self, "or_body", x))
    }

    /// `loop_block -> ( scope_label )? loop_block1`
    pub fn loop_block(&mut self) -> Option<Result<ast::LoopBlock>> {
        push_parse_call!(self, "loop_block");
        // this may consume the label if there's no block...
        match self.scope_label().transpose() {
            Ok(lbl) => match (lbl, self.loop_block1()) {
                // no label: success is up to the loop.
                (None, block) => block,

                // labelled loop: supported language feature.
                (lbl @ Some(_), Some(block)) => Some(block.map(|mut block| {
                    block.label = lbl;
                    block
                })),

                // loopless label: not a supported feature. should use `cord` for this behavior.
                (Some(_), None) => Some(Err(CheckDiagnostics)),
            },

            // invalid label
            Err(e) => Some(Err(e)),
        }
        .inspect_always(|x| pop_parse_call!(self, "loop_block", x))
    }

    /// `label -> "." IDENTIFIER`
    pub fn label(&mut self) -> Option<Result<ast::Label>> {
        push_parse_call!(self, "label");
        self.pull_if_eq(Punctuation::Dot)
            .map(|dot| {
                Ok(ast::Label {
                    dot,
                    name: self.try_pull_eq(TokenKind::Ident)?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "label", x))
    }

    /// `scope_label -> label ":"`
    pub fn scope_label(&mut self) -> Option<Result<ast::ScopeLabel>> {
        push_parse_call!(self, "scope_label");
        self.label()
            .map(|label| {
                Ok(ast::ScopeLabel {
                    label: label?,
                    colon: self.try_pull_eq(Punctuation::Colon)?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "scope_label", x))
    }

    /// `loop_block1 -> ( "loop" | "cord" | "for" bind_pattern "in" expression | "rep" expression ) block_statement`
    pub fn loop_block1(&mut self) -> Option<Result<ast::LoopBlock>> {
        push_parse_call!(self, "loop_block1");
        self.pull_if_eq(Keyword::Loop)
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
            })
            .map(|res| {
                res.and_then(|(kw, kind)| {
                    self.block_statement()
                        .ok_or_unexpected(self)
                        .map(|block| ast::LoopBlock {
                            label: None,
                            kw,
                            kind,
                            block,
                        })
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "loop_block1", x))
    }

    /// `done_statement -> "done" ( label )? expression ";"`
    pub fn done_statement(&mut self) -> Option<Result<ast::DoneStmt>> {
        push_parse_call!(self, "done_statement");
        self.pull_if_eq(Keyword::Done)
            .map(|kw| {
                let label = self.label().transpose()?;
                let semi_upcoming = self.is_next_eq(Punctuation::Semi);
                Ok(ast::DoneStmt {
                    kw,
                    label,
                    value: (!semi_upcoming).then(|| self.expression()).transpose()?,
                    semi: self.try_pull_eq(Punctuation::Semi)?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "done_statement", x))
    }

    /// `skip_statement -> "skip" ( label )? ";"`
    pub fn skip_statement(&mut self) -> Option<Result<ast::SkipStmt>> {
        push_parse_call!(self, "skip_statement");
        self.pull_if_eq(Keyword::Skip)
            .map(|kw| {
                Ok(ast::SkipStmt {
                    kw,
                    label: self.label().transpose()?,
                    semi: self.try_pull_eq(Punctuation::Semi)?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "skip_statement", x))
    }

    /// `block_statement -> "{" ( declaration )* "}"`
    pub fn block_statement(&mut self) -> Option<Result<ast::BlockStmt>> {
        push_parse_call!(self, "block_statement");
        self.pull_if_delim(Bracket::Brace, |parser| {
            std::iter::from_fn(|| {
                parser
                    .is_next_not_close()
                    .then(|| parser.declaration().inspect_err(|_| parser.sync()))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<Result<_>>()
        })
        .map_inner(|(open, body, close)| ast::BlockStmt { open, body, close })
        .inspect_always(|x| pop_parse_call!(self, "block_statement", x))
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
        push_parse_call!(self, "assign");
        self.expression()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "assign", x))
    }

    /// `expression -> if_statement | loop_block | block_statement | or`
    pub fn expression(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "expression");
        self.if_statement()
            .map_inner(|x| ast::BlockExpr::If(Box::new(x)))
            .or_else(|| {
                self.loop_block()
                    .map_inner(|x| ast::BlockExpr::Loop(Box::new(x)))
            })
            .or_else(|| self.block_statement().map_inner(ast::BlockExpr::Block))
            .map_inner(ast::Expr::Block)
            .unwrap_or_else(|| self.or())
            .inspect_always(|x| pop_parse_call!(self, "expression", x))
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "or");
        self.xor()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "or", x))
    }

    /// `xor -> and ( ("^" | "!^") and )*`
    fn xor(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "xor");
        self.and()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "xor", x))
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "and");
        self.equality()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "and", x))
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "equality");
        self.comparison()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "equality", x))
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "comparison");
        self.shift()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "comparison", x))
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "shift");
        self.term()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "shift", x))
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "term");
        self.factor()
            .and_then(|mut expr| {
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
            })
            .inspect_always(|x| pop_parse_call!(self, "term", x))
    }

    /// `factor -> unary_prefix ( ( "*" | "/" | "%" ) unary_prefix )*`
    fn factor(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "factor");
        self.unary_prefix()
            .and_then(|mut expr| {
                while let Some((op_span, op)) = self.pull_into_if(|kind| {
                    matches!(kind, Operator::Mul | Operator::Div | Operator::Rem)
                }) {
                    let rhs = self.unary_prefix()?;
                    expr = ast::Expr::Binary(Box::new(ast::Binary {
                        lhs: expr,
                        op,
                        op_span,
                        rhs,
                    }));
                }
                Ok(expr)
            })
            .inspect_always(|x| pop_parse_call!(self, "factor", x))
    }

    /// `unary_prefix -> "szof" type_expression | ( "!" | "!!" | "-" )* exponent`
    fn unary_prefix(&mut self) -> Result<ast::Expr> {
        use Operator::{Exists, Not, SubNeg, Szof};
        push_parse_call!(self, "unary_prefix");
        if let Some((op_span, op)) =
            self.pull_into_if(|k| matches!(k, Not | Exists | SubNeg | Szof))
        {
            match op {
                Szof => self.type_expression().map(|x| ast::Expr::Type(Box::new(x))),
                Not | Exists | SubNeg => self.unary_prefix(),
                _ => unreachable!("TODO: turn the `pull_into_if` into a `pull_into`"),
            }
            .map(|operand| {
                ast::Expr::Unary(Box::new(ast::Unary {
                    op,
                    op_span,
                    operand,
                    side: OpSide::Prefix,
                }))
            })
        } else {
            self.exponent()
        }
        .inspect_always(|x| pop_parse_call!(self, "unary_prefix", x))
    }

    /// `exponent -> conversion ( "**" conversion )*`
    fn exponent(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "exponent");
        self.conversion()
            .and_then(|mut expr| {
                while let Some((op_span, op)) =
                    self.pull_into_if(|kind| matches!(kind, Operator::Pow))
                {
                    let rhs = self.conversion()?;
                    expr = ast::Expr::Binary(Box::new(ast::Binary {
                        lhs: expr,
                        op,
                        op_span,
                        rhs,
                    }));
                }
                Ok(expr)
            })
            .inspect_always(|x| pop_parse_call!(self, "exponent", x))
    }

    /// `conversion -> unary_postfix ( "-:>" | "=:>" ) type_expression`
    fn conversion(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "conversion");
        self.unary_postfix()
            .and_then(|mut expr| {
                if let Some((op_span, op)) = self
                    .pull_into_if(|kind| matches!(kind, Operator::Convert | Operator::Transmute))
                {
                    expr = ast::Expr::Binary(Box::new(ast::Binary {
                        lhs: expr,
                        op,
                        op_span,
                        rhs: ast::Expr::Type(Box::new(self.type_expression()?)),
                    }));
                }
                Ok(expr)
            })
            .inspect_always(|x| pop_parse_call!(self, "conversion", x))
    }

    /// `unary_postfix -> primary ( "?" | "++" | "--" | subscript | fn_call )*`
    fn unary_postfix(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "unary_postfix");
        self.primary()
            .map(|mut expr| {
                while let Some((op_span, op)) = self.pull_into_if(|kind| {
                    matches!(kind, Operator::Coalesce | Operator::Incr | Operator::Decr)
                }) {
                    expr = ast::Expr::Unary(Box::new(ast::Unary {
                        op,
                        op_span,
                        operand: expr,
                        side: OpSide::Postfix,
                    }));
                }
                expr
            })
            .inspect_always(|x| pop_parse_call!(self, "unary_postfix", x))
    }

    // TODO: index and call

    /// `primary -> literal | group | variable | directive`
    fn primary(&mut self) -> Result<ast::Expr> {
        push_parse_call!(self, "primary");
        self.literal()
            .map(|x| Ok(ast::Expr::Literal(x)))
            .or_else(|| self.group().map_inner(|x| ast::Expr::Grouping(Box::new(x))))
            .or_else(|| self.variable().map(|x| Ok(ast::Expr::Variable(x))))
            .or_else(|| self.directive().map(|x| Ok(ast::Expr::Directive(x))))
            .ok_or_else_flat(|| self.error(ExpectedToken::Expr))
            .inspect_always(|x| pop_parse_call!(self, "primary", x))
    }

    /// `directive -> "inpt"`
    fn directive(&mut self) -> Option<ast::Directive> {
        push_parse_call!(self, "directive");
        self.pull_into()
            .map(|(kw_span, kw)| ast::Directive { kw, kw_span })
            .inspect_always(|x| pop_parse_call!(self, "directive", x))
    }

    /// `pattern -> IDENTIFIER`
    fn bind_pattern(&mut self) -> Option<ast::BindingPattern> {
        push_parse_call!(self, "bind_pattern");
        self.pull_if_eq(TokenKind::Ident)
            .map(|ident| ast::BindingPattern { ident })
            .inspect_always(|x| pop_parse_call!(self, "bind_pattern", x))
    }

    /// `variable -> IDENTIFIER`
    fn variable(&mut self) -> Option<ast::Variable> {
        push_parse_call!(self, "variable");
        self.pull_if_eq(TokenKind::Ident)
            .map(|ident| ast::Variable { ident })
            .inspect_always(|x| pop_parse_call!(self, "variable", x))
    }

    /// `literal -> "true" | "fals" | "none" | BYTE | UINT | SINT | FLPT | CHAR | TEXT | MACRO`
    fn literal(&mut self) -> Option<ast::Literal> {
        push_parse_call!(self, "literal");
        self.pull_into()
            .map(|(span, kind)| ast::Literal { kind, span })
            .inspect_always(|x| pop_parse_call!(self, "literal", x))
    }

    /// `sequence -> "[" expression "]"`
    fn sequence(&mut self) -> Option<Result<ast::Grouping>> {
        push_parse_call!(self, "sequence");
        self.pull_if_delim(Bracket::Brack, Self::expression)
            .map_inner(|(open, expr, close)| ast::Grouping { open, expr, close })
            .inspect_always(|x| pop_parse_call!(self, "sequence", x))
    }

    /// `group -> "(" expression ")"`
    fn group(&mut self) -> Option<Result<ast::Grouping>> {
        push_parse_call!(self, "group");
        self.pull_if_delim(Bracket::Paren, Self::expression)
            .map_inner(|(open, expr, close)| ast::Grouping { open, expr, close })
            .inspect_always(|x| pop_parse_call!(self, "group", x))
    }

    /// `generic_group -> "<" ( type_expression ( "," type_expression )* ( "," )? )? ">"`
    fn generic_group(&mut self) -> Option<Result<ast::GenericGroup>> {
        push_parse_call!(self, "generic_group");
        self.pull_if_delim(Bracket::Angle, |parser| {
            parser.pull_terminated(Punctuation::Comma, |parser| {
                parser
                    .is_next_not_angle_close()
                    .then(|| parser.type_expression())
            })
        })
        .map_inner(|(open, list, close)| ast::GenericGroup { open, list, close })
        .inspect_always(|x| pop_parse_call!(self, "generic_group", x))
    }

    /// `type_expression -> fallible_type`
    fn type_expression(&mut self) -> Result<ast::TypeExpr> {
        push_parse_call!(self, "type_expression");
        self.fallible_type()
            .inspect_always(|x| pop_parse_call!(self, "type_expression", x))
    }

    /// `fallible_type -> ref_type ( "|" ( "none" | "nevr" | "fail" ) )?`
    fn fallible_type(&mut self) -> Result<ast::TypeExpr> {
        push_parse_call!(self, "fallible_type");
        self.ref_type()
            .and_then(|then_ty| {
                if let Some(pipe) = self.pull_if_eq(Punctuation::Or) {
                    let (ty_span, ty) = self.try_pull_into(ExpectedToken::OrType)?;
                    Ok(ast::TypeExpr::Fallible(ast::FallType {
                        then_ty: Box::new(then_ty),
                        pipe,
                        ty,
                        ty_span,
                    }))
                } else {
                    Ok(then_ty)
                }
            })
            .inspect_always(|x| pop_parse_call!(self, "fallible_type", x))
    }

    /// `ref_type -> ( "@" )* ref_type | assoc_type`
    fn ref_type(&mut self) -> Result<ast::TypeExpr> {
        push_parse_call!(self, "ref_type");
        if let Some(sym) = self.pull_if_eq(Punctuation::Ref) {
            self.ref_type().map(|t| {
                ast::TypeExpr::Ref(ast::RefType {
                    sym,
                    ty: Box::new(t),
                })
            })
        } else {
            self.assoc_type()
        }
        .inspect_always(|x| pop_parse_call!(self, "ref_type", x))
    }

    /// `assoc_type -> type_source ( "::" type_instance )? | primary_type`
    fn assoc_type(&mut self) -> Result<ast::TypeExpr> {
        push_parse_call!(self, "assoc_type");
        self.type_source()
            .transpose()
            .and_then(|base| {
                if let Some(base) = base {
                    if let Some(sep) = self.pull_if_eq(Punctuation::PathSep) {
                        self.type_instance().map(|assoc| {
                            ast::TypeExpr::Assoc(Box::new(ast::AssocType { base, sep, assoc }))
                        })
                    } else {
                        match base {
                            ast::AssocTypeSource::Simple(name) => {
                                Ok(ast::TypeExpr::Instance(ast::TypeInstance {
                                    name,
                                    args: self.generic_group().transpose()?,
                                }))
                            }
                            // TBD: is the `<>` is important?
                            ast::AssocTypeSource::Grouped(ast::GroupedType { inner, .. }) => {
                                Ok(inner)
                            }
                        }
                    }
                } else {
                    self.primary_type()
                }
            })
            .inspect_always(|x| pop_parse_call!(self, "assoc_type", x))
    }

    /// `type_source -> grouped_type | typename`
    fn type_source(&mut self) -> Option<Result<ast::AssocTypeSource>> {
        push_parse_call!(self, "type_source");
        self.grouped_type()
            .map_inner(ast::AssocTypeSource::Grouped)
            .or_else(|| self.typename().map_inner(ast::AssocTypeSource::Simple))
            .inspect_always(|x| pop_parse_call!(self, "type_source", x))
    }

    /// `primary_type -> view_type | fun_type | primitive_type | type_instance`
    fn primary_type(&mut self) -> Result<ast::TypeExpr> {
        push_parse_call!(self, "primary_type");
        self.view_type()
            .map_inner(ast::TypeExpr::View)
            .or_else(|| self.fun_type().map_inner(ast::TypeExpr::FunPtr))
            .or_else(|| {
                self.primitive_type()
                    .map(|x| Ok(ast::TypeExpr::Primitive(x)))
            })
            .unwrap_or_else(|| self.type_instance().map(ast::TypeExpr::Instance))
            .inspect_always(|x| pop_parse_call!(self, "primary_type", x))
    }

    /// `fun_type -> "fun" "(" ( type_expression ( "," type_expression )* ( "," )? )? ")" "->" type_expression`
    fn fun_type(&mut self) -> Option<Result<ast::FunType>> {
        push_parse_call!(self, "fun_type");
        self.pull_if_eq(Keyword::Fun)
            .map(|kw| {
                let (open, args, close) = self
                    .pull_if_delim(Bracket::Paren, |parser| {
                        parser.pull_terminated(Punctuation::Comma, |parser| {
                            parser.is_next_not_close().then(|| parser.type_expression())
                        })
                    })
                    .ok_or_else_flat(|| {
                        self.error(ExpectedToken::Verbatim(TokenKind::Punc(
                            Punctuation::LParen,
                        )))
                    })?;
                Ok(ast::FunType {
                    kw,
                    open,
                    params: args,
                    close,
                    ret: self.fun_ret().transpose()?.map(Box::new),
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "fun_type", x))
    }

    /// `fun_ret -> "->" type_expression`
    fn fun_ret(&mut self) -> Option<Result<ast::FunRet>> {
        push_parse_call!(self, "fun_ret");
        self.pull_if_eq(Punctuation::Arrow)
            .map(|arrow| {
                Ok(ast::FunRet {
                    arrow,
                    ty: self.type_expression()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "fun_ret", x))
    }

    /// `grouped_type -> "<" type_expression ">"`
    fn grouped_type(&mut self) -> Option<Result<ast::GroupedType>> {
        push_parse_call!(self, "grouped_type");
        self.pull_if_delim(Bracket::Angle, Self::type_expression)
            .map_inner(|(open, inner, close)| ast::GroupedType { open, inner, close })
            .inspect_always(|x| pop_parse_call!(self, "grouped_type", x))
    }

    /// `view_type -> "[" type_expression ( ";" expression )? "]"`
    fn view_type(&mut self) -> Option<Result<ast::ViewType>> {
        push_parse_call!(self, "view_type");
        self.pull_if_delim(Bracket::Brack, |parser| {
            let item = parser.type_expression()?;
            let count = parser
                .pull_if_eq(Punctuation::Semi)
                .map(|semi| {
                    Ok(ast::ArrCount {
                        semi,
                        count: parser.expression()?,
                    })
                })
                .transpose()?;
            Ok((item, count))
        })
        .map_inner(|(open, (item, count), close)| ast::ViewType {
            open,
            item: Box::new(item),
            count,
            close,
        })
        .inspect_always(|x| pop_parse_call!(self, "view_type", x))
    }

    /// `primitive_type -> "none" | "nevr" | "fail" | "bool" | "byte" | "uint" | "sint" | "flpt" | "char" | "text"`
    fn primitive_type(&mut self) -> Option<ast::PrimitiveType> {
        push_parse_call!(self, "primitive_type");
        self.pull_into()
            .map(|(span, ty)| ast::PrimitiveType { ty, span })
            .inspect_always(|x| pop_parse_call!(self, "primitive_type", x))
    }

    /// `type_instance -> typename ( generic_group )?`
    fn type_instance(&mut self) -> Result<ast::TypeInstance> {
        push_parse_call!(self, "type_instance");
        self.typename()
            .ok_or_else_flat(|| self.error(ExpectedToken::Typename))
            .and_then(|name| {
                Ok(ast::TypeInstance {
                    name,
                    args: self.generic_group().transpose()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "type_instance", x))
    }

    /// `typename -> ( ( TYPENAME_KEYWORD | IDENTIFIER ) "::" )* ( TYPENAME_KEYWORD | IDENTIFIER )`
    fn typename(&mut self) -> Option<Result<ast::Typename>> {
        push_parse_call!(self, "typename");
        self.pull_if_eq(TokenKind::Ident)
            .map(|mut name| {
                let mut path = Vec::new();
                while let Some(sep) = self.pull_if_eq(Punctuation::PathSep) {
                    path.push(ast::NamespacePathPart { name, sep });
                    name = self.try_pull_eq(TokenKind::Ident)?; // TODO: ExpectedToken::TypeExpr
                }
                Ok(ast::Typename { path, name })
            })
            .inspect_always(|x| pop_parse_call!(self, "typename", x))
    }

    /// `iff_clause -> "<=>" type_bound`
    fn iff_clause(&mut self) -> Option<Result<ast::IffClause>> {
        push_parse_call!(self, "iff_clause");
        self.pull_if_eq(Punctuation::Iff)
            .map(|iff| {
                Ok(ast::IffClause {
                    iff,
                    bound: self.type_bound()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "iff_clause", x))
    }

    /// `type_bound -> or_bound`
    fn type_bound(&mut self) -> Result<ast::TypeBound> {
        push_parse_call!(self, "type_bound");
        self.or_bound()
            .inspect_always(|x| pop_parse_call!(self, "type_bound", x))
    }

    /// `or_bound -> xor_bound ( "|" xor_bound )*`
    fn or_bound(&mut self) -> Result<ast::TypeBound> {
        push_parse_call!(self, "or_bound");
        self.xor_bound()
            .and_then(|mut bound| {
                while let Some(sep_span) = self.pull_if_eq(Punctuation::Or) {
                    let rhs = self.xor_bound()?;
                    bound = ast::TypeBound::Combo(Box::new(ast::ComboBound {
                        lhs: bound,
                        sep: Combinator::Any,
                        sep_span,
                        rhs,
                    }));
                }
                Ok(bound)
            })
            .inspect_always(|x| pop_parse_call!(self, "or_bound", x))
    }

    /// `xor_bound -> and_bound ( "/" and_bound )*`
    fn xor_bound(&mut self) -> Result<ast::TypeBound> {
        push_parse_call!(self, "xor_bound");
        self.and_bound()
            .and_then(|mut bound| {
                while let Some(sep_span) = self.pull_if_eq(Punctuation::Div) {
                    let rhs = self.and_bound()?;
                    bound = ast::TypeBound::Combo(Box::new(ast::ComboBound {
                        lhs: bound,
                        sep: Combinator::One,
                        sep_span,
                        rhs,
                    }));
                }
                Ok(bound)
            })
            .inspect_always(|x| pop_parse_call!(self, "xor_bound", x))
    }

    /// `and_bound -> not_bound ( "+" not_bound )*`
    fn and_bound(&mut self) -> Result<ast::TypeBound> {
        push_parse_call!(self, "and_bound");
        self.not_bound()
            .and_then(|mut bound| {
                while let Some(sep_span) = self.pull_if_eq(Punctuation::Add) {
                    let rhs = self.not_bound()?;
                    bound = ast::TypeBound::Combo(Box::new(ast::ComboBound {
                        lhs: bound,
                        sep: Combinator::All,
                        sep_span,
                        rhs,
                    }));
                }
                Ok(bound)
            })
            .inspect_always(|x| pop_parse_call!(self, "and_bound", x))
    }

    /// `not_bound -> ( "!" )* primary_bound`
    fn not_bound(&mut self) -> Result<ast::TypeBound> {
        push_parse_call!(self, "not_bound");
        if let Some(not) = self.pull_if_eq(Punctuation::Not) {
            self.not_bound()
                .map(|rhs| ast::TypeBound::Not(Box::new(ast::NegativeBound { not, rhs })))
        } else {
            self.primary_bound()
        }
        .inspect_always(|x| pop_parse_call!(self, "not_bound", x))
    }

    /// `primary_bound -> exact_bound | member_bound | shape_bound | expr_bound | grouped_bound`
    fn primary_bound(&mut self) -> Result<ast::TypeBound> {
        push_parse_call!(self, "primary_bound");
        self.exact_bound()
            .map_inner(|x| ast::TypeBound::Exact(Box::new(x)))
            .or_else(|| {
                self.has_bound()
                    .map_inner(|x| ast::TypeBound::Has(Box::new(x)))
            })
            .or_else(|| {
                self.shape_bound()
                    .map_inner(|x| ast::TypeBound::Shape(Box::new(x)))
            })
            .or_else(|| {
                self.satisfies_bound()
                    .map_inner(|x| ast::TypeBound::Satisfies(Box::new(x)))
            })
            .or_else(|| {
                self.grouped_bound()
                    .map_inner(|x| ast::TypeBound::Group(Box::new(x)))
            })
            .ok_or_else_flat(|| self.error(ExpectedToken::TypeBound))
            .inspect_always(|x| pop_parse_call!(self, "primary_bound", x))
    }

    /// `exact_bound -> "only" type_expression`
    fn exact_bound(&mut self) -> Option<Result<ast::ExactTypeBound>> {
        push_parse_call!(self, "exact_bound");
        self.pull_if_eq(Keyword::Only)
            .map(|kw| {
                Ok(ast::ExactTypeBound {
                    kw,
                    ty: self.type_expression()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "exact_bound", x))
    }

    /// `member_bound -> "has" IDENTIFIER ( colon_bound )?`
    fn has_bound(&mut self) -> Option<Result<ast::MemberBound>> {
        push_parse_call!(self, "has_bound");
        self.pull_if_eq(Keyword::Has)
            .map(|kw| {
                Ok(ast::MemberBound {
                    kw,
                    member: self.try_pull_eq(TokenKind::Ident)?,
                    req: self.colon_bound().transpose()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "has_bound", x))
    }

    /// `type_bound -> ":" type_bound`
    fn colon_bound(&mut self) -> Option<Result<ast::ColonBound>> {
        push_parse_call!(self, "colon_bound");
        self.pull_if_eq(Punctuation::Colon)
            .map(|colon| {
                Ok(ast::ColonBound {
                    colon,
                    bound: self.type_bound()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "colon_bound", x))
    }

    /// `shape_bound -> "fun" "(" ( type_expression ( "," type_expression )* ( "," )? )? ")" ( shape_bound_ret )?`
    fn shape_bound(&mut self) -> Option<Result<ast::ShapeBound>> {
        push_parse_call!(self, "shape_bound");
        self.pull_if_eq(Keyword::Fun)
            .map(|kw| {
                let (open, args, close) = self
                    .pull_if_delim(Bracket::Paren, |parser| {
                        parser.pull_terminated(Punctuation::Comma, |parser| {
                            Some(parser.type_expression())
                        })
                    })
                    .ok_or_else_flat(|| {
                        self.error(ExpectedToken::Verbatim(TokenKind::Punc(
                            Punctuation::LParen,
                        )))
                    })?;
                Ok(ast::ShapeBound {
                    kw,
                    open,
                    args,
                    close,
                    ret: self.shape_bound_ret().transpose()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "shape_bound", x))
    }

    /// `shape_bound_ret -> "->" type_expression`
    fn shape_bound_ret(&mut self) -> Option<Result<ast::ShapeBoundRet>> {
        push_parse_call!(self, "shape_bound_ret");
        self.pull_if_eq(Punctuation::Arrow)
            .map(|arrow| {
                Ok(ast::ShapeBoundRet {
                    arrow,
                    ty: self.type_expression()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "shape_bound_ret", x))
    }

    /// `expr_bound -> "for" IDENTIFIER ":" expression`
    fn satisfies_bound(&mut self) -> Option<Result<ast::ExprBound>> {
        push_parse_call!(self, "satisfies_bound");
        self.pull_if_eq(Keyword::For)
            .map(|kw| {
                Ok(ast::ExprBound {
                    kw,
                    name: self.try_pull_eq(TokenKind::Ident)?,
                    colon: self.try_pull_eq(Punctuation::Colon)?,
                    expr: self.expression()?,
                })
            })
            .inspect_always(|x| pop_parse_call!(self, "satisfies_bound", x))
    }

    /// `grouped_bound -> "(" type_bound ")"`
    fn grouped_bound(&mut self) -> Option<Result<ast::GroupedBound>> {
        push_parse_call!(self, "grouped_bound");
        self.pull_if_delim(Bracket::Paren, Self::type_bound)
            .map_inner(|(open, inner, close)| ast::GroupedBound { open, inner, close })
            .inspect_always(|x| pop_parse_call!(self, "grouped_bound", x))
    }
}
