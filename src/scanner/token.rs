use super::{
    Scanner,
    error::{ErrorType, NumLitError},
    symbols::*,
    unbalanced,
};
use std::{borrow::Cow, iter::Peekable, range::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// An entire chunk of whitespace, not just one character
    Whitespace,
    Comment,
    NumberLiteral,
    CharLiteral,
    StringLiteral,
    /// A string that can contain expressions
    InterpolatedString,
    Identifier,
    /// Identical to [`Self::Identifier`], but implies a function by context
    /// i.e. The next token is an open parentheses (`(`)
    Callable,
    Keyword,
    /// Identical to [`Self::Keyword`], but specific to [`KeywordType::Control`]
    /// (because they have a different highlight color)
    CtrlKeyword,
    Punctuation,
}

/// Helper macro for preventing issues with missed variants when adding new ones
///
/// Variants should be in the order they should be tested
macro_rules! define_token_eq {
    (
        $(#[$em:meta])*
        $vis:vis enum $Enum:ident {$(
            $(#[$vm:meta])*
            $Variant:ident = $value:literal
        ),+ $(,)?}
    ) => {
        $(#[$em])*
        $vis enum $Enum {$(
            $(#[$vm])*
            #[doc = concat!("`", $value, "`")]
            $Variant,
        )+}

        impl $Enum {
            /// Descending length, so bigger tokens aren't broken apart by subset tokens
            pub const OPTIONS: [(&str, Self); [$(Self::$Variant),+].len()] = [
                $(($value, Self::$Variant),)+
            ];

            /// Matches the prefix of `s` to a [`Self`]. Tries to find the longest one possible.
            #[allow(dead_code)]
            pub fn from_prefix(s: &str) -> Option<Self> {
                Self::OPTIONS
                    .into_iter()
                    .find(|(pat, _)| s.starts_with(pat))
                    .map(|(_, punc)| punc)
            }

            /// Like [`Self::from_prefix`] but matches the full string
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($value => Some(Self::$Variant),)+
                    _ => None,
                }
            }

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $value,)+
                }
            }
        }

        impl std::fmt::Display for $Enum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordType {
    Definition,
    Value,
    Control,
}

define_token_eq! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Keyword {
        // Definitions
        Struct = "struct",
        Union = "union",
        Enum = "enum",
        Type = "type",

        // halfway between Definition and Value
        Def = "def",
        Fn = "fn",

        // Value
        Let = "let",
        Const = "const",
        Static = "static",

        // Flow control
        If = "if",
        Else = "else",
        For = "for",
        While = "while",
        With = "with",
        Where = "where",
        Loop = "loop",
        In = "in",
    }
}

impl Keyword {
    pub const fn kw_type(self) -> KeywordType {
        match self {
            Self::Struct | Self::Union | Self::Enum | Self::Type => KeywordType::Definition,

            #[allow(
                clippy::match_same_arms,
                reason = "gray area - isolated for future decision"
            )]
            Self::Def | Self::Fn => KeywordType::Definition,

            Self::Let | Self::Const | Self::Static => KeywordType::Value,

            Self::If
            | Self::Else
            | Self::For
            | Self::While
            | Self::With
            | Self::Where
            | Self::Loop
            | Self::In => KeywordType::Control,
        }
    }
}

define_token_eq! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Punctuation {
        // 3-char
        ShlAssign = "<<=",
        ShrAssign = ">>=",

        // 2-char
        Neq = "!=",
        RemAssign = "%=",
        AndAssign = "&=",
        MulAssign = "*=",
        Exponent = "**",
        AddAssign = "+=",
        SubAssign = "-=",
        Arrow = "->",
        DivAssign = "/=",
        PathSep = "::",
        Le = "<=",
        Shl = "<<",
        Eq = "==",
        FatArrow = "=>",
        Ge = ">=",
        Shr = ">>",
        XorAssign = "^=",
        OrAssign = "|=",

        // 1-char
        Not = "!",
        MacroArgCount = "#",
        Ref = "$",
        Remainder = "%",
        And = "&",
        LParen = "(",
        RParen = ")",
        Mul = "*",
        Add = "+",
        Comma = ",",
        Sub = "-",
        Dot = ".",
        Div = "/",
        Colon = ":",
        Semi = ";",
        Lt = "<",
        Assign = "=",
        Gt = ">",
        QMark = "?",
        LBrack = "[",
        MacroStart = "\\",
        RBrack = "]",
        Xor = "^",
        LBrace = "{",
        Or = "|",
        RBrace = "}",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CharLiteral {
    pub ch: char,
    pub is_escaped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct StringLiteral<'a> {
    /// The text content of the string literal; escape sequences converted, "`${}`"s removed, and delimiters excluded.
    ///
    /// Like a string literal, it's possible no escape sequences or "`${}`"s were present,
    /// in which case this will be borrowed and [`Self::expressions`] will be empty.
    pub text: Cow<'a, str>,

    /// Ranges of the original lexeme (quote delimiters excluded) that refer to escape sequences
    pub escapes: Vec<Range<usize>>,
}

impl<'a> StringLiteral<'a> {
    pub const fn borrowed(text: &'a str) -> Self {
        Self {
            text: Cow::Borrowed(text),
            escapes: Vec::new(),
        }
    }
}

/// `T`: The collection that lists expression sub-tokens.
/// It is typically one of the following:
/// - [`Scanner`]
/// - [`Vec`] (or similar) of [`TokenResult`]
/// - [`!`](https://doc.rust-lang.org/std/primitive.never.html) (because interpolated strings can't be nested)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct InterpolatedExpr<T> {
    /// The range of the entire `${...}` segment within the original lexeme (quote delimiters excluded) containing this expression
    ///
    /// The `${` and `}` delimiters are **included** in this range
    pub range: Range<usize>,

    /// Position in the processed text where the expression result should be inserted
    pub position: usize,

    /// Token stream of the expression WITHIN the original lexeme
    /// (you will need to supply the offset of that lexeme yourself)
    pub expr: T,
}

/// `T`: The collection that lists [`InterpolatedExpr`] sub-tokens
#[derive(Debug, Clone)]
pub struct ReplacementIter<'a, T> {
    escapes: Peekable<std::slice::Iter<'a, Range<usize>>>,
    exprs: Peekable<std::slice::Iter<'a, InterpolatedExpr<T>>>,
}

impl<'a, T> ReplacementIter<'a, T> {
    fn new(escapes: &'a [Range<usize>], exprs: &'a [InterpolatedExpr<T>]) -> Self {
        Self {
            escapes: escapes.iter().peekable(),
            exprs: exprs.iter().peekable(),
        }
    }
}

impl<'a, T> Iterator for ReplacementIter<'a, T> {
    type Item = (Range<usize>, Option<&'a T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.escapes
            .next_if(|range| {
                self.exprs
                    .peek()
                    .is_none_or(|item| range.start < item.range.start)
            })
            .map(|&range| (range, None))
            .or_else(|| self.exprs.next().map(|item| (item.range, Some(&item.expr))))
    }
}

/// `T`: The collection that lists [`InterpolatedExpr`] sub-tokens
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct InterpolatedString<'a, T> {
    /// The text content of the string literal; escape sequences converted, "`${}`"s removed, and delimiters excluded.
    ///
    /// Like a string literal, it's possible no escape sequences or "`${}`"s were present,
    /// in which case this will be borrowed and [`Self::expressions`] will be empty.
    pub text: StringLiteral<'a>,

    /// The positions and tokens of the expressions to insert into [`Self::text`]
    ///
    /// Note: It is impossible to have an interpolated string *within* an interpolated string expression,
    /// as the opening delimiter to the inner literal would be identical to the closing delimiter of the outer literal.
    /// Instead of getting a nested interpolated string, you would get an [`ErrorType::EndlessInterpStrExpr`] error.
    pub expressions: Vec<InterpolatedExpr<T>>,
}

impl<'a, T> InterpolatedString<'a, T> {
    pub const fn borrowed(text: &'a str) -> Self {
        Self {
            text: StringLiteral::borrowed(text),
            expressions: Vec::new(),
        }
    }

    pub fn replacements(&self) -> ReplacementIter<'_, T> {
        ReplacementIter::new(&self.text.escapes, &self.expressions)
    }
}

/// Escape sequences in an interpolated string
fn interpolated_escapes() -> impl FnMut(char) -> bool {
    // [`None`] if in the outer literal
    let mut inner_literal_delim = None;
    let mut is_esc = false;
    move |ch: char| {
        // we don't want to include escapes that belong to nested literals.
        // those belong to those literals, not this one.
        if !is_esc {
            if let Some(delim) = inner_literal_delim {
                if ch == delim {
                    inner_literal_delim = None;
                }
            } else if matches!(ch, CHAR_DELIM | STR_DELIM) {
                inner_literal_delim = Some(ch);
            }
        }
        is_esc = !is_esc && ch == ESCAPE;

        inner_literal_delim.is_none() && is_esc
    }
}

pub trait TokenValueSimplicity {
    type StringLiteral<'a>;
    type InterpolatedString<'a>;
}

/// String literals may contain unconverted escape sequences
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NoAlloc(());

impl TokenValueSimplicity for NoAlloc {
    type StringLiteral<'a> = &'a str;
    type InterpolatedString<'a> = &'a str;
}

/// String literals have escape sequences converted
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Allocated<T>(std::marker::PhantomData<T>);

impl<T> TokenValueSimplicity for Allocated<T> {
    type StringLiteral<'a> = StringLiteral<'a>;
    type InterpolatedString<'a> = InterpolatedString<'a, T>;
}

/// String literals have escape sequences converted
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AllocNested(());

impl TokenValueSimplicity for AllocNested {
    type StringLiteral<'a> = StringLiteral<'a>;
    type InterpolatedString<'a> = !;
}

/// The value represented by a [`Token`]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TokenValue<'a, S: TokenValueSimplicity = Allocated<Scanner<'a>>> {
    /// Whitespace/comments
    #[default]
    Ignore,
    UIntLiteral(usize),
    SIntLiteral(isize),
    FltLiteral(f64),
    CharLiteral(CharLiteral),
    /// Value is the token source itself
    Direct(&'a str),
    Keyword(Keyword),
    Punctuation(Punctuation),
    /// Escape sequences are converted (unless there are none)
    StringLiteral(S::StringLiteral<'a>),
    InterpolatedString(S::InterpolatedString<'a>),
}

pub type AllocTokenValue<'a, T> = TokenValue<'a, Allocated<T>>;
pub type NoAllocTokenValue<'a> = TokenValue<'a, NoAlloc>;
pub type NestedTokenValue<'a> = TokenValue<'a, AllocNested>;

impl<'a, S: TokenValueSimplicity> TokenValue<'a, S> {
    fn number_literal(src: &'a str) -> Result<Self, ErrorType<'a>> {
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
                                    ErrorType::InvalidNumLiteral(NumLitError::SInt({
                                        // SAFETY: i8::MIN-1 fits in i16. I tried asserting to prove this,
                                        // but got an `invalid_upcast_comparisons` warning, which prove it by itself.
                                        i8::try_from(unsafe { i16::from(i8::MIN).unchecked_sub(1) })
                                            .expect_err("should result in negative overflow")
                                    }))
                                })
                            }))
                        .map(|x| Self::SIntLiteral(x))
                    } else {
                        Ok(Self::UIntLiteral(value))
                    }
                })
        }
    }

    fn char_literal(src: &'a str) -> Result<Self, ErrorType<'a>> {
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
}

impl<'a> TokenValue<'a, NoAlloc> {
    fn string_literal_noalloc(src: &'a str) -> Result<Self, ErrorType<'a>> {
        let src = src
            .strip_circumfix(STR_DELIM, STR_DELIM)
            .expect("string literal tokens should include delimiters (`\"`)");
        let mut is_esc = false;
        if src.contains(ESCAPE)
            && let Some(e) = src
                .match_indices(|ch: char| {
                    is_esc = !is_esc && ch == ESCAPE;
                    is_esc
                })
                .find_map(|(i, _)| escape_seq(src, i).err())
        {
            Err(e)
        } else {
            Ok(Self::StringLiteral(src))
        }
    }

    fn interpolated_string_noalloc(src: &'a str) -> Result<Self, ErrorType<'a>> {
        let src = src
            .strip_circumfix(INTERP_STR_DELIM, INTERP_STR_DELIM)
            .expect("interpolated string literal tokens should include delimiters (`` ` ``)");
        if let Some(e) = src
            .match_indices(interpolated_escapes())
            .find_map(|(i, _)| escape_seq(src, i).err())
            .or_else(|| {
                src.match_indices(INTERP_EXPR_OPEN)
                    .find_map(|(i, _)| interp_str_expr(src, i).err())
            })
        {
            Err(e)
        } else {
            Ok(Self::InterpolatedString(src))
        }
    }
}

impl<'a> TokenValue<'a, Allocated<Scanner<'a>>> {
    fn string_literal(src: &'a str) -> Result<Self, ErrorType<'a>> {
        let mut is_esc = false;
        let replacements = src
            .match_indices(|ch: char| {
                is_esc = !is_esc && ch == ESCAPE;
                is_esc
            })
            .map(|(i, _)| escape_seq(src, i))
            .collect::<Result<Vec<_>, _>>()?;
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
            src.len()
                .checked_sub(byte_diff)
                .expect("should only be removing bytes, not adding"),
        );
        let mut prev_end = 0;
        for (range, repl) in replacements {
            processed.push_str(&src[prev_end..range.start]);
            processed.push(repl);
            prev_end = range.end;
        }

        Ok(Self::StringLiteral(StringLiteral {
            text: Cow::Owned(processed),
            escapes,
        }))
    }

    fn interpolated_string(src: &'a str) -> Result<Self, ErrorType<'a>> {
        let esc_replacements = src
            .match_indices(interpolated_escapes())
            .map(|(i, _)| escape_seq(src, i))
            .collect::<Result<Vec<_>, _>>()?;
        let escapes = esc_replacements.iter().map(|(range, _)| *range).collect();
        let (expr_replacements, expr_scanners) = src
            .match_indices(INTERP_EXPR_OPEN)
            .map(|(i, _)| interp_str_expr(src, i))
            .collect::<Result<(Vec<_>, Vec<_>), _>>()?;
        let mut replacements = Vec::with_capacity(
            esc_replacements
                .len()
                .checked_add(expr_replacements.len())
                .expect("sum of escapes and expressions should not exceed the number of characters in a string, \
                         since they cannot occupy the same space. the number of characters in the string should\
                         not exceed usize::MAX."),
        );
        // everything is in order, but expressions and escapes can be interspersed
        {
            let mut esc_iter = esc_replacements
                .into_iter()
                .map(|(range, ch)| (range, Some(ch)))
                .peekable();

            let mut expr_iter = expr_replacements
                .into_iter()
                .map(|range| (range, None))
                .peekable();

            replacements.extend(std::iter::from_fn(|| {
                esc_iter
                    .next_if(|(esc_range, _)| {
                        expr_iter
                            .peek()
                            .is_none_or(|(expr_range, _)| esc_range.start < expr_range.start)
                    })
                    .or_else(|| expr_iter.next())
            }));
        }

        // how many bytes of difference between the original string and the processed string
        let mut byte_diff = 0;
        // need to get the correct positions of the expression insertions, since their positions change when we replace substrings
        let expressions = replacements
            .iter()
            .copied()
            .filter_map(|(range, repl)| {
                let start = range.start.checked_sub(byte_diff).unwrap_or_else(|| {
                    panic!("shouldn't move tokens backwards\n replacements: {replacements:?}")
                });
                let being_replaced_len = range
                    .end
                    .checked_sub(range.start)
                    .expect("ranges should be ascending");
                let replace_with_len = repl.map_or(0, char::len_utf8);
                byte_diff = byte_diff.strict_add(
                    being_replaced_len
                        .checked_sub(replace_with_len)
                        .expect("replacements should be smaller than the text being replaced"),
                );
                // expression replacements are always None, so if we see a None we know that's an expression.
                repl.is_none().then_some((range, start))
            })
            .zip(expr_scanners)
            .map(|((range, position), expr)| InterpolatedExpr {
                range,
                position,
                expr,
            })
            .collect();

        let mut processed = String::with_capacity(
            src.len()
                .checked_sub(byte_diff)
                .expect("should not remove more bytes than exist in the original string"),
        );
        let mut prev_end = 0;
        for (range, repl) in replacements {
            processed.push_str(&src[prev_end..range.start]);
            prev_end = range.end;
            if let Some(ch) = repl {
                processed.push(ch);
            }
        }

        Ok(Self::InterpolatedString(InterpolatedString {
            text: StringLiteral {
                text: Cow::Owned(processed),
                escapes,
            },
            expressions,
        }))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token<'a> {
    /// Because this is a pointer into the original source string, we can use pointer arithmetic to find its location.
    /// If a program has a thousand tokens, why allocate a new string and store two additional integers in case of error
    /// when we can just keep the original string around and calculate those integers *on demand*?
    pub src: &'a str,

    /// Couldn't be named `type` because that's a keyword in Rust
    pub ty: TokenType,
}

impl std::fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { src, ty } = self;
        write!(f, "{ty:?}({src:?})")
    }
}

/// Returns [`None`] if `src` does not start with `\`
fn escape_char(src: &str) -> Option<(usize, Result<char, ()>)> {
    let mut iter = src.chars();
    iter.next().filter(|ch| *ch == ESCAPE).map(|_| {
        let res = iter.next().ok_or(ESCAPE.len_utf8()).and_then(|ch| {
            const {
                assert!(
                    char::MAX_LEN_UTF8.checked_mul(2).is_some(),
                    "proof. 2 UTF8 characters are guaranteed not to exceed usize::MAX"
                );
            }
            // SAFETY: 2 UTF8 characters are guaranteed not to exceed usize::MAX
            let base_len = unsafe { ESCAPE.len_utf8().unchecked_add(ch.len_utf8()) };
            match ch {
                '\\' | '"' | '\'' | '`' => Ok((base_len, ch)),

                'a' => Ok((base_len, '\x07')),
                'b' => Ok((base_len, '\x08')),
                't' => Ok((base_len, '\t')),
                'n' => Ok((base_len, '\n')),
                'v' => Ok((base_len, '\x0b')),
                'f' => Ok((base_len, '\x0c')),
                'r' => Ok((base_len, '\r')),
                'e' => Ok((base_len, '\x1b')),

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
    escape_char(&src[i..])
        .ok_or(ErrorType::InvalidEscape(&src[i..])) // no remaining characters
        .and_then(|(len, res)| {
            let range = Range::from(
                i..i.checked_add(len)
                    .expect("should be at most the length of a string already in memory"),
            );
            res.map(|ch| (range, ch))
                .map_err(|()| ErrorType::InvalidEscape(&src[range]))
        })
}

/// Range part of return is the range in the string literal that should get eliminated due to being replaced with a runtime expression
///
/// # Panics
/// This function will panic if `i` is not the position of a `${` in `src`
fn interp_str_expr(src: &str, i: usize) -> Result<(Range<usize>, Scanner<'_>), ErrorType<'_>> {
    let expr = src[i..]
        .strip_prefix(INTERP_EXPR_OPEN)
        .expect("`i` should be the position of a `${` in `src`");
    expr.find(unbalanced('{', '}'))
        .map(|len| {
            const DELIMS_LEN: usize = INTERP_EXPR_OPEN.len() + INTERP_EXPR_CLOSE.len_utf8();
            let start_rm = i;
            let end_rm = start_rm
                .checked_add(len)
                .and_then(|n| n.checked_add(DELIMS_LEN))
                .expect("should be a subset of an existing string whose len must fit in usize");
            (Range::from(start_rm..end_rm), Scanner::new(&expr[..len]))
        })
        .ok_or(ErrorType::EndlessInterpStrExpr)
}

impl<'a> Token<'a> {
    #[cfg(test)]
    pub const fn new(src: &'a str, ty: TokenType) -> Self {
        Self { src, ty }
    }

    /// Obtains the value of a token without allocating
    ///
    /// **Warning:** String literals will be incorrect because of the "no alloc" rule.
    pub(super) fn value_noalloc(self) -> Result<NoAllocTokenValue<'a>, ErrorType<'a>> {
        const VALID_TOKENS: &str = "Token::value() expects vaild tokens";
        match self.ty {
            TokenType::Whitespace | TokenType::Comment => Ok(TokenValue::Ignore),
            TokenType::NumberLiteral => TokenValue::number_literal(self.src),
            TokenType::CharLiteral => TokenValue::char_literal(self.src),
            TokenType::StringLiteral => TokenValue::string_literal_noalloc(self.src),
            TokenType::InterpolatedString => TokenValue::interpolated_string_noalloc(self.src),
            TokenType::Identifier | TokenType::Callable => Ok(TokenValue::Direct(self.src)),
            TokenType::Keyword | TokenType::CtrlKeyword => Ok(TokenValue::Keyword(
                Keyword::from_str(self.src).expect(VALID_TOKENS),
            )),
            TokenType::Punctuation => Ok(TokenValue::Punctuation(
                Punctuation::from_str(self.src).expect(VALID_TOKENS),
            )),
        }
    }
}

impl<'a> TryFrom<TokenValue<'a, NoAlloc>> for TokenValue<'a, Allocated<Scanner<'a>>> {
    type Error = ErrorType<'a>;

    fn try_from(value: TokenValue<'a, NoAlloc>) -> Result<Self, Self::Error> {
        match value {
            // string literal
            TokenValue::StringLiteral(src) => {
                if src.contains(ESCAPE) {
                    TokenValue::string_literal(src)
                } else {
                    Ok(Self::StringLiteral(StringLiteral::borrowed(src)))
                }
            }

            // interpolated string literal
            TokenValue::InterpolatedString(src) => {
                if src.contains(ESCAPE) || src.contains(INTERP_EXPR_OPEN) {
                    TokenValue::interpolated_string(src)
                } else {
                    Ok(Self::InterpolatedString(InterpolatedString::borrowed(src)))
                }
            }

            TokenValue::Ignore => Ok(Self::Ignore),
            TokenValue::UIntLiteral(x) => Ok(Self::UIntLiteral(x)),
            TokenValue::SIntLiteral(x) => Ok(Self::SIntLiteral(x)),
            TokenValue::FltLiteral(x) => Ok(Self::FltLiteral(x)),
            TokenValue::CharLiteral(x) => Ok(Self::CharLiteral(x)),
            TokenValue::Direct(x) => Ok(Self::Direct(x)),
            TokenValue::Keyword(x) => Ok(Self::Keyword(x)),
            TokenValue::Punctuation(x) => Ok(Self::Punctuation(x)),
        }
    }
}

impl<'a, T> From<TokenValue<'a, Allocated<T>>> for TokenValue<'a, AllocNested> {
    fn from(value: TokenValue<'a, Allocated<T>>) -> Self {
        match value {
            TokenValue::InterpolatedString(..) => {
                unreachable!(
                    "nested interpolated strings should not be possible; the delimiter does not distinguish open from close"
                )
            }

            TokenValue::Ignore => Self::Ignore,
            TokenValue::UIntLiteral(x) => Self::UIntLiteral(x),
            TokenValue::SIntLiteral(x) => Self::SIntLiteral(x),
            TokenValue::FltLiteral(x) => Self::FltLiteral(x),
            TokenValue::CharLiteral(x) => Self::CharLiteral(x),
            TokenValue::StringLiteral(x) => Self::StringLiteral(x),
            TokenValue::Direct(x) => Self::Direct(x),
            TokenValue::Keyword(x) => Self::Keyword(x),
            TokenValue::Punctuation(x) => Self::Punctuation(x),
        }
    }
}
