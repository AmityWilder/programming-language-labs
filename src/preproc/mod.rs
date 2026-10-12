//! Preprocessing (macros)

use crate::{
    error::{ContextError, ErrorType, ExpectedToken},
    grammar::match_token,
    scanner::{
        Bracket,
        token::{ExpansionData, Token, keyword::Keyword, punc::Punctuation, value::LexValue},
    },
};
use std::{
    collections::{HashMap, VecDeque},
    range::Range,
};

#[cfg(test)]
mod tests;

/// Macro substitution
#[derive(Debug, Clone)]
struct MacroSubstitute<'src, I> {
    /// Mapping of the parameter names to the related argument tokens
    arg_map: Vec<(&'src str, Vec<Token<'src>>)>,
    /// The tokens of the macro definition
    tokens: I,
    /// The [`ExpansionData::arg`] for [`Self::curr_arg`]
    curr_param: (usize, &'src str),
    /// For flattening - the tokens of the argument currently being substituted
    curr_arg: std::vec::IntoIter<Token<'src>>,
}

impl<'src, I> MacroSubstitute<'src, I>
where
    I: Iterator<Item = Token<'src>>,
{
    fn visit(
        &mut self,
        step_curr: fn(&mut std::vec::IntoIter<Token<'src>>) -> Option<Token<'src>>,
        step_token: fn(&mut I) -> Option<I::Item>,
    ) -> Option<(Option<(usize, &'src str)>, Token<'src>)> {
        loop {
            if let Some(token) = step_curr(&mut self.curr_arg) {
                break Some((Some(self.curr_param), token));
            } else if let Some(token) = step_token(&mut self.tokens) {
                if token.val == LexValue::MacroParam
                    && let Some((pos, (name, arg))) = self
                        .arg_map
                        .iter()
                        .enumerate()
                        .find(|(_, (name, _))| name == &token.lex)
                {
                    self.curr_param = (pos, name);
                    self.curr_arg = arg.clone().into_iter();
                } else {
                    break Some((None, token));
                }
            } else {
                break None;
            }
        }
    }
}

impl<'src, I> Iterator for MacroSubstitute<'src, I>
where
    I: Iterator<Item = Token<'src>>,
{
    type Item = (Option<(usize, &'src str)>, Token<'src>);

    fn next(&mut self) -> Option<Self::Item> {
        self.visit(Iterator::next, Iterator::next)
    }
}

impl<'src, I> DoubleEndedIterator for MacroSubstitute<'src, I>
where
    I: DoubleEndedIterator<Item = Token<'src>>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.visit(
            DoubleEndedIterator::next_back,
            DoubleEndedIterator::next_back,
        )
    }
}

impl<'src, I> std::iter::FusedIterator for MacroSubstitute<'src, I> where
    I: std::iter::FusedIterator<Item = Token<'src>>
{
}

/// A macro definition
#[derive(Debug, Clone, PartialEq)]
struct MacroDef<'src> {
    /// The macro parameter names
    pub params: Vec<&'src str>,
    /// The tokens of the definition (may contain macro parameters)
    pub tokens: Vec<Token<'src>>,
}

impl<'src> MacroDef<'src> {
    /// Substitute the arguments in place of the parameters in the definition
    fn substitute<'def, I>(
        &'def self,
        args: I,
    ) -> MacroSubstitute<'src, std::iter::Copied<std::slice::Iter<'def, Token<'src>>>>
    where
        I: IntoIterator<IntoIter: ExactSizeIterator, Item = Vec<Token<'src>>>,
    {
        let args = args.into_iter();
        assert_eq!(args.len(), self.params.len(), "should be handled by caller");
        let arg_map = self.params.iter().copied().zip(args).collect::<Vec<_>>();
        MacroSubstitute {
            arg_map,
            tokens: self.tokens.iter().copied(),
            // HACK: this isn't a substr of source!
            curr_param: (0, ""),
            curr_arg: Vec::new().into_iter(),
        }
    }
}

/// `param_group -> "(" ( MACRO_PARAM )* ")"`
#[derive(Debug, Clone, PartialEq, Default)]
struct ParamGroup<'src> {
    /// `(`
    pub open: Token<'src>,
    /// [`crate::scanner::token::value::LexValue::MacroParam`]
    pub list: Vec<Token<'src>>,
    /// `)`
    pub close: Token<'src>,
}

/// `macro_replacer -> "{" ( TOKEN )* "}"`
#[derive(Debug, Clone, PartialEq, Default)]
struct MacroReplacer<'src> {
    /// `{`
    pub open: Token<'src>,
    pub content: Vec<Token<'src>>,
    /// `}`
    pub close: Token<'src>,
}

impl<'src> MacroReplacer<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        Range {
            start: self.open.lex_range(source).start,
            end: self.close.lex_range(source).end,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum ReplacerKind {
    #[default]
    Body,
    Argument,
}

/// `macro_definition -> "def" MACRO param_group macro_body`
#[derive(Debug, Clone, PartialEq, Default)]
struct MacroDefinition<'src> {
    /// `def`
    pub def_kw: Token<'src>,
    /// [`crate::scanner::token::value::LexValue::Macro`]
    pub name: Token<'src>,
    pub params: ParamGroup<'src>,
    pub body: MacroReplacer<'src>,
}

/// `macro_expansion -> MACRO ( macro_argument )*`
#[derive(Debug, Clone, PartialEq, Default)]
struct MacroExpansion<'src> {
    /// [`crate::scanner::token::value::LexValue::Macro`]
    pub name: Token<'src>,
    pub args: Vec<MacroReplacer<'src>>,
}

impl<'src> MacroExpansion<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        let name_range = self.name.lex_range(source);
        self.args.last().map_or(name_range, |last_arg| Range {
            start: name_range.start,
            end: last_arg.range(source).end,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct PreprocessorStream<'src> {
    /// The source code
    source: &'src str,
    /// The token stream (actively modified)
    tokens: VecDeque<Result<Token<'src>, ContextError<'src>>>,
}

impl<'src> PreprocessorStream<'src> {
    /// Construct a new preprocessor over `tokens`
    fn new<I>(source: &'src str, tokens: I) -> Self
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
    fn pull_if_next<P>(&mut self, p: P) -> Option<Token<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        // peek past ignored tokens without consuming them
        self.tokens
            .pop_front_if(|res| res.as_ref().is_ok_and(p))
            .map(|res| res.expect("should be guarded by is_ok_and"))
    }

    /// Returns the first token that is neither whitespace nor comment.
    fn peek(&self) -> Option<&Result<Token<'src>, ContextError<'src>>> {
        self.tokens.iter().find(|res| {
            !res.is_ok_and(|token| matches!(token.val, LexValue::Whitespace | LexValue::Comment))
        })
    }

    /// Expect a token matching `p` and return [`None`] if it is not found.
    ///
    /// Ignores whitespace and comments, but only consumes them if a match is found.
    fn pull_if<P>(&mut self, p: P) -> Option<Token<'src>>
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
    fn try_pull<P>(
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

    fn param_group(&mut self) -> Result<ParamGroup<'src>, ContextError<'src>> {
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

    fn replacer(&mut self, kind: ReplacerKind) -> Result<MacroReplacer<'src>, ContextError<'src>> {
        let open = self.try_pull(
            match_token!(Punctuation(Punctuation::LBrace)),
            match kind {
                ReplacerKind::Argument => ExpectedToken::MacroArgLBrace,
                ReplacerKind::Body => ExpectedToken::MacroDefLBrace,
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
    fn macro_definition(&mut self) -> Option<Result<MacroDefinition<'src>, ContextError<'src>>> {
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
    fn macro_expansion<F>(
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
}

/// Consumes macros and converts them into token sequences
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Preprocessor<'src> {
    stream: PreprocessorStream<'src>,
    /// Macro definitions (actively modified)
    macros: HashMap<&'src str, MacroDef<'src>>,
    errors: Vec<ContextError<'src>>,
}

impl<'src> Preprocessor<'src> {
    /// Construct a new preprocessor over `tokens`
    fn new<I>(source: &'src str, tokens: I) -> Self
    where
        I: IntoIterator<Item = Result<Token<'src>, ContextError<'src>>>,
    {
        Self {
            stream: PreprocessorStream::new(source, tokens),
            macros: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn define_macro(
        &mut self,
        definition: MacroDefinition<'src>,
    ) -> Result<(), impl 'src + DoubleEndedIterator<Item = ContextError<'src>> + Clone> {
        // prevent infinitely recursive macro
        if definition
            .body
            .content
            .iter()
            .filter(match_token!(Macro))
            .any(|token| token.lex == definition.name.lex)
        {
            let head_lex = definition.name.lex;
            let head_range = definition.name.lex_range(self.stream.source);
            let source = self.stream.source;
            return Err(definition
                .body
                .content
                .into_iter()
                .filter(match_token!(Macro))
                .filter(move |token| token.lex == head_lex)
                .map(move |token| {
                    ContextError::token_error(
                        source,
                        Some(token),
                        ErrorType::RecursiveMacro { head_range },
                    )
                }));
        }

        // replace existing definition
        _ = self.macros.insert(
            definition.name.lex,
            MacroDef {
                params: definition
                    .params
                    .list
                    .into_iter()
                    .map(|token| token.lex)
                    .collect(),
                tokens: definition.body.content,
            },
        );
        Ok(())
    }

    /// # Panics
    /// This method may panic if `expansion` does not refer to an existing macro in [`Self::macros`]
    fn expand_macro(&mut self, expansion: MacroExpansion<'src>) {
        let macro_def = self
            .macros
            .get(expansion.name.lex)
            .expect("should have errored when determining the number of parameters to expect");

        let macro_range = expansion.range(self.stream.source);

        self.stream.tokens.prepend(
            macro_def
                .substitute(expansion.args.into_iter().map(|repl| repl.content))
                .map(|(argument, mut token)| {
                    token.mac = Some(ExpansionData {
                        range: macro_range,
                        arg: argument,
                    });
                    Ok(token)
                }),
        );
    }
}

impl<'src> Iterator for Preprocessor<'src> {
    type Item = Result<Token<'src>, ContextError<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(e) = self.errors.pop() {
                break Some(Err(e));
            } else if let Some(definition) = self.stream.macro_definition() {
                match definition {
                    Ok(x) => {
                        if let Err(e) = self.define_macro(x) {
                            self.errors.extend(e.rev()); // reversed because we're popping the back
                        }
                    }
                    Err(e) => break Some(Err(e)),
                }
            } else if let Some(expansion) = self
                .stream
                .macro_expansion(|name| self.macros.get(name).map(|m| m.params.len()))
            {
                match expansion {
                    Ok(x) => self.expand_macro(x),
                    Err(e) => break Some(Err(e)),
                }
            } else {
                break self.stream.tokens.pop_front();
            }
        }
    }
}

/// Preprocess a token stream to evaluate macros
pub fn preprocess<'src, A>(source: &'src str, stream: A) -> Preprocessor<'src>
where
    A: IntoIterator<IntoIter: 'src, Item = Result<Token<'src>, ContextError<'src>>>,
{
    Preprocessor::new(source, stream)
}
