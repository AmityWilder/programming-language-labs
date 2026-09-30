//! Preprocessing (macros)

use crate::{
    error::{ContextError, ErrorType},
    grammar::match_token,
    scanner::{
        Bracket,
        token::{Token, keyword::Keyword, punc::Punctuation, value::Value},
    },
};
use std::collections::{HashMap, VecDeque};

/// Macro substitution
#[derive(Debug, Clone)]
struct MacroSub<'src, I> {
    /// Mapping of the parameter names to the related argument tokens
    arg_map: HashMap<&'src str, Vec<Token<'src>>>,
    /// The tokens of the macro definition
    tokens: I,
    /// For flattening - the tokens of the argument currently being substituted
    curr: std::vec::IntoIter<Token<'src>>,
}

impl<'src, I> Iterator for MacroSub<'src, I>
where
    I: Iterator<Item = Token<'src>>,
{
    type Item = Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let item @ Some(_) = self.curr.next() {
                break item;
            } else if let Some(token) = self.tokens.next() {
                match token.val {
                    Value::MacroParam if let Some(arg) = self.arg_map.get(&token.lex) => {
                        self.curr = arg.clone().into_iter();
                    }
                    _ => break Some(token),
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
        let arg_map = HashMap::from_iter(self.params.iter().copied().zip(args));

        MacroSub {
            arg_map,
            tokens: self.tokens.iter().copied(),
            curr: Vec::new().into_iter(),
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
        expecting: &'static str,
    ) -> Result<Token<'src>, ContextError<'src>>
    where
        P: FnOnce(Token<'src>) -> bool,
    {
        // clear out the ignored tokens
        while self
            .tokens
            .pop_front_if(|res| {
                res.as_ref()
                    .is_ok_and(|token| matches!(token.val, Value::Whitespace | Value::Comment))
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
        let macro_name = self.require(match_token!(Macro), "a macro identifier")?;

        _ = self.require(match_token!(Punctuation(Punctuation::LParen)), "a `(`")?;

        let mut params = Vec::new();
        loop {
            if !params.is_empty() {
                match self
                    .require(
                        match_token!(Punctuation(Punctuation::Comma | Punctuation::RParen)),
                        "a `,` or `)`",
                    )?
                    .val
                {
                    Value::Punctuation(Punctuation::Comma) => (),
                    Value::Punctuation(Punctuation::RParen) => break,

                    _ => unreachable!(),
                }
            }

            let token = self.require(
                match_token!(MacroParam | Punctuation(Punctuation::RParen)),
                "a macro parameter or `)`",
            )?;
            match token.val {
                Value::MacroParam => params.push(token.lex),
                Value::Punctuation(Punctuation::RParen) => break,

                _ => unreachable!(),
            }
        }

        let open_brace = self.require(
            match_token!(Punctuation(Punctuation::LBrace)),
            "a `{` for macro definition",
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
                        expect: (Bracket::Brace, open_brace.lex_range(self.source)),
                    },
                )
            })??;
            match token.val {
                Value::Punctuation(Punctuation::LBrace) => depth = depth.strict_add(1),
                Value::Punctuation(Punctuation::RBrace) => {
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
            .ok_or_else(|| ContextError {
                source: self.source,
                range: macro_name.lex_range(self.source),
                macro_range: Some(macro_range),
                err: ErrorType::MacroUndefined,
            })?
            .params
            .len();

        let mut args = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            let open_brace = self.require(
                match_token!(Punctuation(Punctuation::LBrace)),
                "a `{` for macro argument",
            )?;
            let mut arg = Vec::new();
            let mut depth: usize = 0;
            loop {
                let token = self.tokens.pop_front().ok_or_else(|| {
                    ContextError::error(
                        self.source,
                        None,
                        Some(macro_range),
                        ErrorType::MissingCloseBracket {
                            expect: (Bracket::Brace, open_brace.lex_range(self.source)),
                        },
                    )
                })??;
                match token.val {
                    Value::Punctuation(Punctuation::LBrace) => depth = depth.strict_add(1),
                    Value::Punctuation(Punctuation::RBrace) => {
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
                .map(|mut token| {
                    token.mac = Some(macro_range);
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
                    val: Value::Keyword(Keyword::Def),
                    ..
                })) => {
                    if let Err(e) = self.macro_define() {
                        break Some(Err(e));
                    }
                }

                // expand macro
                Some(Ok(
                    token @ Token {
                        val: Value::Macro, ..
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
