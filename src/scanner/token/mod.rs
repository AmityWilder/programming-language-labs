//! Definitions of tokens and their values.

use crate::{
    error::ErrorType,
    scanner::{symbols::ESCAPE, token::value::LexValue},
};
use std::range::Range;

/// Helper macro for preventing issues with missed variants when adding new ones
///
/// Variants should be in the order they should be tested
macro_rules! define_token_eq {
    (
        $(#[$em:meta])*
        $vis:vis enum $Enum:ident = $name:ident {$(
            $(#[$vm:meta])*
            $Variant:ident = $(#[$valm:meta])* $value:literal
        ),+ $(,)?}
    ) => {
        $(#[$em])*
        $vis enum $Enum {$(
            #[doc = concat!("`", $value, "`\n")]
            $(#[$vm])*
            $Variant,
        )+}

        #[allow(dead_code, reason = "not always used in all expressions of this macro")]
        impl $Enum {
            /// Matches the prefix of `s` to a variant of [`Self`], finding the longest match possible.
            pub fn from_prefix(s: &str) -> Option<Self> {
                $name::OPTIONS
                    .into_iter()
                    .filter(|(pat, _)| s.starts_with(pat))
                    .max_by_key(|(pat, _)| pat.len()) // maximal munch
                    .map(|(_, punc)| punc)
            }

            /// Like [`Self::from_prefix`], but matches the full string
            pub const fn try_from_str(s: &str) -> Option<Self> {
                match s {
                    $($(#[$valm])* $value => Some(Self::$Variant),)+
                    _ => None,
                }
            }

            /// The constant string name of the token
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $value),+
                }
            }
        }

        impl std::fmt::Display for $Enum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        pub mod $name {
            use super::*;

            pub static OPTIONS: std::sync::LazyLock<[(&str, $Enum); [$($Enum::$Variant),+].len()]> = std::sync::LazyLock::new(||{
                let mut list: [(&'static str, $Enum); _] = [
                    $(($value, $Enum::$Variant),)+
                ];
                list.sort_by_key(|(name, _)| *name);
                list
            });

            #[cfg(test)]
            #[expect(non_snake_case)]
            mod tests {
                use super::*;

                $(#[test]
                fn $Variant() {
                    assert_eq!($Enum::from_prefix($value), Some($Enum::$Variant));
                })+
            }
        }
    };
}

pub mod keyword;
pub mod punc;
pub mod value;

/// A single token - its lexeme ([`Self::src`]) and type ([`Self::ty`]).
/// Does not contain the token's value, but can have the value obtained with [`Self::value_noalloc`].
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Token<'src> {
    /// Because this is a pointer into the original source string, we can use pointer arithmetic to find its location.
    /// If a program has a thousand tokens, why allocate a new string and store two additional integers in case of error
    /// when we can just keep the original string around and calculate those integers *on demand*?
    pub lex: &'src str,

    /// The value of the token
    pub val: LexValue<'src>,

    /// The range of the macro this token expanded from
    pub mac: Option<Range<usize>>,
}

impl std::fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { lex, val, mac } = self;
        let (name, dbg_val): (&str, Option<&dyn std::fmt::Debug>) = match val {
            LexValue::Whitespace => ("Whitespace", None),
            LexValue::Comment => ("Comment", None),
            LexValue::UIntLiteral(x) => ("UIntLiteral", Some(x)),
            LexValue::SIntLiteral(x) => ("SIntLiteral", Some(x)),
            LexValue::FracLiteral(x) => ("FltLiteral", Some(x)),
            LexValue::CharLiteral(x) => ("CharLiteral", Some(x)),
            LexValue::TextLiteral(x) => ("StringLiteral", Some(x)),
            LexValue::BoolLiteral(x) => ("BoolLiteral", Some(x)),
            LexValue::Identifier | LexValue::Callable => ("Identifier", None),
            LexValue::Macro => ("Macro", None),
            LexValue::MacroParam => ("MacroParam", None),
            LexValue::Keyword(x) => ("Keyword", Some(x)),
            LexValue::Punctuation(x) => ("Punctuation", Some(x)),
        };
        write!(f, "{name}({lex:?}")?;
        if let Some(mac) = mac {
            write!(f, " from {mac:?}")?;
        }
        write!(f, ")")?;
        if let Some(dbg_val) = dbg_val {
            write!(f, ": {dbg_val:?}")
        } else if !matches!(val, LexValue::Whitespace | LexValue::Comment) {
            write!(f, ": {lex}") // token value is its lexeme
        } else {
            Ok(())
        }
    }
}

impl<'src> Token<'src> {
    /// Returns the [`Range`] of `self.lex` in `source`
    /// # Panics
    /// This method will panic if `source` is not the source string of `self`
    pub fn lex_range(&self, source: &'src str) -> Range<usize> {
        source
            .substr_range(self.lex)
            .expect("every lexeme should be a substr of the source code")
    }
}

/// Returns [`None`] if `src` does not start with `\`
///
/// # Panics
/// This method can panic if its internal assumptions prove false
#[must_use]
pub fn escape_char(src: &str) -> Option<(usize, Result<char, ()>)> {
    let mut iter = src.chars();
    iter.next().filter(|ch| *ch == ESCAPE).map(|_| {
        let res = iter.next().ok_or(ESCAPE.len_utf8()).and_then(|ch| {
            const {
                assert!(
                    char::MAX_LEN_UTF8.checked_mul(2).is_some(),
                    "proof. 2 UTF8 characters are guaranteed not to exceed usize::MAX"
                );
            }
            // SAFETY: As shown above, `char::MAX_LEN_UTF8 * 2` fits in usize.
            // By definition of `char::MAX_LEN_UTF8`, `c.len_utf8()` is at most `char::MAX_LEN_UTF8` for all `c: char`.
            // Therefore, `c.len_utf8() + d.len_utf8()` fits in usize for all `c,d: char`.
            let base_len = unsafe { ESCAPE.len_utf8().unchecked_add(ch.len_utf8()) };
            match ch {
                '0'..='9' => Ok((base_len, char::from((u8::try_from(ch).expect("0-9 are ASCII and therefore 1 byte")).checked_sub(b'0').expect("0-9 are guaranteed to be within u8")))),

                'a' => Ok((base_len, '\x07')),
                'b' => Ok((base_len, '\x08')),
                'e' => Ok((base_len, '\x1b')),
                'f' => Ok((base_len, '\x0c')),
                'n' => Ok((base_len, '\n')),
                'r' => Ok((base_len, '\r')),
                't' => Ok((base_len, '\t')),
                'v' => Ok((base_len, '\x0b')),

                prefix @ ('x' | 'o' /* | 'b' */) => {
                    // digits = ceil(256.log(base))
                    // ilog rounds down but we want rounded up
                    let (digits, base) = match prefix {
                        'x' => (2, 16),
                        'o' => (3, 8),
                        // 'b' => (8, 2),
                        _ => unreachable!("guarded by outer branch"),
                    };
                    let num_start = base_len;
                    // ASCII digits
                    let end = num_start.checked_add(digits).expect("should be a subset of the existing string");
                    let len = base_len.checked_add(digits).expect("should be a subset of the existing string");
                    src.get(num_start..end)
                        .and_then(|n| u8::from_str_radix(n, base).ok())
                        .map(|num| (len, char::from(num)))
                        .ok_or(len)
                }

                // all other non-alphanumeric just output the literal symbol.
                // letters and numbers don't, because not all of them mean their literal symbol
                // and users shouldn't have to be confused why "\a \b \c" results in "\x07 \x08 c" instead of "a b c".
                _ if !ch.is_alphanumeric() => Ok((base_len, ch)),

                _ => Err(base_len),
            }
        });
        match res {
            Ok((len, ch)) => (len, Ok(ch)),
            Err(len) => (len, Err(())),
        }
    })
}

/// Range part of return is the range in the string literal that should get replaced with the char part of the literal
///
/// Errors if `i` is not the position of a `\` in `src`
fn escape_seq(src: &str, i: usize) -> Result<(Range<usize>, char), ErrorType<'_>> {
    let esc_rest = src
        .get(i..)
        .expect("`i` should not be within a UTF-8 character");
    escape_char(esc_rest)
        .ok_or(ErrorType::InvalidEscape(esc_rest)) // no remaining characters
        .and_then(|(len, res)| {
            let esc = esc_rest.get(..len).expect(
                "return of escape_char, starting at `i`, should not be within a UTF-8 character",
            );
            let range = Range::from(
                i..i.checked_add(len)
                    .expect("should be at most the length of a string already in memory"),
            );
            res.map(|ch| (range, ch))
                .map_err(|()| ErrorType::InvalidEscape(esc))
        })
}
