use std::{borrow::Cow, iter::Peekable, range::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumLitError {
    UInt(std::num::ParseIntError),
    SInt(std::num::TryFromIntError),
    Flt(std::num::ParseFloatError),
}

impl std::fmt::Display for NumLitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UInt(e) => e.fmt(f),
            Self::SInt(e) => e.fmt(f),
            Self::Flt(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for NumLitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UInt(e) => Some(e),
            Self::SInt(e) => Some(e),
            Self::Flt(e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType<'a> {
    UnknownToken,
    EndlessBlockComment,
    EmptyCharLiteral,
    MultiCharLiteral,
    EndlessStringLiteral,
    EscapedStringLiteralEnd,
    EndlessInterpStrExpr,
    InvalidEscape(&'a str),
    InvalidNumLiteral(NumLitError),
}

impl std::fmt::Display for ErrorType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownToken => f.write_str("unknown token"),
            Self::EndlessBlockComment => {
                f.write_str("block comment opens (`/*`) but never closes (missing `*/`)")
            }
            Self::EmptyCharLiteral => f.write_str("empty character literal"),
            Self::MultiCharLiteral => f.write_str(
                "character literal may only contain one codepoint; \
                if you meant to write a string literal, use double quotes (`\"`). \
                if you meant to write an interpolated string, use graves (`` ` ``).",
            ),
            Self::EndlessStringLiteral => {
                f.write_str("string literal opens (`\"`) but never closes (missing unescaped `\"`)")
            }
            Self::EscapedStringLiteralEnd => f.write_str(
                "string literal opens (`\"`) but never closes (missing unescaped `\"`). \
                there is a closing double-quote candidate, but it is escaped (`\\\"`). \
                string literals cannot end with an unescaped backslash (`\\`), \
                it is indistinguishable from an escaped double-quote (`\\\"`)",
            ),
            Self::EndlessInterpStrExpr => f.write_str(
                "interpolated string expression opened (`${`) but never closes (missing `}`)",
            ),
            Self::InvalidEscape(s) => write!(f, "unknown character escape: {s}"),
            Self::InvalidNumLiteral(e) => write!(f, "invalid number literal: {e}"),
        }
    }
}

impl std::error::Error for ErrorType<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidNumLiteral(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error<'a> {
    pub range: Range<usize>,
    pub err: ErrorType<'a>,
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            range: Range { start, end },
            err,
        } = self;
        write!(f, "at {start}..{end}: {err}")
    }
}

impl std::error::Error for Error<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}

impl<'a> Error<'a> {
    pub const fn add_context(self, source: &'a str) -> ContextError<'a> {
        ContextError {
            source,
            range: self.range,
            err: self.err,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextError<'a> {
    pub source: &'a str,
    pub range: Range<usize>,
    pub err: ErrorType<'a>,
}

fn line_col(s: &str, position: usize) -> (usize, usize) {
    s[..position]
        .lines()
        .enumerate()
        .last()
        .map_or((0, 0), |(row, line)| {
            (/* 1-based index */ row + 1, line.len())
        })
}

impl std::fmt::Display for ContextError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            source,
            range,
            ref err,
        } = *self;
        let (start_line, start_col) = line_col(source, range.start);
        let (end_line, end_col) = line_col(source, range.end);
        write!(f, "at {start_line}:{start_col}-{end_line}:{end_col}: {err}")
    }
}

impl std::error::Error for ContextError<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharLiteral {
    pub ch: char,
    pub is_escaped: bool,
}

#[derive(Debug, Clone)]
pub struct RemappedEscapes<I> {
    open_delim_len: usize,
    iter: I,
}

impl<I> RemappedEscapes<I> {
    fn new(open_delim_len: usize, iter: I) -> Self {
        Self {
            open_delim_len,
            iter,
        }
    }
}

impl<I> Iterator for RemappedEscapes<I>
where
    I: Iterator<Item = Range<usize>>,
{
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|range| {
            Range::from((range.start + self.open_delim_len)..(range.end + self.open_delim_len))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Remap [`Self::escapes`] to **include** offsets from the quote delimiters
    pub fn remapped_escapes<'b>(
        &'b self,
        open_delim: &str,
    ) -> RemappedEscapes<std::iter::Copied<std::slice::Iter<'b, Range<usize>>>> {
        RemappedEscapes::new(open_delim.len(), self.escapes.iter().copied())
    }
}

/// `T`: The collection that lists expression sub-tokens.
/// It is typically one of the following:
/// - [`Scanner`]
/// - [`Vec`] (or similar) of [`TokenResult`]
/// - [`!`](https://doc.rust-lang.org/std/primitive.never.html) (because interpolated strings can't be nested)
#[derive(Debug, Clone, PartialEq, Eq)]
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
pub struct Replacements<'a, T> {
    escapes: Peekable<std::slice::Iter<'a, Range<usize>>>,
    exprs: Peekable<std::slice::Iter<'a, InterpolatedExpr<T>>>,
}

impl<'a, T> Replacements<'a, T> {
    fn new(escapes: &'a [Range<usize>], exprs: &'a [InterpolatedExpr<T>]) -> Self {
        Self {
            escapes: escapes.iter().peekable(),
            exprs: exprs.iter().peekable(),
        }
    }
}

impl<'a, T> Iterator for Replacements<'a, T> {
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn replacements(&self) -> Replacements<'_, T> {
        Replacements::new(&self.text.escapes, &self.expressions)
    }
}

fn interpolated_escapes() -> impl FnMut(char) -> bool {
    let mut within_inner_literal = None;
    let mut is_esc = false;
    move |ch: char| {
        // we don't want to include escapes that belong to nested literals.
        // those belong to those literals, not this one.
        let is_inner_delim;
        if let Some(delim) = within_inner_literal {
            is_inner_delim = !is_esc && ch == delim;
            if is_inner_delim {
                within_inner_literal = None;
            }
        } else {
            is_inner_delim = !is_esc && matches!(ch, '\'' | '"');
            if is_inner_delim {
                within_inner_literal = Some(ch);
            }
        }
        if within_inner_literal.is_some() && !is_inner_delim {
            print!("{ch}");
        }
        if is_inner_delim {
            if within_inner_literal.is_none() {
                println!();
            }
            println!(
                "{} within inner literal",
                if within_inner_literal.is_some() {
                    "now"
                } else {
                    "no longer"
                }
            );
        }
        is_esc = !is_esc && ch == '\\';
        is_esc && within_inner_literal.is_none()
    }
}

/// The value represented by a [`Token`]
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue<'a, T> {
    UIntLiteral(usize),
    SIntLiteral(isize),
    FltLiteral(f64),
    CharLiteral(CharLiteral),
    /// Escape sequences are converted (unless there are none)
    StringLiteral(StringLiteral<'a>),
    InterpolatedString(InterpolatedString<'a, T>),
    /// Value is the token source itself
    Direct(&'a str),
    Keyword(Keyword),
    Punctuation(Punctuation),
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
    const ESCAPE: char = '\\';
    let mut iter = src.chars();
    iter.next().filter(|ch| *ch == ESCAPE).map(|_| {
        let res = iter.next().ok_or(ESCAPE.len_utf8()).and_then(|ch| {
            let base_len = ESCAPE.len_utf8() + ch.len_utf8();
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
                    let end = num_start + digits; // ASCII digits
                    let len = base_len + digits;
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
            let range = Range::from(i..i + len);
            res.map(|ch| (range, ch))
                .map_err(|()| ErrorType::InvalidEscape(&src[range]))
        })
}

/// Range part of return is the range in the string literal that should get eliminated due to being replaced with a runtime expression
///
/// # Panics
/// This function will panic if `i` is not the position of a `${` in `src`
fn interp_str_expr(src: &str, i: usize) -> Result<(Range<usize>, Scanner<'_>), ErrorType<'_>> {
    const OPEN: &str = "${";
    let expr = src[i..]
        .strip_prefix(OPEN)
        .expect("`i` should be the position of a `${` in `src`");
    let mut depth = 0;
    expr.find(|ch: char| {
        match ch {
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            _ => (),
        }
        false
    })
    .map(|len| {
        let start_rm = i;
        let end_rm = start_rm + OPEN.len() + len + '}'.len_utf8();
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
    fn value_noalloc(self) -> Result<Option<TokenValue<'a, Scanner<'a>>>, ErrorType<'a>> {
        const VALID_TOKENS: &str = "Token::value() expects vaild tokens";
        match self.ty {
            TokenType::Whitespace | TokenType::Comment => Ok(None),

            TokenType::NumberLiteral => {
                const HEX_PREFIX: &str = "0x";
                const OCT_PREFIX: &str = "0o";
                const BIN_PREFIX: &str = "0b";

                // checking the start of a string is easier than looking through every one of its characters, so it goes first.
                // hexadecimal is the only case in which an 'e' might appear while NOT being a float.
                if !self.src.starts_with(HEX_PREFIX) && self.src.contains(['e', 'E'])
                    || self.src.contains('.')
                {
                    self.src
                        .parse() // turns out parse already handles the "e" syntax on its own
                        .map(|x| Some(TokenValue::FltLiteral(x)))
                        .map_err(|e| ErrorType::InvalidNumLiteral(NumLitError::Flt(e)))
                } else {
                    let stripped = self.src.strip_prefix('-');
                    let is_negative = stripped.is_some();
                    let magnitude = stripped.unwrap_or(self.src);

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
                                            ErrorType::InvalidNumLiteral(NumLitError::SInt(
                                                i8::try_from(i16::from(i8::MIN) - 1).expect_err(
                                                    "should result in negative overflow",
                                                ),
                                            ))
                                        })
                                    }))
                                .map(TokenValue::SIntLiteral)
                            } else {
                                Ok(TokenValue::UIntLiteral(value))
                            }
                        })
                        .map(Some)
                }
            }

            TokenType::CharLiteral => {
                const DELIM: char = '\'';
                let src = self
                    .src
                    .strip_prefix(DELIM)
                    .and_then(|s| s.strip_suffix(DELIM))
                    .expect("character literal tokens should include delimiters (`'`)");
                if let Some((len, res)) = escape_char(src) {
                    // escape sequence
                    res.map_err(|()| ErrorType::InvalidEscape(src))
                        .and_then(|ch| {
                            (len == src.len())
                                .then_some(Some(TokenValue::CharLiteral(CharLiteral {
                                    ch,
                                    is_escaped: true,
                                })))
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
                                .then_some(Some(TokenValue::CharLiteral(CharLiteral {
                                    ch,
                                    is_escaped: false,
                                })))
                                .ok_or(ErrorType::MultiCharLiteral)
                        })
                }
            }

            TokenType::StringLiteral => {
                const DELIM: char = '"';
                const ESCAPE: char = '\\';
                let src = self
                    .src
                    .strip_prefix(DELIM)
                    .and_then(|s| s.strip_suffix(DELIM))
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
                    Ok(Some(TokenValue::StringLiteral(StringLiteral::borrowed(
                        src,
                    ))))
                }
            }

            TokenType::InterpolatedString => {
                const DELIM: char = '`';
                let src = self
                    .src
                    .strip_prefix(DELIM)
                    .and_then(|s| s.strip_suffix(DELIM))
                    .expect(
                        "interpolated string literal tokens should include delimiters (`` ` ``)",
                    );
                if let Some(e) = src
                    .match_indices(interpolated_escapes())
                    .find_map(|(i, _)| escape_seq(src, i).err())
                    .or_else(|| {
                        src.match_indices("${")
                            .find_map(|(i, _)| interp_str_expr(src, i).err())
                    })
                {
                    Err(e)
                } else {
                    Ok(Some(TokenValue::InterpolatedString(
                        InterpolatedString::borrowed(src),
                    )))
                }
            }

            TokenType::Identifier | TokenType::Callable => Ok(Some(TokenValue::Direct(self.src))),

            TokenType::Keyword | TokenType::CtrlKeyword => Ok(Some(TokenValue::Keyword(
                Keyword::from_str(self.src).expect(VALID_TOKENS),
            ))),

            TokenType::Punctuation => Ok(Some(TokenValue::Punctuation(
                Punctuation::from_str(self.src).expect(VALID_TOKENS),
            ))),
        }
    }

    /// Returns [`None`] if non-code (whitespace/comment)
    pub fn value(self) -> Result<Option<TokenValue<'a, Scanner<'a>>>, ErrorType<'a>> {
        const EXPR_START: &str = "${";
        const ESC_START: char = '\\';
        let res = self.value_noalloc();
        match res {
            // string literal
            Ok(Some(TokenValue::StringLiteral(StringLiteral {
                text: Cow::Borrowed(src),
                mut escapes,
            }))) if src.contains(ESC_START) => {
                debug_assert_eq!(&escapes, &[], "should have no escapes if text is borrowed");
                let mut is_esc = false;
                let replacements = src
                    .match_indices(|ch: char| {
                        is_esc = !is_esc && ch == ESC_START;
                        is_esc
                    })
                    .map(|(i, _)| escape_seq(src, i))
                    .collect::<Result<Vec<_>, _>>()?;
                escapes.extend(replacements.iter().map(|(range, _)| range));

                let byte_diff: usize = replacements
                    .iter()
                    .map(|(range, ch)| (range.end - range.start) - ch.len_utf8())
                    .sum();
                let mut processed = String::with_capacity(src.len() - byte_diff);
                let mut prev_end = 0;
                for (range, repl) in replacements {
                    processed.push_str(&src[prev_end..range.start]);
                    processed.push(repl);
                    prev_end = range.end;
                }

                Ok(Some(TokenValue::StringLiteral(StringLiteral {
                    text: Cow::Owned(processed),
                    escapes,
                })))
            }

            // interpolated string literal
            Ok(Some(TokenValue::InterpolatedString(InterpolatedString {
                text:
                    StringLiteral {
                        text: Cow::Borrowed(src),
                        mut escapes,
                    },
                mut expressions,
            }))) if src.contains(ESC_START) || src.contains(EXPR_START) => {
                debug_assert_eq!(&escapes, &[], "should have no escapes if text is borrowed");
                debug_assert_eq!(
                    &expressions,
                    &[],
                    "should have no expressions if text is borrowed"
                );
                let esc_replacements = src
                    .match_indices(interpolated_escapes())
                    .map(|(i, _)| escape_seq(src, i))
                    .collect::<Result<Vec<_>, _>>()?;
                escapes.extend(esc_replacements.iter().map(|(range, _)| range));
                let (expr_replacements, expr_scanners) = src
                    .match_indices(EXPR_START)
                    .map(|(i, _)| interp_str_expr(src, i))
                    .collect::<Result<(Vec<_>, Vec<_>), _>>()?;
                let mut replacements =
                    Vec::with_capacity(esc_replacements.len() + expr_replacements.len());
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
                                expr_iter.peek().is_none_or(|(expr_range, _)| {
                                    esc_range.start < expr_range.start
                                })
                            })
                            .or_else(|| expr_iter.next())
                    }));
                }

                // how many bytes of difference between the original string and the processed string
                let mut byte_diff = 0;
                // need to get the correct positions of the expression insertions, since their positions change when we replace substrings
                expressions.extend(
                    replacements
                        .iter()
                        .copied()
                        .filter_map(|(range, repl)| {
                            assert!(
                                byte_diff <= range.start,
                                "shouldn't move tokens backwards\n replacements: {replacements:?}"
                            );
                            let start = range.start - byte_diff;
                            let being_replaced_len = range.end - range.start;
                            let replace_with_len = repl.map_or(0, char::len_utf8);
                            byte_diff += being_replaced_len - replace_with_len;
                            // expression replacements are always None, so if we see a None we know that's an expression.
                            repl.is_none().then_some((range, start))
                        })
                        .zip(expr_scanners)
                        .map(|((range, position), expr)| InterpolatedExpr {
                            range,
                            position,
                            expr,
                        }),
                );

                let mut processed = String::with_capacity(src.len() - byte_diff);
                let mut prev_end = 0;
                for (range, repl) in replacements {
                    processed.push_str(&src[prev_end..range.start]);
                    prev_end = range.end;
                    if let Some(ch) = repl {
                        processed.push(ch);
                    }
                }

                Ok(Some(TokenValue::InterpolatedString(InterpolatedString {
                    text: StringLiteral {
                        text: Cow::Owned(processed),
                        escapes,
                    },
                    expressions,
                })))
            }

            _ => res,
        }
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
                start: end - len,
                end,
            },
            err,
        }
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Result<Token<'a>, ContextError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.source.chars().peekable();
        // if there are no characters remaining, this will return None and stop iterating.
        iter.next()
            // the first character
            .map(|ch| {
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

                // starts with whitespace -> whitespace token
                if ch.is_whitespace() {
                    let len = self
                        .source
                        .find(|ch: char| !ch.is_whitespace())
                        .unwrap_or(self.source.len());
                    Ok(self.split_off_token(len, TokenType::Whitespace))
                }
                // starts with quote -> string/char literal
                // note: identifiers can CONTAIN quotes but cannot START with them
                else if let open_delim @ ('"' | '\'' | '`') = ch {
                    const ESCAPE: char = '\\';
                    let mut is_escaped = false;
                    let len = self.source[open_delim.len_utf8()..]
                        .find(|ch: char| {
                            // unescaped delimiter - end of literal
                            if !is_escaped && ch == open_delim {
                                return true;
                            }
                            // track escapes
                            is_escaped = !is_escaped && ch == ESCAPE;
                            false
                        })
                        // why 2x: first for open delimiter, second for close delimiter (both are the same character)
                        .map(|n| n + 2 * open_delim.len_utf8());
                    len.map(|len| {
                        self.split_off_token(
                            len,
                            match open_delim {
                                '"' => TokenType::StringLiteral,
                                '\'' => TokenType::CharLiteral,
                                '`' => TokenType::InterpolatedString,
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
                // starts with letter or underscore -> identifier
                else if ch.is_alphabetic() || ch == '_' {
                    let len = self
                        .source
                        .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '\'')))
                        .unwrap_or(self.source.len());
                    let src = self.split_off(len);
                    Ok(Token {
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
                    })
                }
                // starts with number or hyphen (where allowed) -> number literal
                else if ch.is_numeric()
                    || self.can_be_negative
                        && ch == '-'
                        && iter.peek().is_some_and(|ch| ch.is_numeric())
                {
                    let mut is_first_decimal = true; // at most one decimal
                    let mut is_first_e_neg = true; // at most one '-' following an 'e'
                    let mut is_prev_e = false;
                    let mut is_following_e = false;
                    let mut len = self.source[ch.len_utf8()..]
                        .find(|ch: char| {
                            let is_end = !(ch.is_alphanumeric()
                                || ch == '.'
                                    && std::mem::take(&mut is_first_decimal)
                                    && !is_following_e
                                || ch == '-' && is_prev_e && std::mem::take(&mut is_first_e_neg));
                            is_prev_e = matches!(ch, 'e' | 'E');
                            is_following_e |= is_prev_e;
                            is_end
                        })
                        .map_or(self.source.len(), |n| n + ch.len_utf8());
                    // no trailing decimal, e (unless hex), or hyphen
                    len = self.source[..len]
                        .trim_end_matches(['.', '-', 'e', 'E']) // TODO: DOESN'T ACCOUNT FOR HEX
                        .len();
                    Ok(self.split_off_token(len, TokenType::NumberLiteral))
                }
                // starts with double forward slashes (`//`) -> (line) comment token
                else if ch == '/' && iter.peek() == Some(&'/') {
                    let len = self
                        .source
                        .lines()
                        .next() // take the first line (excluding newline/return)
                        .expect("the existence of characters should imply the existence of a line")
                        .len();
                    Ok(self.split_off_token(len, TokenType::Comment))
                }
                // starts with forward slash followed by asterisk (`/*`) -> (block) comment token
                else if ch == '/' && iter.peek() == Some(&'*') {
                    const OPEN: &str = "/*";
                    const CLOSE: &str = "*/";
                    let len = self.source[OPEN.len()..]
                        .find(CLOSE)
                        .map(|n| n + const { OPEN.len() + CLOSE.len() });
                    len.map(|len| self.split_off_token(len, TokenType::Comment))
                        .ok_or_else(|| {
                            self.error_here(self.source.len(), ErrorType::EndlessBlockComment)
                        })
                }
                // starts with ascii punctuation -> punctuation
                else if ch.is_ascii_punctuation() {
                    let len = Punctuation::from_prefix(self.source).map(|x| x.as_str().len());
                    len.map(|len| self.split_off_token(len, TokenType::Punctuation))
                        .ok_or_else(|| self.error_here(1, ErrorType::UnknownToken))
                }
                // no other matching pattern -> unknown token
                else {
                    Err(self.error_here(1, ErrorType::UnknownToken))
                }
            })
            .map(|res| {
                res.and_then(|tkn| {
                    tkn.value_noalloc()
                        .map(|_| tkn)
                        .map_err(|err| self.error_prev(tkn.src.len(), err))
                })
            })
            .map(|res| res.map_err(|e| e.add_context(self.original)))
            .inspect(|res| {
                if let Ok(token) = res {
                    // non-whitespace, non-comment token
                    if !matches!(token.ty, TokenType::Whitespace | TokenType::Comment) {
                        // punctuation except for close bracket
                        self.can_be_negative = matches!(token.ty, TokenType::Punctuation)
                            && !matches!(token.src, ")" | "]" | "}");
                    }
                }
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.source.len()))
    }
}

/// [`Scanner`] will never return another element after outputting [`None`].
impl std::iter::FusedIterator for Scanner<'_> {}

pub type TokenResult<'a, T> = Result<(Token<'a>, Option<TokenValue<'a, T>>), ContextError<'a>>;

fn tokenize_uninterpolated(tokens: Scanner<'_>) -> impl Iterator<Item = TokenResult<'_, !>> {
    tokens.map(|item| {
        item.map(|token| {
            let value = token
                .value()
                .expect("should have been caught by scanner")
                .map(|value| {
                    match value {
                    TokenValue::InterpolatedString(..) => {
                        unreachable!(
                            "nested interpolated strings should not be possible; the delimiter does not distinguish open from close"
                        )
                    }
                    TokenValue::UIntLiteral(x) => TokenValue::UIntLiteral(x),
                    TokenValue::SIntLiteral(x) => TokenValue::SIntLiteral(x),
                    TokenValue::FltLiteral(x) => TokenValue::FltLiteral(x),
                    TokenValue::CharLiteral(x) => TokenValue::CharLiteral(x),
                    TokenValue::StringLiteral(x) => TokenValue::StringLiteral(x),
                    TokenValue::Direct(x) => TokenValue::Direct(x),
                    TokenValue::Keyword(x) => TokenValue::Keyword(x),
                    TokenValue::Punctuation(x) => TokenValue::Punctuation(x),
                }});
            (token, value)
        })
    })
}

/// Create a [`Scanner`] for the provided source code, and contextualize errors if there are any
pub fn tokenize(source: &str) -> impl Iterator<Item = TokenResult<'_, Vec<TokenResult<'_, !>>>> {
    Scanner::new(source).map(|item| {
        item.map(|token| {
            let value = token
                .value()
                .expect("should have been caught by scanner")
                .map(|value| match value {
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
                    TokenValue::UIntLiteral(x) => TokenValue::UIntLiteral(x),
                    TokenValue::SIntLiteral(x) => TokenValue::SIntLiteral(x),
                    TokenValue::FltLiteral(x) => TokenValue::FltLiteral(x),
                    TokenValue::CharLiteral(x) => TokenValue::CharLiteral(x),
                    TokenValue::StringLiteral(x) => TokenValue::StringLiteral(x),
                    TokenValue::Direct(x) => TokenValue::Direct(x),
                    TokenValue::Keyword(x) => TokenValue::Keyword(x),
                    TokenValue::Punctuation(x) => TokenValue::Punctuation(x),
                });
            (token, value)
        })
    })
}
