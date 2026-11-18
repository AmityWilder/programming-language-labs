use crate::lex::symbols::ESCAPE;
use std::range::Range;

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

            #[expect(
                clippy::arithmetic_side_effects,
                reason = "As shown above, `char::MAX_LEN_UTF8 * 2` fits in usize. \
                          By definition of `char::MAX_LEN_UTF8`, `c.len_utf8()` is at most `char::MAX_LEN_UTF8` for all `c: char`. \
                          Therefore, `c.len_utf8() + d.len_utf8()` fits in usize for all `c,d: char`.",
            )]
            let base_len = ESCAPE.len_utf8() + ch.len_utf8();
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
fn escape_seq(src: &str, i: usize) -> Result<(Range<usize>, char), &str> {
    let esc_rest = src
        .get(i..)
        .expect("`i` should not be within a UTF-8 character");
    let (len, res) = escape_char(esc_rest).ok_or(esc_rest)?;
    let esc = esc_rest
        .get(..len)
        .expect("return of escape_char, starting at `i`, should not be within a UTF-8 character");
    let range = Range::from(
        i..i.checked_add(len)
            .expect("should be at most the length of a string already in memory"),
    );
    res.map(|ch| (range, ch)).map_err(|()| esc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esc_esc() {
        const SOURCE: &str = r"\\";
        let esc = escape_char(SOURCE);
        assert_eq!(esc, Some((SOURCE.len(), Ok('\\'))));
    }
}
