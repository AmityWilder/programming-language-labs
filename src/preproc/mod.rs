//! Preprocessing (macros)

use crate::{
    error::{ContextError, ErrorType, ExpectedToken},
    grammar::match_token,
    scanner::{
        Bracket,
        token::{ExpansionData, Token, keyword::Keyword, punc::Punctuation, value::LexValue},
    },
};
use std::collections::{HashMap, VecDeque};

/// Macro substitution
#[derive(Debug, Clone)]
struct MacroSub<'src, I> {
    /// Mapping of the parameter names to the related argument tokens
    arg_map: Vec<(&'src str, Vec<Token<'src>>)>,
    /// The tokens of the macro definition
    tokens: I,
    /// For flattening - the tokens of the argument currently being substituted
    curr: ((usize, &'src str), std::vec::IntoIter<Token<'src>>),
}

impl<'src, I> Iterator for MacroSub<'src, I>
where
    I: Iterator<Item = Token<'src>>,
{
    type Item = (Option<(usize, &'src str)>, Token<'src>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(token) = self.curr.1.next() {
                break Some((Some(self.curr.0), token));
            } else if let Some(token) = self.tokens.next() {
                if token.val == LexValue::MacroParam
                    && let Some((pos, (name, arg))) = self
                        .arg_map
                        .iter()
                        .enumerate()
                        .find(|(_, (name, _))| name == &token.lex)
                {
                    self.curr = ((pos, name), arg.clone().into_iter());
                } else {
                    break Some((None, token));
                }
            } else {
                break None;
            }
        }
    }
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
    ) -> MacroSub<'src, std::iter::Copied<std::slice::Iter<'def, Token<'src>>>>
    where
        I: IntoIterator<IntoIter: ExactSizeIterator, Item = Vec<Token<'src>>>,
    {
        let args = args.into_iter();
        assert_eq!(args.len(), self.params.len(), "should be handled by caller");
        let arg_map = self.params.iter().copied().zip(args).collect::<Vec<_>>();
        MacroSub {
            arg_map,
            tokens: self.tokens.iter().copied(),
            // HACK: this isn't a substr of source!
            curr: ((0, ""), Vec::new().into_iter()),
        }
    }
}

/// Consumes macros and converts them into token sequences
#[derive(Debug, Clone)]
pub struct Preprocessor<'src> {
    /// The source code
    source: &'src str,
    /// The token stream (actively modified)
    tokens: VecDeque<Result<Token<'src>, ContextError<'src>>>,
    /// Macro definitions (actively modified)
    macros: HashMap<&'src str, MacroDef<'src>>,
}

impl<'src> Preprocessor<'src> {
    /// Construct a new preprocessor over `tokens`
    fn new<I>(source: &'src str, tokens: I) -> Self
    where
        I: IntoIterator<Item = Result<Token<'src>, ContextError<'src>>>,
    {
        Self {
            source,
            tokens: VecDeque::from_iter(tokens),
            macros: HashMap::new(),
        }
    }
}

impl<'src> Preprocessor<'src> {
    /// Expect a token matching `p` and return an error if it is not found.
    /// Ignores whitespace and comments.
    fn require<P>(
        &mut self,
        p: P,
        expecting: ExpectedToken,
    ) -> Result<Token<'src>, ContextError<'src>>
    where
        P: FnOnce(Token<'src>) -> bool,
    {
        // clear out the ignored tokens
        while self
            .tokens
            .pop_front_if(|res| {
                res.as_ref().is_ok_and(|token| {
                    matches!(token.val, LexValue::Whitespace | LexValue::Comment)
                })
            })
            .is_some()
        {}

        // next item that isn't whitespace or a comment
        match self.tokens.pop_front() {
            Some(Ok(token)) if p(token) => Ok(token),

            item => {
                // put it back
                if let Some(item) = item.clone() {
                    self.tokens.push_front(item);
                }
                Err(ContextError::missing_or_unexpected(
                    item.transpose()?,
                    self.source,
                    expecting,
                ))
            }
        }
    }

    /// Consume a macro definition (expects `def` keyword to have already been consumed)
    fn macro_define(&mut self) -> Result<(), ContextError<'src>> {
        let macro_name = self.require(match_token!(Macro), ExpectedToken::MacroIdent)?;

        _ = self.require(
            match_token!(Punctuation(Punctuation::LParen)),
            ExpectedToken::LParen,
        )?;

        let mut params = Vec::new();
        loop {
            if !params.is_empty() {
                match self
                    .require(
                        match_token!(Punctuation(Punctuation::Comma | Punctuation::RParen)),
                        ExpectedToken::CommaOrRParen,
                    )?
                    .val
                {
                    LexValue::Punctuation(Punctuation::Comma) => (),
                    LexValue::Punctuation(Punctuation::RParen) => break,

                    _ => unreachable!(),
                }
            }

            let token = self.require(
                match_token!(MacroParam | Punctuation(Punctuation::RParen)),
                ExpectedToken::MacroParamOrRParen,
            )?;
            match token.val {
                LexValue::MacroParam => params.push(token.lex),
                LexValue::Punctuation(Punctuation::RParen) => break,

                _ => unreachable!(),
            }
        }

        let open_brace = self.require(
            match_token!(Punctuation(Punctuation::LBrace)),
            ExpectedToken::MacroDefLBrace,
        )?;

        let mut def = Vec::new();
        let mut depth: usize = 0;
        loop {
            let token = self.tokens.pop_front().ok_or_else(|| {
                ContextError::error(
                    self.source,
                    None,
                    None,
                    ErrorType::MissingCloseBracket {
                        open_range: open_brace.lex_range(self.source),
                        expect: Bracket::Brace,
                    },
                )
            })??;
            match token.val {
                LexValue::Punctuation(Punctuation::LBrace) => depth = depth.strict_add(1),
                LexValue::Punctuation(Punctuation::RBrace) => {
                    if let Some(n) = depth.checked_sub(1) {
                        depth = n;
                    } else {
                        break;
                    }
                }
                _ => (),
            }
            def.push(token);
        }
        // replace existing definition
        _ = self.macros.insert(
            macro_name.lex,
            MacroDef {
                params,
                tokens: def,
            },
        );

        Ok(())
    }

    /// Expand a macro call into its substituted definition
    fn macro_expand(&mut self, macro_name: Token<'src>) -> Result<(), ContextError<'src>> {
        let mut macro_range = macro_name.lex_range(self.source);

        let param_count = self
            .macros
            .get(&macro_name.lex)
            .ok_or_else(|| {
                ContextError::error(
                    self.source,
                    // TODO: why is this being duplicated?
                    Some(macro_name.lex_range(self.source)),
                    Some(ExpansionData {
                        range: macro_range,
                        arg: None, // TODO
                    }),
                    ErrorType::MacroUndefined,
                )
            })?
            .params
            .len();

        let mut args = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            let open_brace = self.require(
                match_token!(Punctuation(Punctuation::LBrace)),
                ExpectedToken::MacroArgLBrace,
            )?;
            let mut arg = Vec::new();
            let mut depth: usize = 0;
            loop {
                let token = self.tokens.pop_front().ok_or_else(|| {
                    ContextError::error(
                        self.source,
                        None,
                        Some(ExpansionData {
                            range: macro_range,
                            arg: None, // TODO
                        }),
                        ErrorType::MissingCloseBracket {
                            open_range: open_brace.lex_range(self.source),
                            expect: Bracket::Brace,
                        },
                    )
                })??;
                match token.val {
                    LexValue::Punctuation(Punctuation::LBrace) => depth = depth.strict_add(1),
                    LexValue::Punctuation(Punctuation::RBrace) => {
                        if let Some(n) = depth.checked_sub(1) {
                            depth = n;
                        } else {
                            macro_range.end = token.lex_range(self.source).end;
                            break;
                        }
                    }
                    _ => (),
                }
                arg.push(token);
            }
            args.push(arg);
        }

        let def = self
            .macros
            .get(&macro_name.lex)
            .expect("should have returned an error at the start of the fn");

        self.tokens.prepend(
            def.substitute(args)
                .map(|(argument, mut token)| {
                    token.mac = Some(ExpansionData {
                        range: macro_range,
                        arg: argument,
                    });
                    Ok(token)
                })
                .collect::<Vec<_>>()
                .drain(..),
        );

        Ok(())
    }
}

impl<'src> Iterator for Preprocessor<'src> {
    type Item = Result<Token<'src>, ContextError<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.tokens.pop_front() {
                // define macro
                Some(Ok(Token {
                    val: LexValue::Keyword(Keyword::Def),
                    ..
                })) => {
                    if let Err(e) = self.macro_define() {
                        break Some(Err(e));
                    }
                }

                // expand macro
                Some(Ok(
                    token @ Token {
                        val: LexValue::Macro,
                        ..
                    },
                )) => {
                    if let Err(e) = self.macro_expand(token) {
                        break Some(Err(e));
                    }
                }

                item => break item,
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
