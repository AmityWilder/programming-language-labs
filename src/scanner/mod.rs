use error::{ContextError, Error, ErrorType, NestedTokenResult, SimpleTokenResult};
use std::range::Range;
use symbols::*;
use token::{
    AllocTokenValue, InterpolatedExpr, InterpolatedString, Keyword, KeywordType, NestedTokenValue,
    NoAlloc, Punctuation, Token, TokenType, TokenValue,
};

pub mod error;
pub mod symbols;
pub mod token;

const fn unescaped(looking_for: char) -> impl FnMut(char) -> bool {
    let mut is_escaped = false;
    move |ch| {
        !is_escaped && ch == looking_for || {
            is_escaped = !is_escaped && ch == ESCAPE;
            false
        }
    }
}

/// Finds an instance of `close` that has not been paired with a `open`
const fn unbalanced(open: char, close: char) -> impl FnMut(char) -> bool {
    let mut depth: usize = 0;
    move |ch| {
        if ch == close {
            if let Some(n) = depth.checked_sub(1) {
                depth = n;
            } else {
                // depth must be 0
                return true;
            }
        } else if ch == open {
            depth = depth
                .checked_add(1)
                // TODO: should this be an error?
                .unwrap_or_else(|| panic!("cannot exceed depth of {}", usize::MAX));
        }
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scanner<'a> {
    /// This one doesn't get ripped apart
    original: &'a str,

    /// A reference to the original source code. Since this is only a copy, it will get ripped apart and fed to the tokens.
    /// The next token will always be at the start of this string.
    source: &'a str,

    /// The most recent non-whitespace, non-comment token was either the start of the source code or [`TokenType::Punctuation`]
    /// **and not** `)`, `]`, or `}`.
    can_be_negative: bool,
}

impl<'a> Scanner<'a> {
    pub const fn new(source: &'a str) -> Self {
        Self {
            original: source,
            source,
            // start off true because we are at the start of the source code
            can_be_negative: true,
        }
    }

    /// # Panics
    /// This method will panic if `len` splits `self.source` partway through a character or beyond the end of the source string.
    const fn split_off(&mut self, len: usize) -> &'a str {
        // I know `.map()` exists, but it isn't `const` yet and I like `const`.
        let (front, back) = self
            .source
            .split_at_checked(len)
            .expect("should have checked length");
        self.source = back;
        front
    }

    /// # Panics
    /// See [`Self::split_off`]
    const fn split_off_token(&mut self, len: usize, ty: TokenType) -> Token<'a> {
        Token {
            src: self.split_off(len),
            ty,
        }
    }

    /// Generate an error starting at the current (incomplete) token
    ///
    /// [Splits off](Self::split_off) the erroneous segment so we can find more errors
    fn error_here(&mut self, len: usize, err: ErrorType<'a>) -> Error<'a> {
        let err = Error {
            range: self
                .original
                .substr_range(&self.source[..len])
                .expect("source should be a substring of original"),
            err,
        };
        _ = self.split_off(1);
        err
    }

    /// Generate an error on the most recent (complete) token
    fn error_prev(&mut self, len: usize, err: ErrorType<'a>) -> Error<'a> {
        let end = self
            .original
            .substr_range(self.source)
            .expect("source should be a substring of original")
            .start;
        Error {
            range: Range {
                start: end.checked_sub(len).expect(
                    "len should be the size of a token that was split off from the source string",
                ),
                end,
            },
            err,
        }
    }

    fn starts_with_whitespace(&self) -> bool {
        self.source.starts_with(char::is_whitespace)
    }

    fn scan_whitespace(&mut self) -> Token<'a> {
        let len = self
            .source
            .find(|ch: char| !ch.is_whitespace())
            .unwrap_or(self.source.len());
        self.split_off_token(len, TokenType::Whitespace)
    }

    /// Returns the delimiter
    fn starts_with_strlike_literal(&self) -> Option<char> {
        self.source
            .chars()
            .next()
            .filter(|ch| matches!(*ch, STR_DELIM | CHAR_DELIM | INTERP_STR_DELIM))
    }

    fn scan_strlike_literal(&mut self, open_delim: char) -> Result<Token<'a>, Error<'a>> {
        let len = self.source[open_delim.len_utf8()..]
            .find(unescaped(open_delim))
            .map(|n| {
                // why 2x? first for open delimiter, second for close delimiter (both are the same character)
                // SAFETY: char::MAX_LEN_UTF8 * 2 fits in usize
                (unsafe { open_delim.len_utf8().unchecked_mul(2) })
                    .checked_add(n)
                    .expect(
                        "stringlike literal should include both open and close delimiters, \
                         which must have fit in memory in the original source code and therefore \
                         have a len that fits in usize",
                    )
            });
        len.map(|len| {
            self.split_off_token(
                len,
                match open_delim {
                    STR_DELIM => TokenType::StringLiteral,
                    CHAR_DELIM => TokenType::CharLiteral,
                    INTERP_STR_DELIM => TokenType::InterpolatedString,
                    _ => unreachable!("should be guarded by if condition"),
                },
            )
        })
        .ok_or_else(|| {
            self.error_here(
                self.source.len(),
                // the fact there is a closing delimiter that didn't end the string shows it must be escaped
                // (or else there wouldn't have been an error)
                if self.source[open_delim.len_utf8()..].contains(open_delim) {
                    ErrorType::EscapedStringLiteralEnd
                } else {
                    ErrorType::EndlessStringLiteral
                },
            )
        })
    }

    fn starts_with_ident(&self) -> bool {
        self.source
            .starts_with(|ch: char| ch.is_alphabetic() || ch == '_')
    }

    fn scan_ident(&mut self) -> Token<'a> {
        let len = self
            .source
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
            .unwrap_or(self.source.len());
        let src = self.split_off(len);
        Token {
            src,
            ty: if let Some(kw) = Keyword::from_str(src) {
                if matches!(kw.kw_type(), KeywordType::Control) {
                    TokenType::CtrlKeyword
                } else {
                    TokenType::Keyword
                }
            }
            // assumes the token has already been split off
            else if self.source.starts_with('(') {
                TokenType::Callable
            } else {
                TokenType::Identifier
            },
        }
    }

    fn starts_with_num_literal(&self) -> bool {
        self.source
            .strip_prefix('-')
            .filter(|_| self.can_be_negative)
            .unwrap_or(self.source)
            .starts_with(char::is_numeric)
    }

    fn scan_num_literal(&mut self) -> Token<'a> {
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
        let mut len = self.source.find(number_end).unwrap_or(self.source.len());
        // skip trailing decimal or hyphen; decimal could be a method, hyphen could be subtraction operator.
        // trailing 'e' is kept since it should be an error, rather than being left in for the next token.
        len = self.source[..len].trim_end_matches(['.', '-']).len();
        self.split_off_token(len, TokenType::NumberLiteral)
    }

    fn starts_with_line_comment(&self) -> bool {
        self.source.starts_with(LINE_COMMENT_OPEN)
    }

    fn scan_line_comment(&mut self) -> Token<'a> {
        let len = self
            .source
            .lines()
            .next() // take the first line (excluding newline/return)
            .expect("the existence of characters should imply the existence of a line")
            .len();
        self.split_off_token(len, TokenType::Comment)
    }

    fn starts_with_block_comment(&self) -> bool {
        self.source.starts_with(BLOCK_COMMENT_OPEN)
    }

    fn scan_block_comment(&mut self) -> Result<Token<'a>, Error<'a>> {
        const BLOCK_COMMENT_CIRCUMFIX_LEN: usize =
            BLOCK_COMMENT_OPEN.len() + BLOCK_COMMENT_CLOSE.len();
        let mut prev_char = None;
        let mut depth: usize = 0;
        let len = self.source[BLOCK_COMMENT_OPEN.len()..]
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
        len.map(|len| self.split_off_token(len, TokenType::Comment))
            .ok_or_else(|| self.error_here(self.source.len(), ErrorType::EndlessBlockComment))
    }

    fn starts_with_punc(&self) -> bool {
        self.source
            .starts_with(|ch: char| ch.is_ascii_punctuation())
    }

    fn scan_punc(&mut self) -> Result<Token<'a>, Error<'a>> {
        let len = Punctuation::from_prefix(self.source).map(|x| x.as_str().len());
        len.map(|len| self.split_off_token(len, TokenType::Punctuation))
            .ok_or_else(|| self.error_here(1, ErrorType::UnknownToken))
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Result<(Token<'a>, TokenValue<'a, NoAlloc>), ContextError<'a>>;

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
            } else if let Some(open_delim) = self.starts_with_strlike_literal() {
                self.scan_strlike_literal(open_delim)
            } else if self.starts_with_ident() {
                Ok(self.scan_ident())
            } else if self.starts_with_num_literal() {
                Ok(self.scan_num_literal())
            } else if self.starts_with_line_comment() {
                Ok(self.scan_line_comment())
            } else if self.starts_with_block_comment() {
                self.scan_block_comment()
            } else if self.starts_with_punc() {
                self.scan_punc()
            } else {
                Err(self.error_here(ch.len_utf8(), ErrorType::UnknownToken))
            }
            .and_then(|tkn| {
                tkn.value_noalloc()
                    .map(|val| (tkn, val))
                    .map_err(|err| self.error_prev(tkn.src.len(), err))
            })
            .map_err(|e| e.add_context(self.original))
            .inspect(|(token, _)| {
                // non-whitespace, non-comment token
                if !matches!(token.ty, TokenType::Whitespace | TokenType::Comment) {
                    // punctuation except for close bracket
                    self.can_be_negative = matches!(token.ty, TokenType::Punctuation)
                        && !matches!(token.src, ")" | "]" | "}");
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

fn tokenize_uninterpolated(tokens: Scanner<'_>) -> impl Iterator<Item = SimpleTokenResult<'_>> {
    tokens.map(|item| {
        item.map(|(token, value)| {
            let value = NestedTokenValue::from(
                AllocTokenValue::<Scanner>::try_from(value)
                    .expect("should have been caught by scanner"),
            );
            (token, value)
        })
    })
}

/// Create a [`Scanner`] for the provided source code, and contextualize errors if there are any
pub fn tokenize(source: &str) -> impl Iterator<Item = NestedTokenResult<'_>> {
    Scanner::new(source).map(|item| {
        item.map(|(token, value)| {
            let value = match AllocTokenValue::<Scanner<'_>>::try_from(value)
                .expect("should have been caught by scanner")
            {
                TokenValue::InterpolatedString(InterpolatedString { text, expressions }) => {
                    TokenValue::InterpolatedString(InterpolatedString {
                        text,
                        expressions: expressions
                            .into_iter()
                            .map(
                                |InterpolatedExpr {
                                     range,
                                     position,
                                     mut expr,
                                 }| InterpolatedExpr {
                                    range,
                                    position,
                                    expr: {
                                        expr.original = source;
                                        tokenize_uninterpolated(expr).collect()
                                    },
                                },
                            )
                            .collect(),
                    })
                }
                TokenValue::Ignore => TokenValue::Ignore,
                TokenValue::UIntLiteral(x) => TokenValue::UIntLiteral(x),
                TokenValue::SIntLiteral(x) => TokenValue::SIntLiteral(x),
                TokenValue::FltLiteral(x) => TokenValue::FltLiteral(x),
                TokenValue::CharLiteral(x) => TokenValue::CharLiteral(x),
                TokenValue::StringLiteral(x) => TokenValue::StringLiteral(x),
                TokenValue::Direct(x) => TokenValue::Direct(x),
                TokenValue::Keyword(x) => TokenValue::Keyword(x),
                TokenValue::Punctuation(x) => TokenValue::Punctuation(x),
            };
            (token, value)
        })
    })
}
