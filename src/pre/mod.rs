use crate::{
    error::{Error, ErrorKind, ExpectedToken},
    lex::{
        lex,
        token::{
            TokenKind,
            keyword::Keyword,
            punc::{BadBracketCombo, Bracket, Punctuation},
        },
    },
    parse::util::MapInner,
    src_code::{
        code::{ExpansionData, SourceCode, SourceLoc},
        map::SourceMap,
        span::{FileId, RelSpan, Span, usize_to_uspan, uspan},
    },
};
use std::collections::{HashMap, VecDeque};

mod ast;
mod eval;

pub fn preprocess(
    id: FileId,
    code: &SourceCode,
    source: &SourceMap,
) -> Result<SourceCode, Vec<Error>> {
    let loc = match code.loc() {
        SourceLoc::Repl | SourceLoc::PreprocRepl => SourceLoc::PreprocRepl,

        SourceLoc::File(path) | SourceLoc::PreprocFile(path) => {
            SourceLoc::PreprocFile(path.clone())
        }
    };
    let file_len = usize_to_uspan(code.content().len());
    let (src, mac, errs) = PreProcessor::new(source, id).preproc();
    if errs.is_empty() {
        let (tokens, errs) = lex(id, src.as_str());
        if errs.is_empty() {
            match SourceCode::with_expansion(loc, src, mac) {
                Ok(mut code) => {
                    code.set_tokens(tokens);
                    Ok(code)
                }
                Err(e) => Err(vec![Error::new(
                    RelSpan::from(file_len..file_len).within_file(id),
                    ErrorKind::OversizePreproc(e),
                )]),
            }
        } else {
            Err(errs)
        }
    } else {
        Err(if let Err(e) = SourceCode::with_expansion(loc, src, mac) {
            vec![Error::new(
                RelSpan::from(file_len..file_len).within_file(id),
                ErrorKind::OversizePreproc(e),
            )]
        } else {
            errs
        })
    }
}

#[derive(Debug, Clone)]
struct PreProcessor<'src> {
    file: FileId,
    file_len: uspan,
    tokens: VecDeque<(Span, TokenKind)>,
    source: &'src SourceMap,
    definitions: HashMap<&'src str, MacroDef>,
    expansions: Vec<ExpansionData>,
}

impl<'src> PreProcessor<'src> {
    pub fn new(source: &'src SourceMap, file: FileId) -> Self {
        let code = &source[file];
        Self {
            file,
            file_len: code.len(),
            tokens: VecDeque::from_iter(
                code.tokens()
                    .spans()
                    .map(|(rel, kind)| (rel.within_file(file), kind)),
            ),
            source,
            definitions: HashMap::new(),
            expansions: Vec::new(),
        }
    }

    fn peek_next(&self) -> Option<&(Span, TokenKind)> {
        self.tokens.front()
    }

    fn pull_next(&mut self) -> Option<(Span, TokenKind)> {
        self.tokens.pop_front()
    }

    fn pull_next_if<P>(&mut self, p: P) -> Option<(Span, TokenKind)>
    where
        P: FnOnce(&(Span, TokenKind)) -> bool,
    {
        // can't use `p` directly because Rust isn't smart enough to coerce `FnOnce(&mut T) -> U` into `FnOnce(&T) -> U` :/
        self.tokens.pop_front_if(|x| p(x))
    }

    fn pull_next_if_eq<T>(&mut self, kind: T) -> Option<Span>
    where
        T: Into<TokenKind>,
    {
        self.pull_next_if(|(_, k)| *k == kind.into())
            .map(|(span, _)| span)
    }

    fn peek(&self) -> Option<&(Span, TokenKind)> {
        self.tokens.iter().find(|(_, k)| !k.is_ignored())
    }

    fn pull(&mut self) -> Option<(Span, TokenKind)> {
        std::iter::from_fn(|| self.tokens.pop_front()).find(|(_, k)| !k.is_ignored())
    }

    fn pull_if<P>(&mut self, p: P) -> Option<(Span, TokenKind)>
    where
        P: FnOnce(&(Span, TokenKind)) -> bool,
    {
        self.tokens
            .iter()
            .position(|(_, k)| !k.is_ignored())
            .filter(|&i| p(&self.tokens[i]))
            .map(|i| {
                drop(self.tokens.drain(..i));
                self.tokens
                    .pop_front()
                    .expect("position should produce a valid index")
            })
    }

    fn pull_if_eq<T>(&mut self, kind: T) -> Option<Span>
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
    ) -> Result<(Span, TokenKind), Error>
    where
        P: FnOnce(&(Span, TokenKind)) -> bool,
    {
        self.pull_next_if(p).ok_or_else(|| self.error(expected))
    }

    fn try_pull_next_eq<T>(&mut self, kind: T) -> Result<Span, Error>
    where
        T: Into<TokenKind>,
    {
        let kind = kind.into();
        self.pull_if_eq(kind)
            .ok_or_else(|| self.error(ExpectedToken::Verbatim(kind)))
    }

    fn try_pull<P>(&mut self, p: P, expected: ExpectedToken) -> Result<(Span, TokenKind), Error>
    where
        P: FnOnce(&(Span, TokenKind)) -> bool,
    {
        self.pull_if(p).ok_or_else(|| self.error(expected))
    }

    fn try_pull_eq<T>(&mut self, kind: T) -> Result<Span, Error>
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
            Error::new(span, ErrorKind::UnexpectedToken { expected })
        } else {
            Error::new(
                RelSpan::from(self.file_len..self.file_len).within_file(self.file),
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
    ) -> Option<Result<(Span, R, Span), Error>>
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
                            open_range: open,
                            failure: BadBracketCombo::new(brack, close)
                                .expect("matching brackets should not produce an error"),
                        }
                    };
                    Error::new(span, kind)
                } else {
                    Error::new(
                        Span {
                            file: self.file,
                            rel: (self.file_len..self.file_len).into(),
                        },
                        ErrorKind::MissingCloseBracket {
                            open_range: open,
                            expect: brack,
                        },
                    )
                }
            })?;
            Ok((open, inner, close))
        })
    }
}

// TODO: cross-file macros?
// TBD: what is the ExpansionData of an intermediate expansion?

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
struct Substitution<'arg> {
    param: Span,
    arg: &'arg ast::Tokens,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct MacroDef {
    pub name_span: Span,
    pub params: Vec<Span>,
    pub body: Vec<(Span, TokenKind)>,
}

impl<'src> PreProcessor<'src> {
    fn preproc(mut self) -> (String, Vec<ExpansionData>, Vec<Error>) {
        let mut preprocessed = String::new();
        let mut errors = Vec::new();
        while self.peek_next().is_some() {
            let res = self.document_item();
            match res {
                Ok(item) => match item {
                    ast::DocumentItem::Def(def) => {
                        let key = self.source.span_str(def.name);
                        let params = def.params.map_or_default(|group| group.params);
                        let body = def.body.items.list;
                        _ = self.definitions.insert(
                            key,
                            MacroDef {
                                name_span: def.name,
                                params,
                                body,
                            },
                        );
                    }

                    ast::DocumentItem::Call(call) => {
                        if let Some(def) = self.definitions.get(self.source.span_str(call.name)) {
                            let subs = def
                                .params
                                .iter()
                                .zip(call.args.iter())
                                .map(|(&param, arg)| {
                                    (
                                        self.source.span_str(param),
                                        Substitution {
                                            param,
                                            arg: &arg.items,
                                        },
                                    )
                                })
                                .collect::<HashMap<&'src str, _>>();

                            let mut expansion = Vec::new();
                            for &item @ (span, kind) in &def.body {
                                if matches!(kind, TokenKind::MParam)
                                    && let Some(sub) = subs.get(self.source.span_str(span))
                                {
                                    expansion.extend_from_slice(&sub.arg.list);
                                } else {
                                    // ignore other macro params, they might belong to an inner definition.
                                    // they will error if they are left behind after all calls have expanded.

                                    expansion.push(item);

                                    // note: the case `def \foo($x) { def \bar($x) { ... } }`
                                    // should substitute `\bar`'s parameter with `\foo`'s argument.
                                    // e.g. `\foo{7}` INTENTIONALLY expands to `def \bar(7) { ... }`.
                                }
                            }

                            // TODO: expansion data
                            self.tokens.prepend(expansion);
                        } else {
                            errors.push(Error::new(call.name, ErrorKind::MacroUndefined));
                        }
                    }

                    ast::DocumentItem::Other(tokens) => {
                        for (span, _) in tokens.list {
                            preprocessed.push_str(self.source.span_str(span));
                        }
                    }
                },
                // TODO: what if the range is within a partially-processed file?
                Err(e) => errors.push(e),
            }
        }
        (preprocessed, self.expansions, errors)
    }

    /// `document_item -> definition | call | tokens`
    fn document_item(&mut self) -> Result<ast::DocumentItem, Error> {
        self.definition()
            .map_inner(ast::DocumentItem::Def)
            .or_else(|| self.call().map_inner(ast::DocumentItem::Call))
            .unwrap_or_else(|| {
                Ok(ast::DocumentItem::Other(self.tokens_while(|(_, kind)| {
                    !matches!(kind, TokenKind::Keyword(Keyword::Def) | TokenKind::Macro)
                })))
            })
    }

    /// `call -> MACRO ( replacer )*`
    fn call(&mut self) -> Option<Result<ast::Call, Error>> {
        self.pull_next_if_eq(TokenKind::Macro).map(|name| {
            let expected_args = self
                .definitions
                .get(self.source.span_str(name))
                .map_or_default(|def| def.params.len());
            let args = (0..expected_args)
                .map(|_| {
                    self.replacer()
                        .ok_or_else(|| self.error(ExpectedToken::MacroArg))
                        .flatten()
                })
                .collect::<Result<_, _>>()?;
            Ok(ast::Call { name, args })
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
        P: FnMut(&(Span, TokenKind)) -> bool,
    {
        ast::Tokens {
            list: std::iter::from_fn(|| self.pull_next_if(&mut p)).collect(),
        }
    }
}
