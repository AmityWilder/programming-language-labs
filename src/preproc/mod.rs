//! Preprocessing (macros)

use crate::{
    error::{ContextError, ErrorType},
    grammar::{ast::AstNodeFinal, parse, token_matches},
    preproc::{
        eval::{PpValue, preproc_eval},
        stream::{MacroConditional, MacroDefinition, MacroExpansion, PreprocessorStream},
    },
    scanner::token::{ExpansionData, Lexical, Token, value::LexValue},
};
use std::collections::HashMap;

pub mod eval;
pub mod stream;

#[cfg(test)]
mod tests;

/// Macro substitution
#[derive(Debug, Clone)]
struct MacroSubstitute<'src, I> {
    /// Mapping of the parameter names to the related argument tokens
    arg_map: Vec<(&'src str, Vec<Token<'src>>)>,
    /// The tokens of the macro definition
    body: I,
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
            } else if let Some(token) = step_token(&mut self.body) {
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
    pub body: Vec<Token<'src>>,
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
            body: self.body.iter().copied(),
            // HACK: this isn't a substr of source!
            curr_param: (0, ""),
            curr_arg: Vec::new().into_iter(),
        }
    }
}

type MacroDefResult<'src, F> =
    Result<Option<MacroDef<'src>>, std::iter::FilterMap<std::vec::IntoIter<Token<'src>>, F>>;

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
    fn new<I>(source: &'src str, macros: HashMap<&'src str, MacroDef<'src>>, tokens: I) -> Self
    where
        I: IntoIterator<Item = Result<Token<'src>, ContextError<'src>>>,
    {
        Self {
            stream: PreprocessorStream::new(source, tokens),
            macros,
            errors: Vec::new(),
        }
    }

    fn expand_conditional(
        &mut self,
        conditional: MacroConditional<'src>,
    ) -> Result<(), ContextError<'src>> {
        let alternatives = std::iter::once((Some(conditional.pred), conditional.then_def)).chain(
            conditional
                .else_def
                .into_iter()
                .map(|item| (item.predicate, item.then_def)),
        );

        for (pred, body) in alternatives {
            let is_match = if let Some(p) = pred {
                let pred_preproc = Preprocessor::new(
                    self.stream.source,
                    self.macros.clone(),
                    p.pred.content.iter().copied().map(Ok),
                )
                .collect::<Result<Vec<_>, _>>()?;
                let mut parser = parse(self.stream.source, pred_preproc);
                let expr = parser.expression()?;
                assert!(parser.next().is_none(), "leftover tokens"); // TODO: make this an error
                match preproc_eval(self.stream.source, &expr, &mut |name| {
                    self.macros.contains_key(name)
                })? {
                    PpValue::BoolLiteral(x) => x,
                    actual => {
                        return Err(ContextError::node_error(
                            self.stream.source,
                            &p.pred,
                            ErrorType::NonBoolPrePredicate { actual },
                        ));
                    }
                }
            } else {
                true
            };

            if is_match {
                self.stream.tokens.prepend(body.content.into_iter().map(Ok));
                break;
            }
        }
        Ok(())
    }

    fn define_macro(
        &mut self,
        definition: MacroDefinition<'src>,
    ) -> MacroDefResult<'src, impl 'src + FnMut(Token<'src>) -> Option<ContextError<'src>>> {
        const fn is_recursive(token: &Token<'_>, name: &str) -> bool {
            token_matches!(token, Macro) && token.lex == name
        }

        fn is_unknown_param(token: &Token<'_>, params: &[&str]) -> bool {
            token_matches!(token, MacroParam) && !params.contains(&token.lex)
        }

        let MacroDefinition {
            def_kw: _,
            name,
            params,
            body,
        } = definition;

        let head = name.lex;
        let body = body.content;
        let params: Vec<&str> = params
            .into_iter()
            .flat_map(|list| list.list.into_iter().map(|token| token.lex))
            .collect();

        // if there are any errors, return an iterator over all of them
        if body
            .iter()
            .any(|token| is_recursive(token, head) || is_unknown_param(token, &params))
        {
            let head_range = name.lex_range(self.stream.source);
            let source = self.stream.source;
            Err(body.into_iter().filter_map(move |token| {
                if is_recursive(&token, head) {
                    Some(ContextError::token_error(
                        source,
                        Some(&token),
                        ErrorType::RecursiveMacro { head_range },
                    ))
                } else if is_unknown_param(&token, &params) {
                    Some(ContextError::token_error(
                        source,
                        Some(&token),
                        ErrorType::MacroParamUndefined {
                            head_range: Some(head_range),
                            param_name: token.lex,
                        },
                    ))
                } else {
                    None
                }
            }))
        } else {
            Ok(self.macros.insert(head, MacroDef { params, body }))
        }
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
            } else if let Some(macro_ref) = self.stream.macro_reference() {
                // skip expansion
                break Some(macro_ref.map(|x| x.macro_name));
            } else if let Some(conditional) = self.stream.macro_conditional() {
                if let Err(e) = conditional.and_then(|x| self.expand_conditional(x)) {
                    break Some(Err(e));
                }
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
    Preprocessor::new(source, HashMap::new(), stream)
}
