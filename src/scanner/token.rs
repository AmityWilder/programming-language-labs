use super::{
    error::{ErrorType, NumLitError},
    symbols::*,
};
use std::{borrow::Cow, range::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// An entire chunk of whitespace, not just one character
    Whitespace,
    Comment,
    NumberLiteral,
    CharLiteral,
    StringLiteral,
    Identifier,
    /// Identical to [`Self::Identifier`], but implies a function by context
    /// i.e. The next token is an open parentheses (`(`)
    Callable,
    Keyword,
    /// Identical to [`Self::Keyword`], but specific to [`KeywordType::Control`]
    /// (because they have a different highlight color)
    CtrlKeyword,
    Macro,
    MacroParam,
    Punctuation,
    /// A subset of [`Self::Punctuation`] with depth
    Bracket(usize),
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

pub trait TokenValueSimplicity: Sized + 'static {
    type StringLiteral<'a>;

    fn has_escapes(literal: &Self::StringLiteral<'_>) -> bool;
}

/// String literals may contain unconverted escape sequences
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoAlloc(!);

impl TokenValueSimplicity for NoAlloc {
    type StringLiteral<'a> = &'a str;

    fn has_escapes(literal: &&str) -> bool {
        literal.contains(ESCAPE)
    }
}

/// String literals have escape sequences converted
#[derive(Debug, Clone, PartialEq)]
pub struct Allocated(!);

impl TokenValueSimplicity for Allocated {
    type StringLiteral<'a> = StringLiteral<'a>;

    fn has_escapes(literal: &Self::StringLiteral<'_>) -> bool {
        !literal.escapes.is_empty()
    }
}

/// The value represented by a [`Token`]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TokenValue<'a, S: TokenValueSimplicity = Allocated> {
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
    Bracket(usize),
    /// Escape sequences are converted (unless there are none)
    StringLiteral(S::StringLiteral<'a>),
}

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
}

#[derive(Debug, Clone)]
pub struct Escapes<'a> {
    source: &'a str,
    offset: usize,
    is_esc: bool,
}

impl<'a> Escapes<'a> {
    pub const fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            is_esc: false,
        }
    }
}

impl<'a> Iterator for Escapes<'a> {
    type Item = Result<(Range<usize>, char), ErrorType<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.source[self.offset..]
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

impl<'a> TokenValue<'a, Allocated> {
    fn string_literal(src: &'a str) -> Result<Self, ErrorType<'a>> {
        let replacements = Escapes::new(src).collect::<Result<Vec<_>, _>>()?;
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
            // SAFETY: 2 UTF8 characters are guaranteed not to exceed usize::MAX
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

impl<'a> Token<'a> {
    /// Obtains the value of a token without allocating
    ///
    /// **Warning:** String literals will be incorrect because of the "no alloc" rule.
    pub(super) fn value_noalloc(self) -> Result<TokenValue<'a, NoAlloc>, ErrorType<'a>> {
        const VALID_TOKENS: &str = "Token::value() expects vaild tokens";
        match self.ty {
            TokenType::Whitespace | TokenType::Comment => Ok(TokenValue::Ignore),
            TokenType::NumberLiteral => TokenValue::number_literal(self.src),
            TokenType::CharLiteral => TokenValue::char_literal(self.src),
            TokenType::StringLiteral => TokenValue::string_literal_noalloc(self.src),
            TokenType::Identifier
            | TokenType::Callable
            | TokenType::Macro
            | TokenType::MacroParam => Ok(TokenValue::Direct(self.src)),
            TokenType::Keyword | TokenType::CtrlKeyword => Ok(TokenValue::Keyword(
                Keyword::from_str(self.src).expect(VALID_TOKENS),
            )),
            TokenType::Punctuation => Ok(TokenValue::Punctuation(
                Punctuation::from_str(self.src).expect(VALID_TOKENS),
            )),
            TokenType::Bracket(depth) => Ok(TokenValue::Bracket(depth)),
        }
    }
}

impl<'a> TryFrom<TokenValue<'a, NoAlloc>> for TokenValue<'a, Allocated> {
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

            TokenValue::Ignore => Ok(Self::Ignore),
            TokenValue::UIntLiteral(x) => Ok(Self::UIntLiteral(x)),
            TokenValue::SIntLiteral(x) => Ok(Self::SIntLiteral(x)),
            TokenValue::FltLiteral(x) => Ok(Self::FltLiteral(x)),
            TokenValue::CharLiteral(x) => Ok(Self::CharLiteral(x)),
            TokenValue::Direct(x) => Ok(Self::Direct(x)),
            TokenValue::Keyword(x) => Ok(Self::Keyword(x)),
            TokenValue::Punctuation(x) => Ok(Self::Punctuation(x)),
            TokenValue::Bracket(x) => Ok(Self::Bracket(x)),
        }
    }
}
