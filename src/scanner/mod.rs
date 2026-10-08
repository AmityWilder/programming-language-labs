//! The iterator that breaks source code into tokens (which are defined in [`token`] module).

use crate::{
    error::{ContextError, ErrorType},
    regex::{FauxRegex, digit_char, word_char},
    scanner::symbols::{
        BLOCK_COMMENT_CLOSE, BLOCK_COMMENT_OPEN, CHAR_DELIM, ESCAPE, LINE_COMMENT_OPEN,
        MACRO_PARAM_PREFIX, MACRO_PREFIX, TEXT_DELIM,
    },
};
use std::range::Range;
use token::{Token, keyword::Keyword, punc::Punctuation, value::LexValue};

pub mod symbols;
pub mod token;

/// Finds the first `looking_for` not preceded by an odd number of [`ESCAPE`]s
const fn unescaped(looking_for: char) -> impl FnMut(char) -> bool {
    let mut is_escaped = false;
    move |ch| {
        !is_escaped && ch == looking_for || {
            is_escaped = !is_escaped && ch == ESCAPE;
            false
        }
    }
}

/// A bracket character
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bracket {
    /// `[`/`]`
    Brack,
    /// `(`/`)`
    Paren,
    /// `{`/`}`
    Brace,
}

impl Bracket {
    /// The open partner of the bracket
    #[must_use]
    pub const fn open(self) -> char {
        match self {
            Self::Brack => '[',
            Self::Paren => '(',
            Self::Brace => '{',
        }
    }

    /// The close partner of the bracket
    #[must_use]
    pub const fn close(self) -> char {
        match self {
            Self::Brack => ']',
            Self::Paren => ')',
            Self::Brace => '}',
        }
    }
}

/// An invalid combination of brackets (order matters)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadBracketCombo {
    /// `[`/`)`
    BrackParen,
    /// `[`/`}`
    BrackBrace,
    /// `(`/`]`
    ParenBrack,
    /// `(`/`}`
    ParenBrace,
    /// `{`/`]`
    BraceParen,
    /// `{`/`)`
    BraceBrack,
}

impl BadBracketCombo {
    #[must_use]
    pub const fn new(open: Bracket, close: Bracket) -> Option<Self> {
        match (open, close) {
            (Bracket::Brack, Bracket::Paren) => Some(Self::BrackParen),
            (Bracket::Brack, Bracket::Brace) => Some(Self::BrackBrace),
            (Bracket::Paren, Bracket::Brack) => Some(Self::ParenBrack),
            (Bracket::Paren, Bracket::Brace) => Some(Self::ParenBrace),
            (Bracket::Brace, Bracket::Brack) => Some(Self::BraceBrack),
            (Bracket::Brace, Bracket::Paren) => Some(Self::BraceParen),

            _ => None,
        }
    }

    /// (open, close)
    #[must_use]
    pub const fn decompose(self) -> (Bracket, Bracket) {
        match self {
            Self::BrackParen => (Bracket::Brack, Bracket::Paren),
            Self::BrackBrace => (Bracket::Brack, Bracket::Brace),
            Self::ParenBrack => (Bracket::Paren, Bracket::Brack),
            Self::ParenBrace => (Bracket::Paren, Bracket::Brace),
            Self::BraceParen => (Bracket::Brace, Bracket::Brack),
            Self::BraceBrack => (Bracket::Brace, Bracket::Paren),
        }
    }
}

impl From<BadBracketCombo> for (Bracket, Bracket) {
    #[inline]
    fn from(value: BadBracketCombo) -> Self {
        value.decompose()
    }
}

/// Returns [`None`] if this is not a number literal, and probably something else
///
/// Equivalent to regex: `-?\d\w*(?:\.\d\w*)?(?:[eE][-+]\d\w*)?`
fn match_num_literal(src: &str, allow_negative: bool) -> Option<&str> {
    let mut re = FauxRegex::within(src);
    re
        // conditional on whether negative is allowed here
        .opt_group(|re| allow_negative.then(|| re.optional('-'))) // -?
        // only instance of disqualification instead of shortening
        .exactly(digit_char)? // \d
        .repeat(word_char) // \w*
        // (?:
        .opt_group(|re| {
            re
                .exactly('.')? // \.
                // if the first character after the dot is a letter, it might instead be a method or range expression
                .exactly(digit_char)? // \d
                .repeat(word_char) // \w*
                .end()
        })
        // )?
        // (?:
        .opt_group(|re| {
            re
                // uses lookbehind because [`FauxRegex`] is EXTRA greedy and will munch characters
                // even if the next part of the pattern could match it
                .lookbehind(['e', 'E'])? // [eE]
                // this is required because otherwise there's no reason to make this special case.
                // the rest of the pattern would have matched a fully alphanumeric exponent anyway.
                .exactly(['-', '+'])? // [-+]
                // if the first character after the minus is a letter, it might instead be subtracting an identifier
                .exactly(digit_char)? // \d
                // not just digits in case there's a suffix
                .repeat(word_char) // \w*
                .end()
        })
        // )?
    ;
    Some(re.matched())
}

/// An iterator that breaks down text into tokens
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scanner<'src> {
    /// This one doesn't get ripped apart, it exists for fulfilling context errors
    original: &'src str,

    /// A reference to the original source code. Since this is only a copy, it will get ripped apart and fed to the tokens.
    /// The next token will always be at the start of this string.
    source: &'src str,

    /// The most recent non-whitespace, non-comment token was either the start of the source code or [`LexValue::Punctuation`]
    /// **and not** `)`, `]`, or `}`.
    can_be_negative: bool,

    /// The most recent non-whitespace, non-comment token was a `fn` keyword
    is_following_fn: bool,
}

impl<'src> Scanner<'src> {
    /// Construct a new [`Scanner`] for `source`
    pub const fn new(source: &'src str) -> Self {
        Self {
            original: source,
            source,
            // start off true because we are at the start of the source code
            can_be_negative: true,
            is_following_fn: false,
        }
    }

    pub const fn new_subset(original: &'src str, source: &'src str, can_be_negative: bool) -> Self {
        Self {
            original,
            source,
            can_be_negative,
            is_following_fn: false,
        }
    }

    /// Get a lexeme of length `len` from the start of the source code
    ///
    /// # Panics
    /// This method will panic if `len` splits `self.source` partway through a character or beyond the end of the source string.
    fn split_off(&mut self, len: usize) -> Option<&'src str> {
        self.source.split_at_checked(len).map(|(front, back)| {
            self.source = back;
            front
        })
    }

    /// Generate an error on the most recent (complete) token
    fn error_prev(&mut self, len: usize, err: ErrorType<'src>) -> ContextError<'src> {
        let end = self
            .original
            .substr_range(self.source)
            .expect("source should be a substring of original")
            .start;
        ContextError::error(
            self.original,
            Some(Range {
                start: end.checked_sub(len).expect(
                    "len should be the size of a token that was split off from the source string",
                ),
                end,
            }),
            None,
            err,
        )
    }

    /// Generate an error starting at the current (incomplete) token
    ///
    /// [Splits off](Self::split_off) the erroneous segment so we can find more errors
    fn error_here(&mut self, len: usize, err: ErrorType<'src>) -> ContextError<'src> {
        _ = self.split_off(len);
        self.error_prev(len, err)
    }

    /// Split off a [`LexValue::Whitespace`] from the start of the source code
    ///
    /// Returns [`None`] if the next token is not whitespace
    fn scan_whitespace(&mut self) -> Option<Token<'src>> {
        let len = self
            .source
            .find(|ch: char| !ch.is_whitespace())
            .unwrap_or(self.source.len());
        (len != 0).then(|| Token {
            lex: self
                .split_off(len)
                .expect("find and len should return safe positions within source"),
            val: LexValue::Whitespace,
            mac: None,
        })
    }

    /// Split off a [`LexValue::Macro`] from the start of the source code
    ///
    /// Returns [`None`] if the next token is not a macro
    fn scan_macro(&mut self) -> Option<Token<'src>> {
        let len = self
            .source
            .strip_prefix(MACRO_PREFIX)?
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
            .map_or(self.source.len(), |n| {
                n.checked_add(MACRO_PREFIX.len_utf8())
                    .expect("n is the length of the string after this character")
            });
        let lex = self
            .split_off(len)
            .expect("find and len should return safe positions to split at");
        Some(Token {
            lex,
            val: LexValue::Macro,
            mac: None,
        })
    }

    /// Split off a macro parameter from the start of the source code
    ///
    /// Returns [`None`] if the next token is not a macro parameter
    fn scan_macro_param(&mut self) -> Option<Token<'src>> {
        let len = self
            .source
            .strip_prefix(MACRO_PARAM_PREFIX)?
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
            .map_or(self.source.len(), |n| {
                n.checked_add(MACRO_PARAM_PREFIX.len_utf8())
                    .expect("n is the length of the string after this character")
            });
        let lex = self
            .split_off(len)
            .expect("find and len should return safe positions to split at");
        Some(Token {
            lex,
            val: LexValue::MacroParam,
            mac: None,
        })
    }

    /// Split off a [`LexValue::TextLiteral`]/[`LexValue::CharLiteral`] from the start of the source code
    ///
    /// Returns [`None`] if the next token is not a text/char literal
    fn scan_strlike_literal(&mut self) -> Option<Result<Token<'src>, ContextError<'src>>> {
        let open_delim = self
            .source
            .chars()
            .next()
            .filter(|ch| matches!(*ch, TEXT_DELIM | CHAR_DELIM))?;
        let rest = self.source.strip_prefix(open_delim).expect(
            "should not call `scan_strlike_literal` if `starts_with_strlike_literal` is false",
        );
        let line_end = rest.find('\n').unwrap_or(rest.len());
        let rest = rest
            .get(..line_end)
            .expect("find and len should not be within a UTF-8 character");
        Some(rest.find(unescaped(open_delim))
            .map(|n| {
                const {
                    assert!(
                        char::MAX_LEN_UTF8.checked_mul(2).is_some(),
                        "proof. char::MAX_LEN_UTF8 * 2 fits in usize"
                    );
                }
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "As shown above, `char::MAX_LEN_UTF8 * 2` fits in usize. \
                              By definition of `char::MAX_LEN_UTF8`, `c.len_utf8()` is at most `char::MAX_LEN_UTF8` for all `c: char`. \
                              Therefore, `c.len_utf8() * 2` fits in usize for all `c: char`.",
                )]
                // why 2x? first for open delimiter, second for close delimiter (both are the same character)
                (open_delim.len_utf8() * 2)
                    .checked_add(n)
                    .expect(
                        "stringlike literal should include both open and close delimiters, \
                         which must have fit in memory in the original source code and therefore \
                         have a len that fits in usize",
                    )
            })
            .ok_or_else(|| {
                self.error_here(
                    line_end.strict_add(1),
                    // the fact there is a closing delimiter that didn't end the string shows it must be escaped
                    // (or else there wouldn't have been an error)
                    match (open_delim, rest.contains(open_delim)) {
                        (CHAR_DELIM, true) => ErrorType::EscapedCharLiteralEnd,
                        (CHAR_DELIM, false) => ErrorType::EndlessCharLiteral,
                        (TEXT_DELIM, true) => ErrorType::EscapedStringLiteralEnd,
                        (TEXT_DELIM, false) => ErrorType::EndlessStringLiteral,
                        _ => unimplemented!(),
                    },
                )
            })
            .and_then(|len| {
                let lex = self
                    .split_off(len)
                    .expect("find and len should return safe positions to split at");
                match open_delim {
                    TEXT_DELIM => LexValue::text_literal(lex).map(|val| Token {
                        lex,
                        val,
                        mac: None,
                    }),
                    CHAR_DELIM => LexValue::char_literal(lex).map(|val| Token {
                        lex,
                        val,
                        mac: None,
                    }),

                    _ => unreachable!("should be guarded by if condition"),
                }
                .map_err(|err| self.error_prev(len, err))
            }))
    }

    /// Split off a [`LexValue::Identifier`] from the start of the source code
    ///
    /// Returns [`None`] if the next token is not an identifier
    fn scan_ident(&mut self) -> Option<Token<'src>> {
        if !self
            .source
            .starts_with(|ch: char| ch.is_alphabetic() || ch == '_')
        {
            return None;
        }
        let len = self
            .source
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
            .unwrap_or(self.source.len());
        let lex = self
            .split_off(len)
            .expect("find and len should return safe positions to split at");
        let val = if let Some(kw) = Keyword::try_from_str(lex) {
            LexValue::Keyword(kw)
        } else {
            match lex {
                "true" => LexValue::BoolLiteral(true),
                "fals" => LexValue::BoolLiteral(false),
                _ => {
                    if self.is_following_fn || self.source.starts_with('(')
                    // assumes the token has already been split off
                    {
                        LexValue::Callable
                    } else {
                        LexValue::Identifier
                    }
                }
            }
        };
        Some(Token {
            lex,
            val,
            mac: None,
        })
    }

    /// Split off a number literal from the start of the source code
    ///
    /// Returns [`None`] if the next token is not a number literal
    fn scan_num_literal(&mut self) -> Option<Result<Token<'src>, ContextError<'src>>> {
        let len = match_num_literal(self.source, self.can_be_negative)?.len();
        let lex = self
            .split_off(len)
            .expect("should be a safe position to split at");
        Some(
            LexValue::number_literal(lex)
                .map(|val| Token {
                    lex,
                    val,
                    mac: None,
                })
                .map_err(|err| self.error_prev(len, err)),
        )
    }

    /// Split off a line comment from the start of the source code
    ///
    /// Returns [`None`] if the next token is not a line comment
    fn scan_line_comment(&mut self) -> Option<Token<'src>> {
        if !self.source.starts_with(LINE_COMMENT_OPEN) {
            return None;
        }
        let len = self
            .source
            .lines()
            .next() // take the first line (excluding newline/return)
            .expect("the existence of characters should imply the existence of a line")
            .len();
        Some(Token {
            lex: self
                .split_off(len)
                .expect("should be a safe position to split at"),
            val: LexValue::Comment,
            mac: None,
        })
    }

    /// Split off a block comment from the start of the source code
    ///
    /// Returns [`None`] if the next token is not a block comment
    fn scan_block_comment(&mut self) -> Option<Result<Token<'src>, ContextError<'src>>> {
        const BLOCK_COMMENT_CIRCUMFIX_LEN: usize =
            BLOCK_COMMENT_OPEN.len() + BLOCK_COMMENT_CLOSE.len();
        if !self.source.starts_with(BLOCK_COMMENT_OPEN) {
            return None;
        }
        let mut prev_char = None;
        let mut depth: usize = 0;
        let len = self.source.strip_prefix(BLOCK_COMMENT_OPEN)
            .expect("should not call `scan_block_comment` if `starts_with_block_comment` is false")
            .find(|ch: char| {
                if prev_char == Some('*') && ch == '/' {
                    if let Some(n) = depth.checked_sub(1) {
                        depth = n;
                    } else {
                        return true;
                    }
                } else if prev_char == Some('/') && ch == '*' {
                    depth = depth
                        .checked_add(1)
                        .unwrap_or_else(|| panic!("cannot exceed depth of {}", usize::MAX));
                }
                prev_char = Some(ch);
                false
            })
            .map(|n| n.checked_add(BLOCK_COMMENT_CIRCUMFIX_LEN)
                .expect("n should describe the non-block-comment-circumfix subset of a string in memory"));
        Some(
            len.map(|len| Token {
                lex: self
                    .split_off(len)
                    .expect("should be a safe position to split at"),
                val: LexValue::Comment,
                mac: None,
            })
            .ok_or_else(|| self.error_here(self.source.len(), ErrorType::EndlessBlockComment)),
        )
    }

    /// Split off a [`LexValue::Punctuation`] from the start of the source code
    ///
    /// Returns [`None`] if the next token is not valid punctuation
    fn scan_punc(&mut self) -> Option<Token<'src>> {
        Punctuation::from_prefix(self.source).map(|punc| {
            let lex = self
                .split_off(punc.as_str().len())
                .expect("should be a safe position to split at");
            Token {
                lex,
                val: LexValue::Punctuation(punc),
                mac: None,
            }
        })
    }
}

impl<'src> Iterator for Scanner<'src> {
    type Item = Result<Token<'src>, ContextError<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        // if there are no characters remaining, this will return None and stop iterating.
        (!self.source.is_empty()).then(|| {
            // we check for the pattern of the token with "if/else" instead of "if { return }"
            // because once we have identified what type of token it should be, there must be an error if it isn't that.
            // if we continued going down the list of possible tokens until one succeeded, we would be doing
            // more processing and miss the fact that it wasn't a *different* token, it was just an *invalid* token.

            // branches ordered by:
            // 1. if a pattern might fit multiple branches, the most specific one must come before a less specific one;
            //    so that we don't eliminate the opportunity to check if it's more specific.
            // 2. if branches are equally simple or do not overlap, simplest conditions first; so that we aren't testing
            //    a complex condition on tokens that don't satisfy them, when they might have satisfied a less expensive
            //    condition for a different branch.

            None // <- exists only so the first item can also be in an `or_else`, for cleaner formatting
                .or_else(|| self.scan_whitespace().map(Ok))
                .or_else(|| self.scan_macro().map(Ok))
                .or_else(|| self.scan_macro_param().map(Ok))
                .or_else(|| self.scan_strlike_literal())
                .or_else(|| self.scan_ident().map(Ok))
                .or_else(|| self.scan_num_literal())
                .or_else(|| self.scan_line_comment().map(Ok))
                .or_else(|| self.scan_block_comment())
                .or_else(|| self.scan_punc().map(Ok))
                .unwrap_or_else(|| Err(self.error_here(
                    self.source
                        .chars()
                        .next()
                        .expect("source should have at least one character if it is not empty")
                        .len_utf8(),
                    ErrorType::UnknownToken,
                )))
                .inspect(|token| {
                    // non-whitespace, non-comment token
                    if !matches!(token.val, LexValue::Whitespace | LexValue::Comment) {
                        self.is_following_fn = matches!(token.val, LexValue::Keyword(Keyword::Fn));

                        // punctuation except for close bracket
                        self.can_be_negative = matches!(token.val, LexValue::Punctuation(punc) if
                            !matches!(punc, Punctuation::RParen | Punctuation::RBrack | Punctuation::RBrace));
                    }
                })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.source.len()))
    }
}

/// [`Scanner`] will never return another element after outputting [`None`].
impl std::iter::FusedIterator for Scanner<'_> {}

/// Create a [`Scanner`] for the provided source code, and contextualize errors if there are any
pub const fn tokenize(source: &str) -> Scanner<'_> {
    Scanner::new(source)
}

/// Finds the last block comment in the string
pub fn rfind_block_comment(s: &str) -> Option<Range<usize>> {
    s.rfind(BLOCK_COMMENT_CLOSE).map(|end| {
        let mut prev = '\0';
        let mut depth: usize = 0;
        #[expect(
            clippy::string_slice,
            reason = "rfind should not be within a UTF-8 character"
        )]
        let start = s[..end]
            .rfind(|ch: char| {
                match (ch, prev) {
                    ('*', '/') => depth = depth.strict_add(1),
                    ('/', '*') => {
                        if let Some(n) = depth.checked_sub(1) {
                            depth = n;
                        } else {
                            return true;
                        }
                    }
                    _ => (),
                }
                prev = ch;
                false
            })
            .unwrap_or(0);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "this is the length of the substr found at the end"
        )]
        Range {
            start,
            end: end + BLOCK_COMMENT_CLOSE.len(),
        }
    })
}
