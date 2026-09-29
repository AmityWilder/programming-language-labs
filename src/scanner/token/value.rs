use std::{range::Range, sync::LazyLock};

use crate::{
    error::{ErrorType, NumLitError},
    scanner::{
        symbols::{BIN_PREFIX, CHAR_DELIM, ESCAPE, HEX_PREFIX, OCT_PREFIX, STR_DELIM},
        token::{escape_char, escape_seq, keyword::Keyword, punc::Punctuation},
    },
};

/// A `IntErrorKind::NegOverflow`, since those are private :/
static NEG_UNDERFLOW: LazyLock<std::num::TryFromIntError> = LazyLock::new(|| {
    #[allow(clippy::as_conversions, reason = "into is not const")]
    #[expect(clippy::invalid_upcast_comparisons, reason = "further proves my point")]
    const {
        assert!(i16::MIN < i8::MIN as i16, "proof. i16::MIN < i8::MIN");
    }
    // SAFETY: `i16::MIN < i8::MIN`. Because it is `<` and not `<=`, and both are integers,
    // there must be a difference of at least 1.
    i8::try_from(unsafe { i16::from(i8::MIN).unchecked_sub(1) })
        .expect_err("should result in negative overflow")
});

/// Information about a character literal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CharLiteral {
    /// The character being represented
    pub ch: char,

    /// Whether the character is an escape sequence in the lexeme
    pub is_escaped: bool,
}

/// No-alloc version of [`StringLiteral`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StrLiteral<'src> {
    /// The string literal without delimiters - the value if it has no escapes.
    /// May contain unconverted escape sequences
    pub content: &'src str,
}

impl StrLiteral<'_> {
    /// Identify whether a string literal contains escape sequences.
    pub fn has_escapes(&self) -> bool {
        self.content.contains(ESCAPE)
    }
}

/// Information about a string literal
///
/// Allocating version of [`StrLiteral`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct StringLiteral {
    /// The text content of the string literal; escape sequences converted and delimiters excluded.
    pub text: String,

    /// Ranges of the original lexeme (quote delimiters excluded) that refer to escape sequences
    pub escapes: Vec<Range<usize>>,
}

impl<'src> TryFrom<StrLiteral<'src>> for StringLiteral {
    type Error = ErrorType<'src>;

    fn try_from(value: StrLiteral<'src>) -> Result<Self, Self::Error> {
        if value.has_escapes() {
            let replacements = Escapes::new(value.content).collect::<Result<Vec<_>, _>>()?;
            let escapes = replacements.iter().map(|(range, _)| *range).collect();

            let byte_diff: usize = replacements
                .iter()
                .map(|(range, ch)| {
                    (range
                        .end
                        .checked_sub(range.start)
                        .expect("range should be ascending order"))
                    .checked_sub(ch.len_utf8())
                    .expect("should not be replacing an empty range")
                })
                .sum();
            let mut processed = String::with_capacity(
                value
                    .content
                    .len()
                    .checked_sub(byte_diff)
                    .expect("should only be removing bytes, not adding"),
            );
            let mut prev_end = 0;
            for (range, repl) in replacements {
                processed.push_str(value.content.get(prev_end..range.start)
                    .expect("range should never start/end within a UTF-8 character, and prev_end should always be from such a range (or 0)"));
                processed.push(repl);
                prev_end = range.end;
            }

            Ok(StringLiteral {
                text: processed,
                escapes,
            })
        } else {
            Ok(StringLiteral {
                text: value.content.to_string(),
                escapes: Vec::new(),
            })
        }
    }
}

/// The value represented by a [`Token`]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Value<'src> {
    // ignored
    /// An entire chunk of whitespace, not just one character
    #[default]
    Whitespace,
    /// A comment (either block or line)
    Comment,

    // number literal
    /// Unsigned integer literal
    UIntLiteral(usize),
    /// Signed integer literal
    SIntLiteral(isize),
    /// Floating point literal
    FltLiteral(f64),

    // strlike literal
    /// Character literal
    CharLiteral(CharLiteral),
    /// String literal
    StringLiteral(StrLiteral<'src>),

    // language builtin
    /// Boolean literal
    BoolLiteral(bool),

    // identifiers
    /// An identifier - its value is the lexeme itself
    Identifier,
    /// An identifier followed by `(` or following a `fn` keyword
    #[deprecated(note = "will be determined by parser in future versions")]
    Callable,
    /// An identifier prefixed with `\`
    Macro,
    /// An identifier prefixed with `$`
    MacroParam,

    /// A language keyword
    Keyword(Keyword),
    /// Operators and other non-alphanumeric tokens
    Punctuation(Punctuation),
}

impl<'src> Value<'src> {
    /// Parses a number literal lexeme into its value
    pub fn number_literal(src: &'src str) -> Result<Self, ErrorType<'src>> {
        // checking the start of a string is easier than looking through every one of its characters, so it goes first.
        // hexadecimal is the only case in which an 'e' might appear while NOT being a float.
        if !src.starts_with(HEX_PREFIX) && src.contains(['e', 'E']) || src.contains('.') {
            src.parse() // turns out parse already handles the "e" syntax on its own
                .map(Self::FltLiteral)
                .map_err(|e| ErrorType::InvalidNumLiteral(NumLitError::Flt(e)))
        } else {
            let stripped = src.strip_prefix('-');
            let is_negative = stripped.is_some();
            let magnitude = stripped.unwrap_or(src);

            let (digits, radix) = if let Some(n) = magnitude.strip_prefix(HEX_PREFIX) {
                (n, 16)
            } else if let Some(n) = magnitude.strip_prefix(OCT_PREFIX) {
                (n, 8)
            } else if let Some(n) = magnitude.strip_prefix(BIN_PREFIX) {
                (n, 2)
            } else {
                (magnitude, 10)
            };
            usize::from_str_radix(digits, radix)
                .map_err(|e| ErrorType::InvalidNumLiteral(NumLitError::UInt(e)))
                .and_then(|value| {
                    if is_negative {
                        (isize::try_from(value)
                            .map_err(|e| ErrorType::InvalidNumLiteral(NumLitError::SInt(e)))
                            .and_then(|x| {
                                x.checked_neg().ok_or_else(|| {
                                    ErrorType::InvalidNumLiteral(NumLitError::SInt(*NEG_UNDERFLOW))
                                })
                            }))
                        .map(Self::SIntLiteral)
                    } else {
                        Ok(Self::UIntLiteral(value))
                    }
                })
        }
    }

    /// Parses a character literal lexeme into its value
    pub fn char_literal(src: &'src str) -> Result<Self, ErrorType<'src>> {
        let src = src
            .strip_circumfix(CHAR_DELIM, CHAR_DELIM)
            .expect("character literal tokens should include delimiters (`'`)");
        if let Some((len, res)) = escape_char(src) {
            // escape sequence
            res.map_err(|()| ErrorType::InvalidEscape(src))
                .and_then(|ch| {
                    (len == src.len())
                        .then_some(Self::CharLiteral(CharLiteral {
                            ch,
                            is_escaped: true,
                        }))
                        .ok_or(ErrorType::MultiCharLiteral)
                })
        } else {
            // normal character
            let mut iter = src.chars();
            iter.next()
                .ok_or(ErrorType::EmptyCharLiteral)
                .and_then(|ch| {
                    iter.next()
                        .is_none()
                        .then_some(Self::CharLiteral(CharLiteral {
                            ch,
                            is_escaped: false,
                        }))
                        .ok_or(ErrorType::MultiCharLiteral)
                })
        }
    }

    /// Parses a string literal lexeme into its value, without allocating
    pub fn string_literal(src: &'src str) -> Result<Self, ErrorType<'src>> {
        let content = src
            .strip_circumfix(STR_DELIM, STR_DELIM)
            .expect("string literal tokens should include delimiters (`\"`)");
        let mut is_esc = false;
        if content.contains(ESCAPE)
            && let Some(e) = content
                .match_indices(|ch: char| {
                    is_esc = !is_esc && ch == ESCAPE;
                    is_esc
                })
                .find_map(|(i, _)| escape_seq(content, i).err())
        {
            Err(e)
        } else {
            Ok(Self::StringLiteral(StrLiteral { content }))
        }
    }
}

/// An iterator over unescaped escape characters ([`ESCAPE`]) in a string
#[derive(Debug, Clone)]
pub struct Escapes<'src> {
    /// Source code
    source: &'src str,
    /// Tracking of offset into [`Self::source`]
    offset: usize,
    /// Whether the upcoming character is escaped
    is_esc: bool,
}

impl<'src> Escapes<'src> {
    /// Construct a new [`Escapes`] iterator from a source string
    #[must_use]
    pub const fn new(source: &'src str) -> Self {
        Self {
            source,
            offset: 0,
            is_esc: false,
        }
    }
}

impl<'src> Iterator for Escapes<'src> {
    type Item = Result<(Range<usize>, char), ErrorType<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.source
            .get(self.offset..)
            .expect("offset should never be within a UTF-8 character")
            .match_indices(|ch: char| {
                self.is_esc = !self.is_esc && ch == ESCAPE;
                self.is_esc
            })
            .next()
            .map(|(i, _)| {
                self.offset = self.offset.checked_add(i).expect(
                    "`i` should be a position after `offset` in `source`, \
                     a string in memory whose len must fit in usize",
                );
                escape_seq(self.source, self.offset)
            })
    }
}
