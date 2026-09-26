//! The iterator that breaks source code into tokens (which are defined in [`token`] module).

use crate::{
    error::{ContextError, ErrorType},
    scanner::symbols::{
        BLOCK_COMMENT_CLOSE, BLOCK_COMMENT_OPEN, CHAR_DELIM, ESCAPE, LINE_COMMENT_OPEN,
        MACRO_PARAM_PREFIX, MACRO_PREFIX, STR_DELIM,
    },
};
use std::range::Range;
use token::{Keyword, KeywordType, Punctuation, Token, TokenType, TokenValue};

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
#[allow(dead_code, reason = "reserved for future use")]
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

/// An iterator that breaks down text into tokens
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scanner<'a> {
    /// This one doesn't get ripped apart, it exists for fulfilling context errors
    original: &'a str,

    /// A reference to the original source code. Since this is only a copy, it will get ripped apart and fed to the tokens.
    /// The next token will always be at the start of this string.
    source: &'a str,

    /// The most recent non-whitespace, non-comment token was either the start of the source code or [`TokenType::Punctuation`]
    /// **and not** `)`, `]`, or `}`.
    can_be_negative: bool,
}

impl<'a> Scanner<'a> {
    /// Construct a new [`Scanner`] for `source`
    const fn new(source: &'a str) -> Self {
        Self {
            original: source,
            source,
            // start off true because we are at the start of the source code
            can_be_negative: true,
        }
    }

    /// Get a lexeme of length `len` from the start of the source code
    ///
    /// # Panics
    /// This method will panic if `len` splits `self.source` partway through a character or beyond the end of the source string.
    fn split_off(&mut self, len: usize) -> Option<&'a str> {
        self.source.split_at_checked(len).map(|(front, back)| {
            self.source = back;
            front
        })
    }

    /// Generate an error on the most recent (complete) token
    fn error_prev(&mut self, len: usize, err: ErrorType<'a>) -> ContextError<'a> {
        let end = self
            .original
            .substr_range(self.source)
            .expect("source should be a substring of original")
            .start;
        ContextError {
            source: self.original,
            range: Range {
                start: end.checked_sub(len).expect(
                    "len should be the size of a token that was split off from the source string",
                ),
                end,
            },
            err,
        }
    }

    /// Generate an error starting at the current (incomplete) token
    ///
    /// [Splits off](Self::split_off) the erroneous segment so we can find more errors
    fn error_here(&mut self, len: usize, err: ErrorType<'a>) -> ContextError<'a> {
        _ = self.split_off(len);
        self.error_prev(len, err)
    }

    /// The source code starts with [`TokenType::Whitespace`]
    fn starts_with_whitespace(&self) -> bool {
        self.source.starts_with(char::is_whitespace)
    }

    /// Split off a [`TokenType::Whitespace`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_whitespace`] would not have returned true
    fn scan_whitespace(&mut self) -> Token<'a> {
        let len = self
            .source
            .find(|ch: char| !ch.is_whitespace())
            .unwrap_or(self.source.len());
        Token {
            src: self
                .split_off(len)
                .expect("find and len should return safe positions within source"),
            ty: TokenType::Whitespace,
            val: TokenValue::Ignore,
        }
    }

    /// The source code starts with [`TokenType::Macro`]
    fn starts_with_macro(&self) -> bool {
        self.source.starts_with(MACRO_PREFIX)
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_macro(&mut self) -> Token<'a> {
        let len = self
            .source
            .strip_prefix(MACRO_PREFIX)
            .expect("should not call `scan_macro` if `starts_with_macro` is false")
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
            .map_or(self.source.len(), |n| {
                n.checked_add(MACRO_PREFIX.len_utf8())
                    .expect("n is the length of the string after this character")
            });
        let src = self
            .split_off(len)
            .expect("find and len should return safe positions to split at");
        Token {
            src,
            ty: TokenType::Macro,
            val: TokenValue::Direct(src),
        }
    }

    /// The source code starts with [`TokenType::Macro`]
    fn starts_with_macro_param(&self) -> bool {
        self.source.starts_with(MACRO_PARAM_PREFIX)
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_macro_param(&mut self) -> Token<'a> {
        let len = self
            .source
            .strip_prefix(MACRO_PARAM_PREFIX)
            .expect("should not call `scan_macro_param` if `starts_with_macro_param` is false")
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
            .map_or(self.source.len(), |n| {
                n.checked_add(MACRO_PARAM_PREFIX.len_utf8())
                    .expect("n is the length of the string after this character")
            });
        let src = self
            .split_off(len)
            .expect("find and len should return safe positions to split at");
        Token {
            src,
            ty: TokenType::MacroParam,
            val: TokenValue::Direct(src),
        }
    }

    /// The source code starts with [`TokenType::Macro`]
    ///
    /// Returns the delimiter
    fn starts_with_strlike_literal(&self) -> Option<char> {
        self.source
            .chars()
            .next()
            .filter(|ch| matches!(*ch, STR_DELIM | CHAR_DELIM))
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_strlike_literal(&mut self, open_delim: char) -> Result<Token<'a>, ContextError<'a>> {
        let rest = self.source.strip_prefix(open_delim).expect(
            "should not call `scan_strlike_literal` if `starts_with_strlike_literal` is false",
        );
        rest.find(unescaped(open_delim))
            .map(|n| {
                const {
                    assert!(
                        char::MAX_LEN_UTF8.checked_mul(2).is_some(),
                        "proof. char::MAX_LEN_UTF8 * 2 fits in usize"
                    );
                }
                // SAFETY: As shown above, `char::MAX_LEN_UTF8 * 2` fits in usize.
                // By definition of `char::MAX_LEN_UTF8`, `c.len_utf8()` is at most `char::MAX_LEN_UTF8` for all `c: char`.
                // Therefore, `c.len_utf8() * 2` fits in usize for all `c: char`.
                (unsafe { open_delim.len_utf8().unchecked_mul(2) })
                    // why 2x? first for open delimiter, second for close delimiter (both are the same character)
                    .checked_add(n)
                    .expect(
                        "stringlike literal should include both open and close delimiters, \
                         which must have fit in memory in the original source code and therefore \
                         have a len that fits in usize",
                    )
            })
            .ok_or_else(|| {
                self.error_here(
                    self.source.len(),
                    // the fact there is a closing delimiter that didn't end the string shows it must be escaped
                    // (or else there wouldn't have been an error)
                    if rest.contains(open_delim) {
                        match open_delim {
                            '\'' => ErrorType::EscapedCharLiteralEnd,
                            '"' => ErrorType::EscapedStringLiteralEnd,
                            _ => unimplemented!(),
                        }
                    } else {
                        match open_delim {
                            '\'' => ErrorType::EndlessCharLiteral,
                            '"' => ErrorType::EndlessStringLiteral,
                            _ => unimplemented!(),
                        }
                    },
                )
            })
            .and_then(|len| {
                let src = self
                    .split_off(len)
                    .expect("find and len should return safe positions to split at");
                match open_delim {
                    STR_DELIM => TokenValue::string_literal(src).map(|val| Token {
                        src,
                        ty: TokenType::StringLiteral,
                        val,
                    }),

                    CHAR_DELIM => TokenValue::char_literal(src).map(|val| Token {
                        src,
                        ty: TokenType::CharLiteral,
                        val,
                    }),

                    _ => unreachable!("should be guarded by if condition"),
                }
                .map_err(|err| self.error_prev(len, err))
            })
    }

    /// The source code starts with [`TokenType::Macro`]
    fn starts_with_ident(&self) -> bool {
        self.source
            .starts_with(|ch: char| ch.is_alphabetic() || ch == '_')
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_ident(&mut self) -> Token<'a> {
        let len = self
            .source
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
            .unwrap_or(self.source.len());
        let src = self
            .split_off(len)
            .expect("find and len should return safe positions to split at");
        let (ty, val) = if let Some(kw) = Keyword::try_from_str(src) {
            (
                if matches!(kw.kw_type(), KeywordType::Control) {
                    TokenType::CtrlKeyword
                } else {
                    TokenType::Keyword
                },
                TokenValue::Keyword(kw),
            )
        }
        // assumes the token has already been split off
        else {
            match src {
                "true" => (TokenType::BoolLiteral, TokenValue::BoolLiteral(true)),
                "false" => (TokenType::BoolLiteral, TokenValue::BoolLiteral(false)),
                _ => (
                    if self.source.starts_with('(') {
                        TokenType::Callable
                    } else {
                        TokenType::Identifier
                    },
                    TokenValue::Direct(src),
                ),
            }
        };
        Token { src, ty, val }
    }

    /// The source code starts with [`TokenType::Macro`]
    fn starts_with_num_literal(&self) -> bool {
        self.source
            .strip_prefix('-')
            .filter(|_| self.can_be_negative)
            .unwrap_or(self.source)
            .starts_with(char::is_numeric)
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_num_literal(&mut self) -> Result<Token<'a>, ContextError<'a>> {
        let number_end = {
            let mut is_first_char = true;
            let mut is_first_decimal = true; // at most one decimal
            let mut is_first_e_neg = true; // at most one '-' following an 'e'
            let mut is_prev_e = false;
            let mut is_following_e = false;
            move |ch: char| {
                let is_end = !(ch.is_alphanumeric()
                    || ch == '.' && std::mem::take(&mut is_first_decimal) && !is_following_e
                    || ch == '-'
                        && (is_first_char || is_prev_e && std::mem::take(&mut is_first_e_neg)));
                is_prev_e = matches!(ch, 'e' | 'E');
                is_following_e |= is_prev_e;
                is_first_char = false;
                is_end
            }
        };
        let number = self
            .source
            .split_once(number_end)
            .map_or(self.source, |(pre, _)| pre);
        // skip trailing decimal or hyphen; decimal could be a method, hyphen could be subtraction operator.
        // trailing 'e' is kept since it should be an error, rather than being left in for the next token.
        let len = number.trim_end_matches(['.', '-']).len();
        let src = self
            .split_off(len)
            .expect("should be a safe position to split at");
        TokenValue::number_literal(src)
            .map(|val| Token {
                src,
                ty: TokenType::NumberLiteral,
                val,
            })
            .map_err(|err| self.error_prev(len, err))
    }

    /// The source code starts with [`TokenType::Macro`]
    fn starts_with_line_comment(&self) -> bool {
        self.source.starts_with(LINE_COMMENT_OPEN)
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_line_comment(&mut self) -> Token<'a> {
        let len = self
            .source
            .lines()
            .next() // take the first line (excluding newline/return)
            .expect("the existence of characters should imply the existence of a line")
            .len();
        Token {
            src: self
                .split_off(len)
                .expect("should be a safe position to split at"),
            ty: TokenType::Comment,
            val: TokenValue::Ignore,
        }
    }

    /// The source code starts with [`TokenType::Macro`]
    fn starts_with_block_comment(&self) -> bool {
        self.source.starts_with(BLOCK_COMMENT_OPEN)
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_block_comment(&mut self) -> Result<Token<'a>, ContextError<'a>> {
        const BLOCK_COMMENT_CIRCUMFIX_LEN: usize =
            BLOCK_COMMENT_OPEN.len() + BLOCK_COMMENT_CLOSE.len();
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
        len.map(|len| Token {
            src: self
                .split_off(len)
                .expect("should be a safe position to split at"),
            ty: TokenType::Comment,
            val: TokenValue::Ignore,
        })
        .ok_or_else(|| self.error_here(self.source.len(), ErrorType::EndlessBlockComment))
    }

    /// The source code starts with [`TokenType::Macro`]
    fn starts_with_punc(&self) -> bool {
        self.source
            .starts_with(|ch: char| ch.is_ascii_punctuation())
    }

    /// Split off a [`TokenType::Macro`] from the start of the source code
    ///
    /// # Panics
    /// This method is allowed to panic if [`Self::starts_with_macro`] would not have returned true
    fn scan_punc(&mut self) -> Result<Token<'a>, ContextError<'a>> {
        Punctuation::from_prefix(self.source)
            .map(|punc| {
                let src = self
                    .split_off(punc.as_str().len())
                    .expect("should be a safe position to split at");
                Token {
                    src,
                    ty: TokenType::Punctuation,
                    val: TokenValue::Punctuation(punc),
                }
            })
            .ok_or_else(|| {
                self.error_here(
                    self.source
                        .chars()
                        .next()
                        .expect("source should have at least one char to start with punctuation")
                        .len_utf8(),
                    ErrorType::UnknownToken,
                )
            })
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Result<Token<'a>, ContextError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // if there are no characters remaining, this will return None and stop iterating.
        self.source.chars().next().map(|ch| {
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

            if self.starts_with_whitespace() {
                Ok(self.scan_whitespace())
            } else if self.starts_with_macro() {
                Ok(self.scan_macro())
            } else if self.starts_with_macro_param() {
                Ok(self.scan_macro_param())
            } else if let Some(open_delim) = self.starts_with_strlike_literal() {
                self.scan_strlike_literal(open_delim)
            } else if self.starts_with_ident() {
                Ok(self.scan_ident())
            } else if self.starts_with_num_literal() {
                self.scan_num_literal()
            } else if self.starts_with_line_comment() {
                Ok(self.scan_line_comment())
            } else if self.starts_with_block_comment() {
                self.scan_block_comment()
            } else if self.starts_with_punc() {
                self.scan_punc()
            } else {
                Err(self.error_here(ch.len_utf8(), ErrorType::UnknownToken))
            }
            .inspect(|token| {
                // non-whitespace, non-comment token
                if !matches!(token.ty, TokenType::Whitespace | TokenType::Comment) {
                    // punctuation except for close bracket
                    self.can_be_negative = matches!(token.val, TokenValue::Punctuation(punc) if
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
pub fn tokenize(source: &str) -> Scanner<'_> {
    Scanner::new(source)
}
