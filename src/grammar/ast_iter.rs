use crate::{
    error::{ContextError, OpSide},
    grammar::{Binary, Expr, Grouping, OrType, TypeExpr, Unary},
    highlight::{TokenHighlight, syntax::Syntax},
    scanner::{
        Scanner,
        token::{Token, punc::Punctuation, value::LexValue},
    },
};

#[derive(Debug, Clone)]
pub enum ExprOrToken<'src, 'expr> {
    Token(&'expr Token<'src>),
    Semantic(&'expr Token<'src>, Syntax),
    Bracket(&'expr Token<'src>, usize),
    Expr(&'expr Expr<'src>, usize),
}

#[derive(Debug, Clone)]
pub enum ExprIterKind<'src, 'expr> {
    Binary {
        lhs: Option<&'expr Expr<'src>>,
        op: Option<&'expr Token<'src>>,
        rhs: Option<&'expr Expr<'src>>,
    },
    UnaryPre {
        op: Option<&'expr Token<'src>>,
        rhs: Option<&'expr Expr<'src>>,
    },
    UnaryPost {
        lhs: Option<&'expr Expr<'src>>,
        op: Option<&'expr Token<'src>>,
    },
    Literal {
        token: Option<&'expr Token<'src>>,
    },
    Grouping {
        open: Option<&'expr Token<'src>>,
        expr: Option<&'expr Expr<'src>>,
        close: Option<&'expr Token<'src>>,
    },
    Type {
        name: Option<&'expr Token<'src>>,
        pipe: Option<&'expr Token<'src>>,
        or_ty: Option<&'expr Token<'src>>,
    },
}

#[derive(Debug, Clone)]
pub struct ExprIter<'src, 'expr> {
    pub kind: ExprIterKind<'src, 'expr>,
    pub depth: usize,
}

impl<'src, 'expr> Iterator for ExprIter<'src, 'expr> {
    type Item = ExprOrToken<'src, 'expr>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.kind {
            ExprIterKind::Binary { lhs, op, rhs } => lhs
                .take()
                .map(|x| ExprOrToken::Expr(x, self.depth))
                .or_else(|| {
                    op.take().map(|token| match token.val {
                        LexValue::Punctuation(Punctuation::Convert | Punctuation::Transmute) => {
                            ExprOrToken::Semantic(token, Syntax::Keyword)
                        }
                        _ => ExprOrToken::Token(token),
                    })
                })
                .or_else(|| rhs.take().map(|x| ExprOrToken::Expr(x, self.depth))),

            ExprIterKind::UnaryPre { op, rhs } => op
                .take()
                .map(ExprOrToken::Token)
                .or_else(|| rhs.take().map(|x| ExprOrToken::Expr(x, self.depth))),

            ExprIterKind::UnaryPost { lhs, op } => lhs
                .take()
                .map(|x| ExprOrToken::Expr(x, self.depth))
                .or_else(|| {
                    op.take().map(|token| match token.val {
                        LexValue::Punctuation(Punctuation::Coalesce) => {
                            ExprOrToken::Semantic(token, Syntax::CtrlKeyword)
                        }
                        _ => ExprOrToken::Token(token),
                    })
                }),

            ExprIterKind::Literal { token } => token.take().map(ExprOrToken::Token),

            ExprIterKind::Grouping { open, expr, close } => open
                .take()
                // TODO: where do we get the depth from?
                .map(|x| ExprOrToken::Bracket(x, self.depth))
                .or_else(|| {
                    expr.take()
                        .map(|x| ExprOrToken::Expr(x, self.depth.strict_add(1)))
                })
                // TODO: where do we get the depth from?
                .or_else(|| close.take().map(|x| ExprOrToken::Bracket(x, self.depth))),

            ExprIterKind::Type { name, pipe, or_ty } => name
                .take()
                .map(|x| ExprOrToken::Semantic(x, Syntax::Typename))
                .or_else(|| {
                    pipe.take()
                        .map(|x| ExprOrToken::Semantic(x, Syntax::Keyword))
                })
                .or_else(|| {
                    or_ty
                        .take()
                        .map(|x| ExprOrToken::Semantic(x, Syntax::Typename))
                }),
        }
    }
}

impl<'src> Expr<'src> {
    fn iter(&self, depth: usize) -> ExprIter<'src, '_> {
        match self {
            Expr::Binary(inner) => {
                let Binary { lhs, op, rhs } = &**inner;
                ExprIter {
                    kind: ExprIterKind::Binary {
                        lhs: Some(lhs),
                        op: Some(op),
                        rhs: Some(rhs),
                    },
                    depth,
                }
            }

            Expr::Unary(inner) => {
                let Unary { op, operand, side } = &**inner;
                match side {
                    OpSide::Left => ExprIter {
                        kind: ExprIterKind::UnaryPost {
                            lhs: Some(operand),
                            op: Some(op),
                        },
                        depth,
                    },
                    OpSide::Right => ExprIter {
                        kind: ExprIterKind::UnaryPre {
                            op: Some(op),
                            rhs: Some(operand),
                        },
                        depth,
                    },
                }
            }

            Expr::Literal(token) => ExprIter {
                kind: ExprIterKind::Literal { token: Some(token) },
                depth,
            },

            Expr::Grouping(inner) => {
                let Grouping { open, expr, close } = &**inner;
                ExprIter {
                    kind: ExprIterKind::Grouping {
                        open: Some(open),
                        expr: Some(expr),
                        close: Some(close),
                    },
                    depth,
                }
            }

            Expr::Type(TypeExpr { name, or_ty }) => ExprIter {
                kind: ExprIterKind::Type {
                    name: Some(name),
                    pipe: or_ty.as_ref().map(|OrType { pipe, .. }| pipe),
                    or_ty: or_ty.as_ref().map(|OrType { ty, .. }| ty),
                },
                depth,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenSemantics {
    Override(Syntax),
    Bracket(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticToken<'src> {
    token: Token<'src>,
    sem: Option<TokenSemantics>,
}

impl<'src> TokenHighlight<'src> for SemanticToken<'src> {
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>) {
        let Token { lex, val, .. } = self.token;
        let syn = match self.sem {
            Some(TokenSemantics::Override(syn)) => syn,
            Some(TokenSemantics::Bracket(depth)) => Syntax::Bracket(depth),
            None => self.token.syntax(),
        };
        (lex, syn, val)
    }
}

/// Traverse the AST using DFS
#[derive(Debug, Clone)]
pub struct AstIter<'src, 'expr> {
    // TODO: can the stack be eliminated somehow?
    stack: Vec<ExprIter<'src, 'expr>>,
}

impl<'src, 'expr> AstIter<'src, 'expr> {
    pub fn new(ast: &'expr Expr<'src>) -> Self {
        Self {
            stack: vec![ast.iter(0)],
        }
    }
}

impl<'src> Iterator for AstIter<'src, '_> {
    type Item = SemanticToken<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.stack.last_mut() {
            if let Some(item) = top.next() {
                match item {
                    ExprOrToken::Token(&token) => return Some(SemanticToken { token, sem: None }),
                    ExprOrToken::Semantic(&token, syn) => {
                        return Some(SemanticToken {
                            token,
                            sem: Some(TokenSemantics::Override(syn)),
                        });
                    }
                    ExprOrToken::Bracket(&token, depth) => {
                        return Some(SemanticToken {
                            token,
                            sem: Some(TokenSemantics::Bracket(depth)),
                        });
                    }
                    ExprOrToken::Expr(expr, depth) => self.stack.push(expr.iter(depth)),
                }
            } else {
                self.stack.pop();
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct SemanticIter<'src, I: Iterator<Item = SemanticToken<'src>>> {
    source: &'src str,
    prev_end: usize,
    iter: std::iter::Peekable<I>,
}

impl<'src, I: Iterator<Item = SemanticToken<'src>>> SemanticIter<'src, I> {
    fn new(source: &'src str, iter: I) -> Self {
        Self {
            source,
            prev_end: 0,
            iter: iter.peekable(),
        }
    }
}

pub type SemanticScanner<'src> = std::iter::Map<
    Scanner<'src>,
    fn(Result<Token<'src>, ContextError<'src>>) -> SemanticToken<'src>,
>;

#[derive(Debug, Clone)]
pub enum SemanticIterInner<'src> {
    Skipped,
    Lex(SemanticScanner<'src>),
    Sem(std::iter::Once<SemanticToken<'src>>),
}

impl<'src> Iterator for SemanticIterInner<'src> {
    type Item = SemanticToken<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            // HACK
            Self::Skipped => None,
            Self::Lex(iter) => iter.next(),
            Self::Sem(iter) => iter.next(),
        }
    }
}

impl<'src, I> Iterator for SemanticIter<'src, I>
where
    I: Iterator<Item = SemanticToken<'src>>,
{
    type Item = SemanticIterInner<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        fn scanner_to_semantic<'src>(
            res: Result<Token<'src>, ContextError<'src>>,
        ) -> SemanticToken<'src> {
            SemanticToken {
                token: res.expect("should not have any lex errors if an AST exists"),
                sem: None,
            }
        }

        self.iter
            .next_if_map(|item| {
                if let Some(macro_range) = item.token.mac {
                    if macro_range.start == self.prev_end {
                        self.prev_end = macro_range.end;
                        let lex = self
                            .source
                            .get(macro_range)
                            .expect("macro_range should be a valid range in source");
                        Ok(SemanticIterInner::Lex(
                            Scanner::new_subset(self.source, lex, true).map(scanner_to_semantic),
                        ))
                    } else if macro_range.end <= self.prev_end {
                        // TODO: can this be used for semantics within macros?
                        if false {
                            let token_range = item.token.lex_range(self.source);
                            let is_from_argument = macro_range.start <= token_range.start
                                && token_range.end <= macro_range.end;
                            println!(
                                "{macro_range:?} ({token_range:?}): {:?} - {}",
                                item.token,
                                if is_from_argument {
                                    "from argument"
                                } else {
                                    "from definition"
                                }
                            );
                        }
                        // we are within the same macro
                        Ok(SemanticIterInner::Skipped)
                    } else {
                        Err(item)
                    }
                } else if item.token.lex_range(self.source).start == self.prev_end {
                    self.prev_end = item.token.lex_range(self.source).end;
                    Ok(SemanticIterInner::Sem(std::iter::once(item)))
                } else {
                    Err(item)
                }
            })
            .or_else(|| {
                (self.prev_end != self.source.len()).then(|| {
                    let next = self.iter.peek();
                    let next_start = next.map_or(self.source.len(), |item| {
                        item.token
                            .mac
                            .unwrap_or_else(|| item.token.lex_range(self.source))
                            .start
                    });
                    let lex = self
                        .source
                        .get(self.prev_end..next_start)
                        .unwrap_or_else(|| {
                            panic!(
                                "prev_end..next_start should be a valid substr range\n \
                                    prev_end: {} ({:?})\n \
                                    next_start: {} ({:?})\n \
                                    source.len(): {}\n \
                                    next token: {next:?}",
                                self.prev_end,
                                self.source
                                    .get(..self.prev_end)
                                    .and_then(|s| s.lines().next_back()),
                                next_start,
                                self.source.get(next_start..).and_then(|s| s.lines().next()),
                                self.source.len(),
                            );
                        });
                    self.prev_end = next_start;
                    SemanticIterInner::Lex(
                        Scanner::new_subset(self.source, lex, true).map(scanner_to_semantic),
                    )
                })
            })
    }
}

pub type Semantics<'src, 'expr, I> = std::iter::Flatten<
    SemanticIter<
        'src,
        std::iter::FlatMap<
            <I as IntoIterator>::IntoIter,
            AstIter<'src, 'expr>,
            fn(&'expr Expr<'src>) -> AstIter<'src, 'expr>,
        >,
    >,
>;

pub fn semantic<'src, 'expr, I>(source: &'src str, ast: I) -> Semantics<'src, 'expr, I>
where
    I: IntoIterator<Item = &'expr Expr<'src>>,
{
    SemanticIter::new(
        source,
        #[expect(clippy::as_conversions)]
        ast.into_iter()
            .flat_map(AstIter::new as fn(&'expr Expr<'src>) -> AstIter<'src, 'expr>),
    )
    .flatten()
}
