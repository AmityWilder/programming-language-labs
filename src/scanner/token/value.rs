//! [`Value`] of tokens

use crate::{
    error::{ErrorType, NumErrorKind},
    scanner::{
        symbols::{
            BIN_PREFIX, CHAR_DELIM, ESCAPE, HEX_PREFIX, OCT_PREFIX, SIGNED_SUFFIX, TEXT_DELIM,
            UNSIGNED_SUFFIX,
        },
        token::{escape_char, escape_seq, keyword::Keyword, punc::Punctuation},
    },
};
use std::range::Range;

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
pub struct TextLiteral<'src> {
    /// The string literal without delimiters - the value if it has no escapes.
    /// May contain unconverted escape sequences
    pub content: &'src str,
}

impl<'src> TextLiteral<'src> {
    /// Identify whether a string literal contains escape sequences.
    pub fn has_escapes(&self) -> bool {
        self.content.contains(ESCAPE)
    }

    /// Process the [`TextLiteral`] into a [`StringLiteral`]
    pub fn process(&self) -> Result<StringLiteral, ErrorType<'src>> {
        StringLiteral::try_from(*self)
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

impl<'src> TryFrom<TextLiteral<'src>> for StringLiteral {
    type Error = ErrorType<'src>;

    fn try_from(value: TextLiteral<'src>) -> Result<Self, Self::Error> {
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
pub enum LexValue<'src> {
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
    FracLiteral(f64),

    // strlike literal
    /// Character literal
    CharLiteral(CharLiteral),
    /// String literal
    TextLiteral(TextLiteral<'src>),

    // language builtin
    /// Boolean literal
    BoolLiteral(bool),

    // identifiers
    /// An identifier - its value is the lexeme itself
    Identifier,
    /// An identifier followed by `(` or following a `fn` keyword
    // TODO: #[deprecated(note = "will be determined by parser in future versions")]
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

impl<'src> LexValue<'src> {
    /// Parses a number literal lexeme into its value
    pub fn number_literal(src: &'src str) -> Result<Self, ErrorType<'src>> {
        // checking the start of a string is easier than looking through every one of its characters, so it goes first.
        // hexadecimal is the only case in which an 'e' might appear while NOT being a float.
        if !src.starts_with(HEX_PREFIX) && src.contains(['e', 'E']) || src.contains('.') {
            debug_assert_ne!(src, "", "scanner should not emit empty tokens"); // also guarded by condition
            src.replace('_', "") // TODO: this allocates :c
                .parse() // turns out parse already handles the "e" syntax on its own
                .map(Self::FracLiteral)
                .map_err(|_| ErrorType::InvalidNumLiteral(NumErrorKind::InvalidFrac))
        } else {
            enum SignSuffix {
                Signed,
                Unsigned,
            }

            let (with_suffix, sign_suffix) = [
                (SIGNED_SUFFIX, SignSuffix::Signed),
                (UNSIGNED_SUFFIX, SignSuffix::Unsigned),
            ]
            .into_iter()
            .find_map(|(suf, sign)| src.strip_suffix(suf).map(|pre| (pre, Some(sign))))
            .unwrap_or((src, None));

            let stripped = with_suffix.strip_prefix('-');
            let is_negative = stripped.is_some();
            let magnitude = stripped.unwrap_or(with_suffix);

            // default to signed
            // users should explicitly confirm they are doing unsigned math for safety reason (e.g. indexing).
            // signed math shouldn't need to be filled with suffixes.
            let is_signed = matches!(sign_suffix, Some(SignSuffix::Signed) | None);

            // cannot mix explicit unsigned with negative sign
            if matches!(sign_suffix, Some(SignSuffix::Signed)) && is_negative {
                return Err(ErrorType::InvalidNumLiteral(NumErrorKind::NegUnsigned));
            }

            let (digits, radix) = [(HEX_PREFIX, 16), (OCT_PREFIX, 8), (BIN_PREFIX, 2)]
                .into_iter()
                .find_map(|(prefix, radix)| magnitude.strip_prefix(prefix).map(|n| (n, radix)))
                .unwrap_or((magnitude, 10));

            if digits.is_empty() {
                // for digits to be empty, we must have something like "0x".
                // this is a valid prefix for a number, but without a number following it,
                // we treat it like "0" with the suffix "x" (which isn't allowed).
                return Err(ErrorType::InvalidNumLiteral(NumErrorKind::InvalidInt));
            }

            if is_signed {
                // prefix with a hyphen, underscores stripped
                #[expect(clippy::as_conversions)]
                let mut buf = [b'\0'; isize::MAX.ilog2() as usize + 2];

                let digits = join_parts(
                    &mut buf,
                    std::iter::chain(is_negative.then_some("-"), digits.split('_')),
                )
                // excessive digits, probably too large of an integer
                .ok_or(ErrorType::InvalidNumLiteral(if is_negative {
                    NumErrorKind::SNegOverflow
                } else {
                    NumErrorKind::SPosOverflow
                }))?;

                isize::from_str_radix(digits, radix)
                    .map(Self::SIntLiteral)
                    .map_err(|e| ErrorType::InvalidNumLiteral(NumErrorKind::new_signed(*e.kind())))
            } else {
                // underscores stripped
                #[expect(clippy::as_conversions)]
                let mut buf = [b'\0'; usize::MAX.ilog2() as usize + 1];

                let digits = join_parts(&mut buf, digits.split('_'))
                    // excessive digits, probably too large of an integer
                    .ok_or(ErrorType::InvalidNumLiteral(NumErrorKind::UPosOverflow))?;

                usize::from_str_radix(digits, radix)
                    .map(Self::UIntLiteral)
                    .map_err(|e| {
                        ErrorType::InvalidNumLiteral(NumErrorKind::new_unsigned(*e.kind()))
                    })
            }
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
    pub fn text_literal(src: &'src str) -> Result<Self, ErrorType<'src>> {
        let content = src
            .strip_circumfix(TEXT_DELIM, TEXT_DELIM)
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
            Ok(Self::TextLiteral(TextLiteral { content }))
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

/// Concatenates strings into a pre-allocated buffer
fn join_parts<'buf, 'p, I>(buf: &'buf mut [u8], parts: I) -> Option<&'buf str>
where
    I: IntoIterator<Item = &'p str>,
{
    let mut rest = &mut *buf;
    for part in parts {
        rest.split_off_mut(..part.len())?
            .copy_from_slice(part.as_bytes());
    }
    let leftover_len = rest.len();
    #[expect(clippy::arithmetic_side_effects, reason = "rest is a subset of buf")]
    let written_len = buf.len() - leftover_len;
    Some(
        #[expect(clippy::indexing_slicing, reason = "sum length of written segments")]
        std::str::from_utf8(&buf[..written_len])
            .expect("concatenation of valid UTF-8 segments should be valid UTF-8"),
    )
}
