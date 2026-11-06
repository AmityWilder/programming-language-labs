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
        code::{CreateSourceCodeError, ExpansionData, ExpansionField, SourceCode, SourceLoc},
        map::SourceMap,
        span::{FileId, RelSpan, Span, usize_to_uspan, uspan, uspan_to_usize},
    },
};
use std::{
    collections::{HashMap, VecDeque},
    range::Range,
};

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
    // TBD: what if ExpansionCluter is the better structure?
    let mac = mac
        .into_iter()
        .flat_map(|x| {
            let call = x.call;
            let def = x.def;
            std::iter::once(ExpansionData {
                exp: x.exp,
                call,
                def,
                field: None,
            })
            .chain(x.args.into_iter().map(move |arg| ExpansionData {
                exp: arg.exp,
                call,
                def,
                field: Some(ExpansionField {
                    arg: arg.arg,
                    param: arg.param,
                }),
            }))
        })
        .collect();
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct ArgExpansion {
    /// Where the argument expanded into
    exp: RelSpan,
    /// The argument that was expanded
    arg: Span,
    /// The parameter of the argument
    param: Span,
}

/// Multiple expansions in one (grouping their fields)
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct ExpansionCluster {
    /// The definition location
    def: Span,
    /// The original byte-range of the call (before it was stripped)
    call: Span,
    /// The total byte-range of the overall expansion
    exp: RelSpan,
    /// The expansions that came from arguments, and which argument they came from.
    /// Arguments cannot have overlapping expansions, so only the most recent one needs to be updated.
    args: Vec<ArgExpansion>,
}

#[derive(Debug, Clone, PartialEq)]
struct PreProcessor<'src> {
    source: &'src SourceMap,

    /// Current macro definitions - keys are `str`s within [`Self::source`]
    definitions: HashMap<&'src str, MacroDef>,

    /// The stream of tokens to process
    tokens: VecDeque<(Span, TokenKind)>,

    /// Total len of each token consumed, including whitespace
    eaten: uspan,

    /// The ID of the file being processed (NOT the postprocessed file!)
    file: FileId,
    /// The location of the end of the file, for errors
    file_len: uspan,

    /// Expansions created in this preprocessor task
    expansions: Vec<ExpansionCluster>,

    /// The list of errors encountered so far - preprocessing has failed if this is non-empty
    // TODO: prevent cascading
    errors: Vec<Error>,

    /// The current preprocessor result - can only increase
    processed: String,
}

impl<'src> PreProcessor<'src> {
    pub fn new(source: &'src SourceMap, file: FileId) -> Self {
        let code = &source[file];
        Self {
            source,
            definitions: HashMap::new(),
            tokens: VecDeque::from_iter(
                code.tokens()
                    .iter()
                    .map(|(rel, kind)| (rel.within_file(file), kind)),
            ),
            eaten: 0,
            file,
            file_len: code.len(),
            expansions: Vec::new(),
            errors: Vec::new(),
            processed: String::new(),
        }
    }

    const fn track_munch(&mut self, span: Span) {
        self.eaten = self.eaten.strict_add(span.rel.len());
    }

    fn peek_next(&self) -> Option<&(Span, TokenKind)> {
        self.tokens.front()
    }

    fn pull_next_if<P>(&mut self, p: P) -> Option<(Span, TokenKind)>
    where
        P: FnOnce(&(Span, TokenKind)) -> bool,
    {
        // can't use `p` directly because Rust isn't smart enough to coerce `FnOnce(&mut T) -> U` into `FnOnce(&T) -> U` :/
        self.tokens
            .pop_front_if(|x| p(x))
            .inspect(|&(span, _)| self.track_munch(span))
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

    fn pull_if<P>(&mut self, p: P) -> Option<(Span, TokenKind)>
    where
        P: FnOnce(&(Span, TokenKind)) -> bool,
    {
        self.tokens
            .iter()
            .position(|(_, k)| !k.is_ignored())
            .filter(|&i| p(&self.tokens[i]))
            .map(|i| {
                for j in 0..i.strict_add(1) {
                    self.track_munch(self.tokens[j].0);
                }
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

    /// Minimize cascade errors
    fn sync(&mut self) {
        todo!()
    }

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

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct Substitution {
    param: Span,
    arg: ast::Tokens,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct MacroDef {
    pub name_span: Span,
    pub params: Vec<Span>,
    pub body: Vec<(Span, TokenKind)>,
}

fn sum_len<I>(start: uspan, extension: I) -> Result<RelSpan, CreateSourceCodeError>
where
    I: IntoIterator<Item = uspan>,
{
    extension
        .into_iter()
        .try_fold(start, |acc: uspan, len: uspan| {
            acc.checked_add(len)
                .ok_or_else(|| uspan_to_usize(acc).saturating_add(uspan_to_usize(len)))
        })
        .map_err(CreateSourceCodeError)
        .map(|end| (start..end).into())
}

impl PreProcessor<'_> {
    const fn processed_len(&self) -> uspan {
        usize_to_uspan(self.processed.len())
    }

    fn preproc(mut self) -> (String, Vec<ExpansionCluster>, Vec<Error>) {
        while self.peek_next().is_some() {
            #[cfg(debug_assertions)]
            if let Some(curr) = self.expansions.last() {
                if curr.exp.end <= self.eaten {
                    print!("finished with");
                } else {
                    print!("still within");
                }
                println!(
                    " expansion of {:?} ({:?}); expands to {:?} ({:?})",
                    self.source.span_str(curr.call),
                    curr.call,
                    self.source.span_str(curr.exp.within_file(self.file)),
                    curr.exp,
                );
            } else {
                println!("top-level");
            }

            let res = self.document_item();
            if let Err(e) = res.and_then(|item| match item {
                ast::DocumentItem::Def(def) => {
                    self.define_macro(def);
                    Ok(())
                }

                ast::DocumentItem::Cond(cond) => self.expand_conditional(cond),

                ast::DocumentItem::Call(call) => self.expand_call(call),

                ast::DocumentItem::Other(tokens) => {
                    for (span, _) in tokens.list {
                        self.processed.push_str(self.source.span_str(span));
                    }
                    Ok(())
                }
            }) {
                // TODO: what if the range is within a partially-processed file?
                self.errors.push(e);
            }
        }
        #[cfg(debug_assertions)]
        for item in &self.expansions {
            println!(
                "expansion: from {:?} to {:?}, defined with {:?}",
                self.source.span_str(item.call),
                &self.processed[<Range<usize>>::from(item.exp)],
                self.source.span_str(item.def),
            );
        }
        (self.processed, self.expansions, self.errors)
    }

    fn define_macro(&mut self, def: ast::Definition) {
        let key = self.source.span_str(def.name);
        let params = def.params.map_or_default(|group| group.params);
        let body = def.body.items.list;

        // strip from current expansion stack
        if let Some(curr) = self.expansions.last_mut()
            && curr.exp.end >= def.span.start
        {
            curr.exp.end = curr.exp.end.strict_sub(def.span.len());
        }

        _ = self.definitions.insert(
            key,
            MacroDef {
                name_span: def.name,
                params,
                body,
            },
        );
    }

    fn expand_conditional(&mut self, mut cond: ast::Conditional) -> Result<(), Error> {
        let case = loop {
            let pred = eval::eval(&cond.pred.items.list, self.source)?;
            if let eval::PreVal::Bool(pred) = pred {
                if pred {
                    break cond.then.items.list;
                } else if let Some(alt) = cond.alt {
                    use ast::AlternativeBody::{Last, More};
                    match alt.body {
                        More(x) => cond = *x,
                        Last(replacer) => break replacer.items.list,
                    }
                } else {
                    // prepend empty
                    return Ok(());
                }
            } else {
                return Err(Error::new(
                    cond.pred
                        .open
                        .super_union(cond.pred.close)
                        .unwrap_or_else(|| {
                            todo!("union between open and close from different files")
                        }),
                    ErrorKind::NonBoolPrePredicate {
                        // TODO: update ErrorKind to use PreVal
                        actual:
                            // HACK: bool literal would be the correct type.
                            // this should indicate unfinished code.
                            crate::preproc::eval::PpValue::BoolLiteral(
                                false,
                            ),
                    },
                ));
            }
        };
        self.tokens.prepend(case);
        Ok(())
    }

    fn expand_call(&mut self, call: ast::Call) -> Result<(), Error> {
        if let Some(def) = self.definitions.get(self.source.span_str(call.name)) {
            let subs = def
                .params
                .iter()
                .zip(call.args)
                .map(|(&param, arg)| {
                    let key = self.source.span_str(param);
                    let val = Substitution {
                        param,
                        arg: arg.items,
                    };
                    (key, val)
                })
                .collect::<HashMap<_, _>>();

            // in units of tokens, not bytes
            let mut substituted = Vec::new();
            let mut expanded = Vec::new();
            for &item @ (span, kind) in &def.body {
                if matches!(kind, TokenKind::MParam)
                    && let Some(sub) = subs.get(self.source.span_str(span))
                {
                    let replacement = sub.arg.list.as_slice();
                    let start = expanded.len();
                    let end = start.strict_add(replacement.len());
                    substituted.push((sub.param, start..end));
                    expanded.extend_from_slice(replacement);
                } else {
                    // ignore other macro params, they might belong to an inner definition.
                    // they will error if they are left behind after all calls have expanded.

                    expanded.push(item);

                    // note: the case `def \foo($x) { def \bar($x) { ... } }`
                    // should substitute `\bar`'s parameter with `\foo`'s argument.
                    // e.g. `\foo{7}` INTENTIONALLY expands to `def \bar(7) { ... }`.
                }
            }

            println!("substituted: {substituted:?}");

            // note: expansions will only ever add tokens forward, they cannot insert tokens ahead of previous ones

            // TODO: produce an error if the expansion becomes oversized
            let expanded_len = expanded
                .iter()
                .map(|(span, _)| span.rel.len())
                .sum::<uspan>();

            let exp = RelSpan::from(
                self.processed_len()
                    ..uspan::try_from(expanded_len)
                        .ok()
                        .and_then(|len| self.processed_len().checked_add(len))
                        .ok_or_else(|| Error::new(todo!(), ErrorKind::OversizePreproc(todo!())))?,
            );

            if let Some(curr) = self.expansions.last_mut()
                && curr.exp.end >= call.span.start
            {
                #[cfg(debug_assertions)]
                println!(
                    "growing expansion {:?}...",
                    self.source.span_str(curr.exp.within_file(self.file))
                );
                debug_assert!(
                    curr.exp.end >= call.span.end,
                    "outer expansion should not end before inner expansion\
                        \n outer: {}\
                        \n inner: {}",
                    curr.exp.end,
                    call.span.end,
                );

                curr.exp.end = curr
                    .exp
                    .end
                    // subtract call_len because the call is being stripped
                    .strict_sub(call.span.len())
                    // add expanded_len because the expansion is replacing the call
                    .strict_add(expanded_len);

                #[cfg(debug_assertions)]
                println!(
                    "expansion now refers to {:?}",
                    self.source.span_str(curr.exp.within_file(self.file))
                );
            } else {
                self.expansions.push(ExpansionCluster {
                    // TODO: this should be the span of the ENTIRE definition
                    // TBD: what if the definition was within an intermediate expansion?
                    def: def.name_span,
                    call: call.span.within_file(self.file),
                    exp,
                    args: Vec::new(), // TODO
                });
            }

            self.tokens.prepend(expanded);
            Ok(())
        } else {
            Err(Error::new(call.name, ErrorKind::MacroUndefined))
        }
    }

    /// `document_item -> definition | conditional | call | "~" MACRO  | tokens`
    fn document_item(&mut self) -> Result<ast::DocumentItem, Error> {
        self.definition()
            .map_inner(ast::DocumentItem::Def)
            .or_else(|| self.conditional().map_inner(ast::DocumentItem::Cond))
            .or_else(|| self.call().map_inner(ast::DocumentItem::Call))
            // TODO: macro const
            .unwrap_or_else(|| {
                Ok(ast::DocumentItem::Other(self.tokens_while(|(_, kind)| {
                    !matches!(
                        kind,
                        TokenKind::Keyword(Keyword::Def | Keyword::Rot)
                            | TokenKind::Punc(Punctuation::MacroConst)
                            | TokenKind::Macro
                    )
                })))
            })
    }

    /// `conditional -> "rot" replacer replacer ( alternative )?`
    fn conditional(&mut self) -> Option<Result<ast::Conditional, Error>> {
        self.pull_next_if_eq(Keyword::Rot).map(|kw| {
            Ok(ast::Conditional {
                kw,
                pred: self
                    .replacer()
                    .unwrap_or_else(|| Err(self.error(ExpectedToken::MacroPred)))?,
                then: self
                    .replacer()
                    .unwrap_or_else(|| Err(self.error(ExpectedToken::MacroCond)))?,
                alt: self.alternative().transpose()?,
            })
        })
    }

    /// `alternative -> "tor" ( conditional | replacer )`
    fn alternative(&mut self) -> Option<Result<ast::Alternative, Error>> {
        self.pull_if_eq(Keyword::Tor).map(|kw| {
            Ok(ast::Alternative {
                kw,
                body: self
                    .conditional()
                    .map_inner(|x| ast::AlternativeBody::More(Box::new(x)))
                    .or_else(|| self.replacer().map_inner(ast::AlternativeBody::Last))
                    .unwrap_or_else(|| Err(self.error(ExpectedToken::MacroCondContinuation)))?,
            })
        })
    }

    /// `call -> MACRO ( replacer )*`
    fn call(&mut self) -> Option<Result<ast::Call, Error>> {
        let start = self.eaten;
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
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "eaten should be monotonically increasing"
            )]
            let span = RelSpan(Range {
                start,
                end: self.eaten,
            });
            Ok(ast::Call { span, name, args })
        })
    }

    /// `definition -> "def" MACRO ( params )? replacer`
    fn definition(&mut self) -> Option<Result<ast::Definition, Error>> {
        let start = self.eaten;
        self.pull_next_if_eq(Keyword::Def).map(|kw| {
            Ok(ast::Definition {
                kw,
                name: self.try_pull_eq(TokenKind::Macro)?,
                params: self.params().transpose()?,
                body: self
                    .replacer()
                    .unwrap_or_else(|| Err(self.error(ExpectedToken::MacroDef)))?,
                span: RelSpan(Range {
                    start,
                    end: self.eaten,
                }),
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
