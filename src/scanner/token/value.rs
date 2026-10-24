//! [`LexValue`] of tokens

pub use super::{keyword::Keyword, punc::Punctuation};
use crate::{
    error::{ErrorType, IntKind, NumErrorKind},
    scanner::{
        symbols::{
            BIN_PREFIX, BYTE_SUFFIX, CHAR_DELIM, ESCAPE, HEX_PREFIX, OCT_PREFIX, SIGNED_SUFFIX,
            TEXT_DELIM, UNSIGNED_SUFFIX,
        },
        token::{escape_char, escape_seq},
    },
    types::{byte, flpt, sint, uint},
};
use std::{borrow::Cow, range::Range};

/// Information about a character literal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CharLiteral {
    /// The character being represented
    pub ch: char,

    /// Whether the character is an escape sequence in the lexeme
    pub is_escaped: bool,
}

/// Non-allocated version of [`LiteralText`]
///
/// Exists purely to provide member functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextLiteral<'src> {
    /// The string literal without delimiters - the value, if it has no escapes.
    /// May contain unconverted escape sequences
    pub content: &'src str,
}

impl<'src> TextLiteral<'src> {
    /// Identify whether a string literal contains escape sequences.
    pub fn has_escapes(&self) -> bool {
        self.content.contains(ESCAPE)
    }

    /// Replace escape sequences with their meanings
    ///
    /// Returns [`Err`] if an escape sequence is invalid
    pub fn process(&self) -> Result<Cow<'src, str>, &'src str> {
        if self.has_escapes() {
            let mut processed = String::with_capacity(self.content.len());
            let mut prev_end = 0;
            for res in Escapes::new(self.content) {
                let (range, ch) = res?;
                processed.push_str(
                    self.content.get(prev_end..range.start).expect(
                        "prev_end and range.start should be valid positions within content",
                    ),
                );
                prev_end = range.end;
                processed.push(ch);
            }
            processed.push_str(
                self.content
                    .get(prev_end..)
                    .expect("prev_end should be a valid position in content"),
            );
            Ok(Cow::Owned(processed))
        } else {
            Ok(Cow::Borrowed(self.content))
        }
    }
}

/// A [`LexValue`] where the lexeme can be represented by a static string
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum TokenEq {
    Keyword(Keyword),
    Punctuation(Punctuation),
}

impl TokenEq {
    pub const fn as_str(self) -> &'static str {
        match self {
            TokenEq::Keyword(kw) => kw.as_str(),
            TokenEq::Punctuation(punc) => punc.as_str(),
        }
    }

    // pub fn op_description(self, operands: &[ValueType]) -> &'static str {
    //     match self {
    //         TokenEq::Keyword(kw) => kw.op_description(operands),
    //         TokenEq::Punctuation(punc) => punc.op_description(operands),
    //     }
    // }
}

impl std::fmt::Display for TokenEq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

/// The value represented by a [`Token`](super::Token)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LexValue<'src> {
    // ignored
    /// An entire chunk of whitespace, not just one character
    #[default]
    Whitespace,
    /// A comment (either block or line)
    Comment,

    // number literal
    /// Byte literal
    ByteLiteral(byte),
    // number literal
    /// Unsigned integer literal
    UIntLiteral(uint),
    /// Signed integer literal
    SIntLiteral(sint),
    /// Floating point literal
    FlptLiteral(flpt),

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
    pub const fn to_token_eq(self) -> Option<TokenEq> {
        match self {
            LexValue::Keyword(kw) => Some(TokenEq::Keyword(kw)),
            LexValue::Punctuation(punc) => Some(TokenEq::Punctuation(punc)),
            _ => None,
        }
    }

    /// Parses a number literal lexeme into its value
    ///
    /// [`Err`] is [`ErrorType::InvalidNumLiteral`]
    pub fn number_literal(src: &'src str) -> Result<Self, NumErrorKind> {
        // checking the start of a string is easier than looking through every one of its characters, so it goes first.
        // hexadecimal is the only case in which an 'e' might appear while NOT being a float.
        if !src.starts_with(HEX_PREFIX) && src.contains(['e', 'E']) || src.contains('.') {
            debug_assert_ne!(src, "", "scanner should not emit empty tokens"); // also guarded by condition
            src.replace('_', "") // TODO: this allocates :c
                .parse() // turns out parse already handles the "e" syntax on its own
                .map(Self::FlptLiteral)
                .map_err(|_| NumErrorKind::InvalidFlpt)
        } else {
            enum SignSuffix {
                Signed,
                Unsigned,
            }

            let (with_suffix, sign_suffix, bits_suffix) = [
                (SIGNED_SUFFIX, SignSuffix::Signed, sint::BITS),
                (UNSIGNED_SUFFIX, SignSuffix::Unsigned, uint::BITS),
                (BYTE_SUFFIX, SignSuffix::Unsigned, byte::BITS), // byte is always unsigned
            ]
            .into_iter()
            .find_map(|(suf, sign, bits)| src.strip_suffix(suf).map(|pre| (pre, Some(sign), bits)))
            .unwrap_or((src, None, uint::BITS));

            let stripped = with_suffix.strip_prefix('-');
            let is_negative = stripped.is_some();
            let magnitude = stripped.unwrap_or(with_suffix);

            // default to signed
            // users should explicitly confirm they are doing unsigned math for safety reason (e.g. indexing).
            // signed math shouldn't need to be filled with suffixes.
            let is_signed = matches!(sign_suffix, Some(SignSuffix::Signed) | None);

            // cannot mix explicit unsigned with negative sign
            if matches!(sign_suffix, Some(SignSuffix::Signed)) && is_negative {
                return Err(NumErrorKind::NegUnsigned);
            }

            let (digits, radix) = [(HEX_PREFIX, 16), (OCT_PREFIX, 8), (BIN_PREFIX, 2)]
                .into_iter()
                .find_map(|(prefix, radix)| magnitude.strip_prefix(prefix).map(|n| (n, radix)))
                .unwrap_or((magnitude, 10));

            if digits.is_empty() {
                // for digits to be empty, we must have something like "0x".
                // this is a valid prefix for a number, but without a number following it,
                // we treat it like "0" with the suffix "x" (which isn't allowed).
                return Err(NumErrorKind::InvalidInt);
            }

            // yes there's a lot of duplication. annoyingly, generic constants can't be used in array sizes.
            if is_signed {
                match bits_suffix {
                    sint::BITS => {
                        // prefix with a hyphen, underscores stripped
                        #[expect(
                            clippy::as_conversions,
                            reason = "int_primitive::MAX.ilog2() is at most 64, which clearly fits in 16 bits"
                        )]
                        let mut buf = [b'\0'; sint::MAX.ilog2() as usize + 2];

                        let digits = join_parts(
                            &mut buf,
                            std::iter::chain(is_negative.then_some("-"), digits.split('_')),
                        )
                        // excessive digits, probably too large of an integer
                        .ok_or(if is_negative {
                            NumErrorKind::NegOverflow(IntKind::Sint)
                        } else {
                            NumErrorKind::PosOverflow(IntKind::Sint)
                        })?;

                        sint::from_str_radix(digits, radix)
                            .map(Self::SIntLiteral)
                            .map_err(|e| NumErrorKind::new_signed(*e.kind()))
                    }

                    _ => unimplemented!(),
                }
            } else {
                match bits_suffix {
                    uint::BITS => {
                        // underscores stripped
                        #[expect(
                            clippy::as_conversions,
                            reason = "int_primitive::MAX.ilog2() is at most 64, which clearly fits in 16 bits"
                        )]
                        let mut buf = [b'\0'; uint::MAX.ilog2() as usize + 1];

                        let digits = join_parts(&mut buf, digits.split('_'))
                            // excessive digits, probably too large of an integer
                            .ok_or(NumErrorKind::PosOverflow(IntKind::Uint))?;

                        uint::from_str_radix(digits, radix)
                            .map(Self::UIntLiteral)
                            .map_err(|e| NumErrorKind::new_unsigned(*e.kind()))
                    }

                    byte::BITS => {
                        // underscores stripped
                        #[expect(
                            clippy::as_conversions,
                            reason = "int_primitive::MAX.ilog2() is at most 64, which clearly fits in 16 bits"
                        )]
                        let mut buf = [b'\0'; byte::MAX.ilog2() as usize + 1];

                        let digits = join_parts(&mut buf, digits.split('_'))
                            // excessive digits, probably too large of an integer
                            .ok_or(NumErrorKind::PosOverflow(IntKind::Uint))?;

                        byte::from_str_radix(digits, radix)
                            .map(Self::ByteLiteral)
                            .map_err(|e| NumErrorKind::new_unsigned(*e.kind()))
                    }

                    _ => unimplemented!(),
                }
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
    ///
    /// Returns [`Err`] if there is an invalid escape sequence
    pub fn text_literal(src: &'src str) -> Result<Self, &'src str> {
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
    original: &'src str,
    source: &'src str,
}

impl<'src> Escapes<'src> {
    /// Construct a new [`Escapes`] iterator from a source string
    #[must_use]
    pub const fn new(original: &'src str) -> Self {
        Self {
            original,
            source: original,
        }
    }
}

impl<'src> Iterator for Escapes<'src> {
    type Item = Result<(Range<usize>, char), &'src str>;

    fn next(&mut self) -> Option<Self::Item> {
        self.source.find(ESCAPE).map(|pos| {
            #[expect(clippy::string_slice, reason = "find should produce valid positions")]
            let next_esc = &self.source[pos..];
            let item = escape_seq(
                self.original,
                self.original
                    .substr_range(next_esc)
                    .expect("source should be a substr of original")
                    .start,
            );
            if let Ok((range, _)) = item {
                self.source = self.original.get(range.end..).expect(
                    "escape sequence range's end should be a valid position within original",
                );
            } else {
                self.source = next_esc
                    .strip_prefix(ESCAPE)
                    .expect("next_esc should start at the position of an escape character");
            }
            item
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

macro_rules! define_operators {
    (
        $(#[$em:meta])*
        $vis:vis enum $Enum:ident : $Base:ident {$(
            $(#[$vm:meta])*
            $BaseVariant:ident::$Variant:ident
        ),* $(,)?}
    ) => {
        $(#[$em])*
        $vis enum $Enum {$(
            $(#[$vm])*
            $Variant
        ),*}

        impl Operator {
            pub const fn from_token_eq(value: $Base) -> Option<Self> {
                $(use $BaseVariant::$Variant;)*
                Some(match value {
                    $($Base::$BaseVariant($Variant) => Self::$Variant,)*
                    _ => return None,
                })
            }

            pub const fn to_token_eq(self) -> $Base {
                $(use $BaseVariant::$Variant;)*
                match self {
                    $(Self::$Variant => $Base::$BaseVariant($Variant),)*
                }
            }
        }
    };
}

define_operators! {
/// Subset of [`TokenEq`] specific to operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Operator : TokenEq {
    #[default]
    Punctuation::Assign,

    Punctuation::And,
    Punctuation::Or,
    Punctuation::Nand,
    Punctuation::Nor,
    Punctuation::Xor,
    Punctuation::Xnor,

    Punctuation::AndAssign,
    Punctuation::OrAssign,
    Punctuation::XorAssign,
    Punctuation::NandAssign,
    Punctuation::NorAssign,
    Punctuation::XnorAssign,

    Punctuation::Not,
    Punctuation::Exists,

    Punctuation::Add,
    Punctuation::Incr,
    Punctuation::SubNeg,
    Punctuation::Decr,
    Punctuation::Mul,
    Punctuation::Pow,
    Punctuation::Div,
    Punctuation::Rem,

    Punctuation::AddAssign,
    Punctuation::SubAssign,
    Punctuation::MulAssign,
    Punctuation::PowAssign,
    Punctuation::DivAssign,
    Punctuation::RemAssign,

    Punctuation::Shl,
    Punctuation::Shr,
    Punctuation::Rotl,
    Punctuation::Rotr,

    Punctuation::ShlAssign,
    Punctuation::ShrAssign,
    Punctuation::RotlAssign,
    Punctuation::RotrAssign,

    Punctuation::Lt,
    Punctuation::Gt,
    Punctuation::Ne,
    Punctuation::Le,
    Punctuation::Eq,
    Punctuation::Ge,

    Punctuation::Coalesce,
    Punctuation::Ref,

    Punctuation::Convert,
    Punctuation::Transmute,

    Keyword::Szof,
}
}

impl Operator {
    /// Alias for [`Self::SubNeg`] to better imply what the code is doing
    #[expect(non_upper_case_globals)]
    pub const Sub: Self = Self::SubNeg;

    /// Alias for [`Self::SubNeg`] to better imply what the code is doing
    #[expect(non_upper_case_globals)]
    pub const Neg: Self = Self::SubNeg;

    pub const fn as_str(self) -> &'static str {
        self.to_token_eq().as_str()
    }

    // pub fn op_description(self, operands: &[ValueType]) -> &'static str {
    //     self.to_token_eq().op_description(operands)
    // }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl TokenEq {
    pub const fn to_operator(self) -> Option<Operator> {
        Operator::from_token_eq(self)
    }
}

impl From<Operator> for TokenEq {
    fn from(value: Operator) -> Self {
        value.to_token_eq()
    }
}

#[cfg(test)]
mod escapes_tests {
    use super::*;

    #[test]
    fn test_esc_esc() {
        const SOURCE: &str = r"apple \\foo\0";
        const EXPECT: [Result<(Range<usize>, char), &str>; 2] = [
            Ok((
                Range {
                    start: "apple ".len(),
                    end: r"apple \\".len(),
                },
                '\\',
            )),
            Ok((
                Range {
                    start: r"apple \\foo".len(),
                    end: r"apple \\foo\0".len(),
                },
                '\0',
            )),
        ];
        let actual: Vec<_> = Escapes::new(SOURCE).collect();
        assert_eq!(actual, EXPECT);
    }
}
