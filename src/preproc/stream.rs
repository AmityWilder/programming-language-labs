use crate::{
    error::{ContextError, ErrorType, ExpectedToken},
    grammar::match_token,
    scanner::{
        Bracket,
        token::{
            Token,
            punc::Punctuation,
            value::{Keyword, LexValue},
        },
    },
};
use std::{collections::VecDeque, range::Range};

/// `param_group -> "(" ( MACRO_PARAM )* ")"`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParamGroup<'src> {
    /// `(`
    pub open: Token<'src>,
    /// [`crate::scanner::token::value::LexValue::MacroParam`]
    pub list: Vec<Token<'src>>,
    /// `)`
    pub close: Token<'src>,
}

/// `macro_replacer -> "{" ( TOKEN )* "}"`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MacroReplacer<'src> {
    pub open: Token<'src>,
    pub content: Vec<Token<'src>>,
    pub close: Token<'src>,
}

impl<'src> MacroReplacer<'src> {
    pub fn range(&self, source: &'src str) -> Range<usize> {
        Range {
            start: self.open.lex_range(source).start,
            end: self.close.lex_range(source).end,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReplacerKind {
    #[default]
    Body,
    Argument,
    Predicate,
    ConditionBody,
}

/// `macro_definition -> "def" MACRO param_group macro_replacer`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MacroDefinition<'src> {
    /// `def`
    pub def_kw: Token<'src>,
    /// [`crate::scanner::token::value::LexValue::Macro`]
    pub name: Token<'src>,
    pub params: ParamGroup<'src>,
    pub body: MacroReplacer<'src>,
}

/// `macro_expansion -> MACRO ( macro_replacer )*`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MacroExpansion<'src> {
    /// [`crate::scanner::token::value::LexValue::Macro`]
    pub name: Token<'src>,
    pub args: Vec<MacroReplacer<'src>>,
}

impl<'src> MacroExpansion<'src> {
    pub fn range(&self, source: &'src str) -> Range<usize> {
        let name_range = self.name.lex_range(source);
        self.args.last().map_or(name_range, |last_arg| Range {
            start: name_range.start,
            end: last_arg.range(source).end,
        })
    }
}

/// `macro_predicate -> "rot" macro_replacer`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MacroPredicate<'src> {
    /// `rot`
    pub rot_kw: Token<'src>,
    pub pred: MacroReplacer<'src>,
}

/// `macro_alternative -> "~" ( macro_predicate )? macro_replacer $( macro_alternative )?`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MacroAlternative<'src> {
    /// `~`; necessary to distinguish from non-preprocessor `or`
    pub squiggle: Token<'src>,
    pub predicate: Option<MacroPredicate<'src>>,
    pub then_def: MacroReplacer<'src>,
}

/// `macro_conditional -> macro_predicate macro_replacer ( macro_alternative )*`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MacroConditional<'src> {
    pub pred: MacroPredicate<'src>,
    pub then_def: MacroReplacer<'src>,
    pub else_def: Vec<MacroAlternative<'src>>,
}

/// `macro_reference -> "~" MACRO`
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MacroRefence<'src> {
    pub squiggle: Token<'src>,
    pub macro_name: Token<'src>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreprocessorStream<'src> {
    /// The source code
    pub(super) source: &'src str,
    /// The token stream (actively modified)
    pub(super) tokens: VecDeque<Result<Token<'src>, ContextError<'src>>>,
}

impl<'src> PreprocessorStream<'src> {
    /// Construct a new preprocessor over `tokens`
    pub fn new<I>(source: &'src str, tokens: I) -> Self
    where
        I: IntoIterator<Item = Result<Token<'src>, ContextError<'src>>>,
    {
        Self {
            source,
            tokens: VecDeque::from_iter(tokens),
        }
    }

    /// Expect a token matching `p` and return [`None`] if it is not found.
    ///
    /// Includes whitespace and comments
    pub fn pull_if_next<P>(&mut self, p: P) -> Option<Token<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        // peek past ignored tokens without consuming them
        self.tokens
            .pop_front_if(|res| res.as_ref().is_ok_and(p))
            .map(|res| res.expect("should be guarded by is_ok_and"))
    }

    /// Returns the first token that is neither whitespace nor comment.
    pub fn peek(&self) -> Option<&Result<Token<'src>, ContextError<'src>>> {
        self.tokens.iter().find(|res| {
            !res.is_ok_and(|token| matches!(token.val, LexValue::Whitespace | LexValue::Comment))
        })
    }

    /// Expect a token matching `p` and return [`None`] if it is not found.
    ///
    /// Ignores whitespace and comments, but only consumes them if a match is found.
    pub fn pull_if<P>(&mut self, p: P) -> Option<Token<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        // peek past ignored tokens without consuming them
        self.tokens
            .iter()
            .enumerate()
            .map_while(|(i, res)| res.ok().map(|token| (i, token)))
            .find(|(_, token)| !matches!(token.val, LexValue::Whitespace | LexValue::Comment))
            .filter(|(_, token)| p(token))
            .map(|(pos, _)| {
                drop(self.tokens.drain(..pos));
                self.tokens
                    .pop_front()
                    .expect("should be guarded by first_not_ignored being Some")
                    .expect("should be guarded by is_ok_and")
            })
    }

    /// Expect a token matching `p` and return an error if it is not found.
    ///
    /// Ignores whitespace and comments, but only consumes them if a match is found.
    pub fn try_pull<P>(
        &mut self,
        p: P,
        expecting: ExpectedToken,
    ) -> Result<Token<'src>, ContextError<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.pull_if(p)
            .ok_or_else(|| match self.peek().copied().transpose() {
                Ok(token) => ContextError::missing_or_unexpected(token, self.source, expecting),
                Err(e) => e,
            })
    }

    pub fn param_group(&mut self) -> Result<ParamGroup<'src>, ContextError<'src>> {
        let open = self.try_pull(
            match_token!(Punctuation(Punctuation::LParen)),
            ExpectedToken::LParen,
        )?;
        let params = std::iter::from_fn(|| self.pull_if(match_token!(MacroParam))).collect();
        let close = self
            .try_pull(
                match_token!(Punctuation(Punctuation::RParen)),
                ExpectedToken::MacroParamOrRParen,
            )
            .map_err(|e| e.to_bracket_mismatch(Bracket::Paren, open.lex_range(self.source)))?;
        Ok(ParamGroup {
            open,
            list: params,
            close,
        })
    }

    pub fn replacer(
        &mut self,
        kind: ReplacerKind,
    ) -> Result<MacroReplacer<'src>, ContextError<'src>> {
        let open = self.try_pull(
            match_token!(Punctuation(Punctuation::LBrace)),
            match kind {
                ReplacerKind::Argument => ExpectedToken::MacroArgLBrace,
                ReplacerKind::Body => ExpectedToken::MacroDefLBrace,
                ReplacerKind::Predicate => ExpectedToken::MacroPredLBrace,
                ReplacerKind::ConditionBody => ExpectedToken::MacroCondLBrace,
            },
        )?;
        let content = {
            let mut depth: usize = 0;
            std::iter::from_fn(|| {
                let res = self
                    .tokens
                    .pop_front()
                    .ok_or_else(|| {
                        ContextError::error(
                            self.source,
                            None,
                            None,
                            ErrorType::MissingCloseBracket {
                                open_range: open.lex_range(self.source),
                                expect: Bracket::Brace,
                            },
                        )
                    })
                    .flatten();
                let token = match res {
                    Ok(x) => x,
                    Err(e) => return Some(Err(e)),
                };
                match token.val {
                    LexValue::Punctuation(Punctuation::LBrace) => depth = depth.strict_add(1),
                    LexValue::Punctuation(Punctuation::RBrace) => {
                        if let Some(n) = depth.checked_sub(1) {
                            depth = n;
                        } else {
                            // put it back so we can consume it after
                            self.tokens.push_front(res);
                            return None;
                        }
                    }
                    _ => (),
                }
                Some(Ok(token))
            })
            .collect::<Result<Vec<_>, _>>()?
        };
        let close = self
            .try_pull(
                match_token!(Punctuation(Punctuation::RBrace)),
                // HACK: we don't actually want an ExpectedToken error, we're going to replace it.
                ExpectedToken::MacroIdent,
            )
            .map_err(|e| e.to_bracket_mismatch(Bracket::Brace, open.lex_range(self.source)))?;
        Ok(MacroReplacer {
            open,
            content,
            close,
        })
    }

    /// Will only produce [`Some`] if the **immediately next** token begins a macro definition.
    ///
    /// Sensitive to whitespace and comments
    pub fn macro_definition(
        &mut self,
    ) -> Option<Result<MacroDefinition<'src>, ContextError<'src>>> {
        self.pull_if_next(match_token!(Keyword(Keyword::Def)))
            .map(|def_kw| {
                Ok(MacroDefinition {
                    def_kw,
                    name: self.try_pull(match_token!(Macro), ExpectedToken::MacroIdent)?,
                    params: self.param_group()?,
                    body: self.replacer(ReplacerKind::Body)?,
                })
            })
    }

    /// Will only produce [`Some`] if the **immediately next** token begins a macro expansion.
    ///
    /// Sensitive to whitespace and comments
    pub fn macro_expansion<F>(
        &mut self,
        arg_count: F,
    ) -> Option<Result<MacroExpansion<'src>, ContextError<'src>>>
    where
        F: FnOnce(&'src str) -> Option<usize>,
    {
        self.pull_if_next(match_token!(Macro)).map(|name| {
            let expected_count = arg_count(name.lex).ok_or_else(|| {
                ContextError::token_error(self.source, Some(name), ErrorType::MacroUndefined)
            })?;
            Ok(MacroExpansion {
                name,
                args: (0..expected_count)
                    .map(|_| self.replacer(ReplacerKind::Argument))
                    .collect::<Result<_, _>>()?,
            })
        })
    }

    pub fn macro_alternative(
        &mut self,
    ) -> Option<Result<MacroAlternative<'src>, ContextError<'src>>> {
        self.pull_if(match_token!(Punctuation(Punctuation::MacroConst)))
            .map(|squiggle| {
                Ok(MacroAlternative {
                    squiggle,
                    predicate: {
                        if let Some(rot_kw) = self.pull_if(match_token!(Keyword(Keyword::Rot))) {
                            Some(MacroPredicate {
                                rot_kw,
                                pred: self.replacer(ReplacerKind::Predicate)?,
                            })
                        } else {
                            None
                        }
                    },
                    then_def: self.replacer(ReplacerKind::ConditionBody)?,
                })
            })
    }

    /// Will only produce [`Some`] if the **immediately next** token begins a macro conditional.
    ///
    /// Sensitive to whitespace and comments
    pub fn macro_conditional(
        &mut self,
    ) -> Option<Result<MacroConditional<'src>, ContextError<'src>>> {
        self.pull_if_next(match_token!(Keyword(Keyword::Rot)))
            .map(|rot_kw| {
                Ok(MacroConditional {
                    pred: MacroPredicate {
                        rot_kw,
                        pred: self.replacer(ReplacerKind::Predicate)?,
                    },
                    then_def: self.replacer(ReplacerKind::ConditionBody)?,
                    else_def: std::iter::from_fn(|| self.macro_alternative())
                        .collect::<Result<_, _>>()?,
                })
            })
    }

    /// Will only produce [`Some`] if the **immediately next** token begins a macro reference.
    ///
    /// Sensitive to whitespace and comments
    pub fn macro_reference(&mut self) -> Option<Result<MacroRefence<'src>, ContextError<'src>>> {
        self.pull_if_next(match_token!(Punctuation(Punctuation::MacroConst)))
            .map(|squiggle| {
                Ok(MacroRefence {
                    squiggle,
                    macro_name: self.try_pull(match_token!(Macro), ExpectedToken::MacroIdent)?,
                })
            })
    }
}
