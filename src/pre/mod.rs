use crate::{
    Todo,
    error::{Error, ErrorKind, ExpectedToken},
    lex::token::{
        TokenKind,
        keyword::Keyword,
        punc::{BadBracketCombo, Bracket, Punctuation},
    },
    parse::util::MapInner,
    src_code::{
        code::{CreateSourceCodeError, SourceCode, SourceLoc},
        span::{FileId, RelSpan, Span, uspan},
    },
};
use std::collections::VecDeque;

mod ast;

pub fn preprocess(source: SourceCode, id: FileId) -> Result<SourceCode, CreateSourceCodeError> {
    let loc = match source.loc() {
        SourceLoc::Repl | SourceLoc::PreprocRepl => SourceLoc::PreprocRepl,

        SourceLoc::File(path) | SourceLoc::PreprocFile(path) => {
            SourceLoc::PreprocFile(path.clone())
        }
    };
    let len = source.content().len();
    debug_assert!(uspan::try_from(len).is_ok());
    let mut pp = PreParser::new(id, len as uspan, source.tokens().spans());
    let _ = pp.document();
    let src = todo!();
    let mac = todo!();
    let tokens = todo!();
    SourceCode::with_expansion(loc, src, mac).map(|mut code| {
        code.set_tokens(tokens);
        code
    })
}

/// Preprocessor environment
struct PreprocEnv {
    defs: Todo,
}

#[derive(Debug, Clone)]
struct PreParser {
    file: FileId,
    file_len: uspan,
    tokens: VecDeque<(RelSpan, TokenKind)>,
}

impl PreParser {
    pub fn new<I>(file: FileId, file_len: uspan, tokens: I) -> Self
    where
        I: IntoIterator<Item = (RelSpan, TokenKind)>,
    {
        Self {
            file,
            file_len,
            tokens: VecDeque::from_iter(tokens),
        }
    }

    fn peek_next(&self) -> Option<&(RelSpan, TokenKind)> {
        self.tokens.front()
    }

    fn pull_next(&mut self) -> Option<(RelSpan, TokenKind)> {
        self.tokens.pop_front()
    }

    fn pull_next_if<P>(&mut self, p: P) -> Option<(RelSpan, TokenKind)>
    where
        P: FnOnce(&(RelSpan, TokenKind)) -> bool,
    {
        self.tokens.pop_front_if(|x| p(x))
    }

    fn pull_next_if_eq<T>(&mut self, kind: T) -> Option<RelSpan>
    where
        T: Into<TokenKind>,
    {
        self.pull_next_if(|(_, k)| *k == kind.into())
            .map(|(span, _)| span)
    }

    fn peek(&self) -> Option<&(RelSpan, TokenKind)> {
        self.tokens.iter().find(|(_, k)| !k.is_ignored())
    }

    fn pull(&mut self) -> Option<(RelSpan, TokenKind)> {
        std::iter::from_fn(|| self.tokens.pop_front()).find(|(_, k)| !k.is_ignored())
    }

    fn pull_if<P>(&mut self, p: P) -> Option<(RelSpan, TokenKind)>
    where
        P: FnOnce(&(RelSpan, TokenKind)) -> bool,
    {
        self.tokens
            .iter()
            .position(|(_, k)| !k.is_ignored())
            .filter(|&i| p(&self.tokens[i]))
            .map(|i| {
                std::iter::from_fn(|| self.tokens.pop_front())
                    .nth(i)
                    .expect("position should produce a valid index")
            })
    }

    fn pull_if_eq<T>(&mut self, kind: T) -> Option<RelSpan>
    where
        T: Into<TokenKind>,
    {
        self.pull_if(|(_, k)| *k == kind.into())
            .map(|(span, _)| span)
    }

    fn try_pull_next<P>(
        &mut self,
        p: P,
        expected: ExpectedToken,
    ) -> Result<(RelSpan, TokenKind), Error>
    where
        P: FnOnce(&(RelSpan, TokenKind)) -> bool,
    {
        self.pull_next_if(p).ok_or_else(|| self.error(expected))
    }

    fn try_pull_next_eq<T>(&mut self, kind: T) -> Result<RelSpan, Error>
    where
        T: Into<TokenKind>,
    {
        let kind = kind.into();
        self.pull_if_eq(kind)
            .ok_or_else(|| self.error(ExpectedToken::Verbatim(kind)))
    }

    fn try_pull<P>(&mut self, p: P, expected: ExpectedToken) -> Result<(RelSpan, TokenKind), Error>
    where
        P: FnOnce(&(RelSpan, TokenKind)) -> bool,
    {
        self.pull_if(p).ok_or_else(|| self.error(expected))
    }

    fn try_pull_eq<T>(&mut self, kind: T) -> Result<RelSpan, Error>
    where
        T: Into<TokenKind>,
    {
        let kind = kind.into();
        self.pull_if_eq(kind)
            .ok_or_else(|| self.error(ExpectedToken::Verbatim(kind)))
    }

    /// Does not consume the token
    fn error(&self, expected: ExpectedToken) -> Error {
        if let Some(&(span, _)) = self.peek() {
            Error::new(
                Span {
                    file: self.file,
                    rel: span,
                },
                ErrorKind::UnexpectedToken { expected },
            )
        } else {
            Error::new(
                Span {
                    file: self.file,
                    rel: (self.file_len..self.file_len).into(),
                },
                ErrorKind::MissingToken { expected },
            )
        }
    }

    // TBD: macro synchronize?

    #[inline]
    fn pull_if_delimited<R, F>(
        &mut self,
        brack: Bracket,
        inner: F,
    ) -> Option<Result<(RelSpan, R, RelSpan), Error>>
    where
        F: FnOnce(&mut Self) -> Result<R, Error>,
    {
        self.pull_if_eq(brack.open_punc()).map(|open| {
            let inner = inner(self)?;
            // HACK: copied from `parse` module
            let close = self.pull_if_eq(brack.close_punc()).ok_or_else(|| {
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
                            open_range: Span {
                                file: self.file,
                                rel: open,
                            },
                            failure: BadBracketCombo::new(brack, close)
                                .expect("matching brackets should not produce an error"),
                        }
                    };
                    Error::new(
                        Span {
                            file: self.file,
                            rel: span,
                        },
                        kind,
                    )
                } else {
                    Error::new(
                        Span {
                            file: self.file,
                            rel: (self.file_len..self.file_len).into(),
                        },
                        ErrorKind::MissingCloseBracket {
                            open_range: Span {
                                file: self.file,
                                rel: open,
                            },
                            expect: brack,
                        },
                    )
                }
            })?;
            Ok((open, inner, close))
        })
    }
}

impl PreParser {
    /// `document -> ( document_item )*`
    fn document(&mut self) -> Result<ast::Document, impl '_ + IntoIterator<Item = Error>> {
        let mut items = Vec::new();
        let mut iter =
            std::iter::from_fn(|| self.peek_next().is_some().then(|| self.document_item()));
        while let Some(res) = iter.next() {
            match res {
                Ok(x) => items.push(x),
                Err(e) => return Err(std::iter::once(e).chain(iter.filter_map(Result::err))),
            }
        }
        Ok(ast::Document { items })
    }

    /// `document_item -> definition | tokens`
    fn document_item(&mut self) -> Result<ast::DocumentItem, Error> {
        self.definition()
            .map_inner(ast::DocumentItem::Def)
            .unwrap_or_else(|| {
                Ok(ast::DocumentItem::Other(self.tokens_while(|(_, kind)| {
                    *kind != TokenKind::Keyword(Keyword::Def)
                })))
            })
    }

    /// `definition -> "def" MACRO ( params )? replacer`
    fn definition(&mut self) -> Option<Result<ast::Definition, Error>> {
        self.pull_next_if_eq(Keyword::Def).map(|kw| {
            Ok(ast::Definition {
                kw,
                name: self.try_pull_eq(TokenKind::Macro)?,
                params: self.params().transpose()?,
                body: self
                    .replacer()
                    .unwrap_or_else(|| Err(self.error(ExpectedToken::MacroDef)))?,
            })
        })
    }

    /// `params -> "(" ( MPARAM )* ")"`
    fn params(&mut self) -> Option<Result<ast::ParamGroup, Error>> {
        self.pull_if_delimited(Bracket::Paren, |pp| {
            Ok(std::iter::from_fn(|| pp.pull_if_eq(TokenKind::MParam)).collect())
        })
        .map_inner(|(open, params, close)| ast::ParamGroup {
            open,
            params,
            close,
        })
    }

    /// `replacer -> "{" tokens "}"`
    fn replacer(&mut self) -> Option<Result<ast::Replacer, Error>> {
        self.pull_if_delimited(Bracket::Brace, |pp| {
            Ok(pp.tokens_while({
                let mut depth: usize = 0;
                move |(_, kind)| {
                    match kind {
                        TokenKind::Punc(Punctuation::LBrace) => depth = depth.strict_add(1),

                        TokenKind::Punc(Punctuation::RBrace) => {
                            if let Some(n) = depth.checked_sub(1) {
                                depth = n;
                            } else {
                                return false;
                            }
                        }

                        _ => {}
                    }
                    true
                }
            }))
        })
        .map_inner(|(open, items, close)| ast::Replacer { open, items, close })
    }

    fn tokens_while<P>(&mut self, mut p: P) -> ast::Tokens
    where
        P: FnMut(&(RelSpan, TokenKind)) -> bool,
    {
        let mut spans = Vec::new();
        let mut tokens = std::iter::from_fn(|| self.pull_next_if(&mut p)).map(|(span, _)| span);
        if let Some(first) = tokens.next() {
            let mut curr = spans.push_mut(first);
            while let Some(next) = tokens.next() {
                if curr.end == next.start {
                    curr.end = next.end;
                } else {
                    curr = spans.push_mut(next);
                }
            }
        }
        ast::Tokens { spans }
    }
}
