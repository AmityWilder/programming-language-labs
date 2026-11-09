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
            let call = x.call.within_file(id);
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
    call: RelSpan,
    /// The total byte-range of the overall expansion
    exp: RelSpan,
    /// The expansions that came from arguments, and which argument they came from.
    /// Arguments cannot have overlapping expansions, so only the most recent one needs to be updated.
    args: Vec<ArgExpansion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
enum StreamSpan {
    /// Token came from the file being parsed, in its original arrangement
    Original(RelSpan),
    /// Token came from macro expansion
    Injected(Span),
}

impl StreamSpan {
    pub const fn into_span(self, file: FileId) -> Span {
        match self {
            Self::Original(rel) => rel.within_file(file),
            Self::Injected(span) => span,
        }
    }

    pub const fn into_injected(self, file: FileId) -> Self {
        if let Self::Original(rel) = self {
            Self::Injected(rel.within_file(file))
        } else {
            self
        }
    }

    pub const fn rel(&self) -> &RelSpan {
        match self {
            Self::Original(rel) => rel,
            Self::Injected(span) => &span.rel,
        }
    }

    pub fn span_str<'src>(self, source: &'src SourceMap, code: &'src SourceCode) -> &'src str {
        match self {
            Self::Original(rel) => &code[rel],
            Self::Injected(span) => source.span_str(span),
        }
    }

    pub const fn is_original(&self) -> bool {
        matches!(self, Self::Original(_))
    }

    pub const fn is_injected(&self) -> bool {
        matches!(self, Self::Injected(_))
    }

    pub const fn original(self) -> Option<RelSpan> {
        if let Self::Original(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub const fn injected(self) -> Option<Span> {
        if let Self::Injected(x) = self {
            Some(x)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PreProcessor<'src> {
    source: &'src SourceMap,

    /// The file being parsed
    code: &'src SourceCode,

    /// Current macro definitions - keys are `str`s within [`Self::source`]
    definitions: HashMap<&'src str, MacroDef>,

    /// The stream of tokens to process
    queue: VecDeque<(StreamSpan, TokenKind)>,

    /// Total bytes of each token consumed, including whitespace.
    /// Increases monotonically within AST construction methods.
    /// Can be used to track the length of an AST without storing its whitespace or comments.
    eaten: uspan,

    /// Accumulated bytes from [`StreamSpan::Original`] spans consumed
    cursor: uspan,

    /// The ID of the file being processed, [`Self::code`] (NOT the postprocessed file!)
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

    #[cfg(debug_assertions)]
    printed_expansions: usize,
}

impl crate::parser::Parser for PreProcessor<'_> {
    type Item = (StreamSpan, TokenKind);

    fn peek(&mut self) -> Option<&Self::Item> {
        todo!()
    }

    fn pull_if_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>,
    {
        todo!()
    }
}

impl crate::parser::FilterParser for PreProcessor<'_> {
    fn peek_next(&mut self) -> Option<&Self::Item> {
        todo!()
    }

    fn pull_next_if_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>,
    {
        todo!()
    }
}

impl<'src> PreProcessor<'src> {
    pub fn new(source: &'src SourceMap, file: FileId) -> Self {
        let code = &source[file];
        Self {
            source,
            code,
            definitions: HashMap::new(),
            queue: VecDeque::from_iter(
                code.tokens()
                    .iter()
                    .map(|(span, kind)| (StreamSpan::Original(span), kind)),
            ),
            eaten: 0,
            cursor: 0,
            file,
            file_len: code.len(),
            expansions: Vec::new(),
            errors: Vec::new(),
            processed: String::new(),
            #[cfg(debug_assertions)]
            printed_expansions: 0,
        }
    }

    const fn track_munch(&mut self, span: StreamSpan) {
        self.eaten = self.eaten.strict_add(
            match span {
                StreamSpan::Original(rel) => rel,
                StreamSpan::Injected(span) => span.rel,
            }
            .len(),
        );
        if let StreamSpan::Original(rel) = span {
            self.cursor = self.cursor.strict_add(rel.len());
        }
    }

    fn peek_next(&self) -> Option<&(StreamSpan, TokenKind)> {
        self.queue.front()
    }

    fn pull_next_if<P>(&mut self, p: P) -> Option<(StreamSpan, TokenKind)>
    where
        P: FnOnce(&(StreamSpan, TokenKind)) -> bool,
    {
        // can't use `p` directly because Rust isn't smart enough to coerce `FnOnce(&mut T) -> U` into `FnOnce(&T) -> U` :/
        self.queue
            .pop_front_if(|x| p(x))
            .inspect(|&(span, _)| self.track_munch(span))
    }

    fn pull_next_if_eq<T>(&mut self, kind: T) -> Option<StreamSpan>
    where
        T: Into<TokenKind>,
    {
        self.pull_next_if(|(_, k)| *k == kind.into())
            .map(|(span, _)| span)
    }

    fn peek(&self) -> Option<&(StreamSpan, TokenKind)> {
        self.queue.iter().find(|(_, k)| !k.is_ignored())
    }

    fn pull_if<P>(&mut self, p: P) -> Option<(StreamSpan, TokenKind)>
    where
        P: FnOnce(&(StreamSpan, TokenKind)) -> bool,
    {
        self.queue
            .iter()
            .position(|(_, k)| !k.is_ignored())
            .filter(|&i| p(&self.queue[i]))
            .map(|i| {
                for j in 0..i.strict_add(1) {
                    self.track_munch(self.queue[j].0);
                }
                drop(self.queue.drain(..i));
                self.queue
                    .pop_front()
                    .expect("position should produce a valid index")
            })
    }

    fn pull_if_eq<T>(&mut self, kind: T) -> Option<StreamSpan>
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
    ) -> Result<(StreamSpan, TokenKind), Error>
    where
        P: FnOnce(&(StreamSpan, TokenKind)) -> bool,
    {
        self.pull_next_if(p).ok_or_else(|| self.error(expected))
    }

    fn try_pull_next_eq<T>(&mut self, kind: T) -> Result<StreamSpan, Error>
    where
        T: Into<TokenKind>,
    {
        let kind = kind.into();
        self.pull_if_eq(kind)
            .ok_or_else(|| self.error(ExpectedToken::Verbatim(kind)))
    }

    fn try_pull<P>(
        &mut self,
        p: P,
        expected: ExpectedToken,
    ) -> Result<(StreamSpan, TokenKind), Error>
    where
        P: FnOnce(&(StreamSpan, TokenKind)) -> bool,
    {
        self.pull_if(p).ok_or_else(|| self.error(expected))
    }

    fn try_pull_eq<T>(&mut self, kind: T) -> Result<StreamSpan, Error>
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
                span.into_span(self.file),
                ErrorKind::UnexpectedToken { expected },
            )
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
    ) -> Option<Result<(StreamSpan, R, StreamSpan), Error>>
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
                            open_range: open.into_span(self.file),
                            failure: BadBracketCombo::new(brack, close)
                                .expect("matching brackets should not produce an error"),
                        }
                    };
                    Error::new(span.into_span(self.file), kind)
                } else {
                    Error::new(
                        Span {
                            file: self.file,
                            rel: (self.file_len..self.file_len).into(),
                        },
                        ErrorKind::MissingCloseBracket {
                            open_range: open.into_span(self.file),
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
struct Substitution<'args> {
    param: StreamSpan,
    arg: &'args ast::Tokens,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct MacroDef {
    pub ast: ast::Definition,
    pub params: Vec<StreamSpan>,
    pub body: Vec<(StreamSpan, TokenKind)>,
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
            self.expand_item();
        }

        #[cfg(debug_assertions)]
        for item in &self.expansions {
            println!(
                "expansion: from {:?} to {:?}, defined with {:?}",
                &self.code[item.call],
                &self.processed[<Range<usize>>::from(item.exp)],
                self.source.span_str(item.def),
            );
        }

        (self.processed, self.expansions, self.errors)
    }

    /// Returns the substring within the incomplete document
    #[cfg(debug_assertions)]
    fn span_str(&self, span: RelSpan) -> std::borrow::Cow<'_, str> {
        self.processed.get(<Range<usize>>::from(span)).map_or_else(
            || {
                std::borrow::Cow::Owned(
                    // at most partially within self.processed, but rest within self.queue
                    std::iter::chain(
                        self.processed.get(uspan_to_usize(span.start)..),
                        self.queue
                            .iter()
                            .map(|(span, _)| span.span_str(self.source, self.code)),
                    )
                    .take_while({
                        let full_end = span.end;
                        let mut end: uspan = span.start;
                        move |s| {
                            let len = usize_to_uspan(s.len());
                            end = end.strict_add(len);
                            end <= full_end
                        }
                    })
                    .collect::<String>(),
                )
            },
            // entirely within self.processed
            std::borrow::Cow::Borrowed,
        )
    }

    fn expand_item(&mut self) {
        #[cfg(debug_assertions)]
        if let Some(curr) = self.expansions.last() {
            if self.processed_len() >= curr.exp.end {
                for _ in self.printed_expansions..self.expansions.len() {
                    println!(
                        "finished expansion of {:?} ({:?}), which expands to {:?} ({:?})",
                        &self.code[curr.call],
                        curr.call,
                        &self.processed[<Range<usize>>::from(curr.exp)],
                        curr.exp,
                    );
                }
                debug_assert!(
                    self.printed_expansions <= self.expansions.len(),
                    "should not remove expansions"
                );
                self.printed_expansions = self.expansions.len();
            } else {
                println!(
                    "  still processing expansion of {:?} ({:?}), currently expands to {:?} ({:?})",
                    &self.code[curr.call],
                    curr.call,
                    self.span_str(curr.exp),
                    curr.exp,
                );
            }
        }

        let res = self.document_item();
        if let Err(e) = res.and_then(|item| match item {
            ast::DocumentItem::Def(def) => {
                self.define_macro(def);
                Ok(())
            }

            ast::DocumentItem::Cond(cond) => self.expand_conditional(cond),

            ast::DocumentItem::Call(ref call) => self.expand_call(call),

            ast::DocumentItem::Const(mref) => {
                for span in [mref.tilde, mref.name] {
                    self.processed
                        .push_str(span.span_str(self.source, self.code));
                }
                Ok(())
            }

            ast::DocumentItem::Other(tokens) => {
                for (span, _) in tokens.list {
                    self.processed
                        .push_str(span.span_str(self.source, self.code));
                }
                Ok(())
            }
        }) {
            // TODO: what if the range gets trampled over when it isn't appended to `processed`? could sync fix this?
            self.errors.push(e);
        }
    }

    fn define_macro(&mut self, def: ast::Definition) {
        let key = def.name.span_str(self.source, self.code);
        let params = def
            .params
            .as_ref()
            .map_or_default(|group| group.params.clone());
        let body = def.body.items.list.clone();

        // strip from current expansion
        let processed_len = self.processed_len();
        if let Some(curr) = self.expansions.last_mut()
            && curr.exp.0.end >= processed_len
        {
            curr.exp.end = curr.exp.end.strict_sub(def.len);
        }

        _ = self.definitions.insert(
            key,
            MacroDef {
                ast: def,
                params,
                body,
            },
        );
    }

    fn expand_conditional(&mut self, mut cond: ast::Conditional) -> Result<(), Error> {
        let case = loop {
            if eval::eval_predicate(cond.pred, self.source, self.file)? {
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
        };
        self.queue.prepend(
            case.into_iter()
                .map(|(span, kind)| (span.into_injected(self.file), kind)),
        );
        Ok(())
    }

    fn expand_call(&mut self, call: &ast::Call) -> Result<(), Error> {
        if let Some(def) = self
            .definitions
            .get(call.name.span_str(self.source, self.code))
        {
            let subs = def
                .params
                .iter()
                .zip(&call.args)
                .map(|(&param, arg)| {
                    let key = param.span_str(self.source, self.code);
                    let val = Substitution {
                        param,
                        arg: &arg.items,
                    };
                    (key, val)
                })
                .collect::<HashMap<_, _>>();

            // in units of tokens, not bytes
            let mut expanded = Vec::new();
            for &item @ (span, kind) in &def.body {
                if matches!(kind, TokenKind::MParam)
                    && let Some(sub) = subs.get(span.span_str(self.source, self.code))
                {
                    let replacement = sub.arg.list.as_slice();
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

            // note: expansions will only ever add tokens forward, they cannot insert tokens ahead of previous ones

            // TODO: produce an error if the expansion becomes oversized
            let expanded_len = expanded
                .iter()
                .map(|(span, _)| span.rel().len())
                .sum::<uspan>();

            let exp = RelSpan::from(
                self.processed_len()
                    ..uspan::try_from(expanded_len)
                        .ok()
                        .and_then(|len| self.processed_len().checked_add(len))
                        .ok_or_else(|| Error::new(todo!(), ErrorKind::OversizePreproc(todo!())))?,
            );

            if call.name.is_original() {
                let name_start = call.first_span();
                let name_end = call.last_span();
                debug_assert!(
                    name_end.is_original(),
                    "call with original name should not have injected tokens in its arguments until they are lazily expanded"
                );
                self.expansions.push(ExpansionCluster {
                    def: def.ast.name.into_span(self.file),
                    call: name_start.rel().super_union(*name_end.rel()),
                    exp,
                    args: Vec::new(), // TODO
                });
            } else if let Some(curr) = self.expansions.last_mut() {
                #[cfg(debug_assertions)]
                println!("updating expansion {:?}", curr.exp);

                curr.exp.end = curr
                    .exp
                    .end
                    // subtract call_len because the call is being stripped
                    .strict_sub(call.len)
                    // add expanded_len because the expansion is replacing the call
                    .strict_add(expanded_len);

                #[cfg(debug_assertions)]
                println!("expansion is now {:?}", curr.exp);
            }

            self.queue.prepend(
                expanded
                    .into_iter()
                    .map(|(span, kind)| (span.into_injected(self.file), kind)),
            );
            Ok(())
        } else {
            Err(Error::new(
                call.name.into_span(self.file),
                ErrorKind::MacroUndefined,
            ))
        }
    }

    /// `document_item -> definition | conditional | call | macro_constant | tokens`
    fn document_item(&mut self) -> Result<ast::DocumentItem, Error> {
        self.definition()
            .map_inner(ast::DocumentItem::Def)
            .or_else(|| self.conditional().map_inner(ast::DocumentItem::Cond))
            .or_else(|| self.call().map_inner(ast::DocumentItem::Call))
            .or_else(|| self.macro_constant().map_inner(ast::DocumentItem::Const))
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

    /// `macro_constant -> "~" MACRO`
    fn macro_constant(&mut self) -> Option<Result<ast::MacroConst, Error>> {
        self.pull_next_if_eq(Punctuation::MacroConst).map(|tilde| {
            Ok(ast::MacroConst {
                tilde,
                name: self.try_pull_eq(TokenKind::Macro)?,
            })
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
                .get(name.span_str(self.source, self.code))
                .map_or_default(|def| def.params.len());
            let args = (0..expected_args)
                .map(|_| {
                    self.replacer()
                        .ok_or_else(|| self.error(ExpectedToken::MacroArg))
                        .flatten()
                })
                .collect::<Result<_, _>>()?;
            Ok(ast::Call {
                name,
                args,
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "`eaten` should increase monotonically during AST construction"
                )]
                len: self.eaten - start,
            })
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
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "`eaten` should increase monotonically during AST construction"
                )]
                len: self.eaten - start,
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
        P: FnMut(&(StreamSpan, TokenKind)) -> bool,
    {
        ast::Tokens {
            list: std::iter::from_fn(|| self.pull_next_if(&mut p)).collect(),
        }
    }
}
