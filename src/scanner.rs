use std::{borrow::Cow, range::Range};

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
    EndlessStringLiteral,
    EscapedStringLiteralEnd,
    InvalidEscape(&'a str),
    EmptyCharLiteral,
    MultiCharLiteral,
    InvalidNumLiteral(NumLitError),
}

impl std::fmt::Display for ErrorType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownToken => f.write_str("unknown token"),
            Self::EndlessBlockComment => {
                f.write_str("block comment opens (`/*`) but never closes (missing `*/`)")
            }
            Self::EndlessStringLiteral => {
                f.write_str("string literal opens (`\"`) but never closes (missing unescaped `\"`)")
            }
            Self::EscapedStringLiteralEnd => f.write_str(
                "string literal opens (`\"`) but never closes (missing unescaped `\"`). \
                there is a closing double-quote candidate, but it is escaped (`\\\"`). \
                string literals cannot end with an unescaped backslash (`\\`), \
                it is indistinguishable from an escaped double-quote (`\\\"`)",
            ),
            Self::InvalidEscape(s) => write!(f, "unknown character escape: {s}"),
            Self::EmptyCharLiteral => f.write_str("empty character literal"),
            Self::MultiCharLiteral => f.write_str(
                "character literal may only contain one codepoint; \
                if you meant to write a string literal, use double quotes (`\"`). \
                if you meant to write an interpolated string, use graves (`` ` ``).",
            ),
            Self::InvalidNumLiteral(e) => write!(f, "invalid number literal: {e}"),
        }
    }
}

impl std::error::Error for ErrorType<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnknownToken
            | Self::EndlessBlockComment
            | Self::EndlessStringLiteral
            | Self::EscapedStringLiteralEnd
            | Self::InvalidEscape(_)
            | Self::EmptyCharLiteral
            | Self::MultiCharLiteral => None,

            Self::InvalidNumLiteral(e) => Some(e),
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
        if range.is_empty() {
            let code = &source[range.start..]; // TODO: what do we print when we don't know what token should be there?
            write!(
                f,
                "at {start_line}:{start_col}: {err}\n```\n    {code}\n```"
            )
        } else {
            let (end_line, end_col) = line_col(source, range.end);
            let code = &source[range];
            write!(
                f,
                "at {start_line}:{start_col}-{end_line}:{end_col}: {err}\n```\n    {code}\n```"
            )
        }
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
    StringLiteral,
    CharLiteral,
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

#[derive(Debug, Clone, PartialEq)]
pub struct InterpolatedString<'a> {
    pub string: String,
    pub expressions: Vec<(usize, Vec<Token<'a>>)>,
}

/// The value represented by a [`Token`]
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue<'a> {
    UIntLiteral(usize),
    SIntLiteral(isize),
    FltLiteral(f64),
    /// Escape sequences are converted (unless there are none)
    StringLiteral(Cow<'a, str>),
    CharLiteral(char),
    InterpolatedString(InterpolatedString<'a>),
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

fn escape_char(src: &str) -> Option<(usize, Result<char, ()>)> {
    const ESCAPE: char = '\\';
    let mut iter = src.chars();
    iter.next().filter(|ch| *ch == ESCAPE).map(|_| {
        let res = iter.next().ok_or(ESCAPE.len_utf8()).and_then(|ch| {
            let base_len = ESCAPE.len_utf8() + ch.len_utf8();
            match ch {
                '\\' | '"' | '\'' | '`' => Ok((base_len, ch)),

                'n' => Ok((base_len, '\n')),
                'r' => Ok((base_len, '\r')),
                't' => Ok((base_len, '\t')),

                prefix @ ('x' | 'o' | 'b') => {
                    // digits = ceil(256.log(base))
                    // ilog rounds down but we want rounded up
                    let (digits, base) = match prefix {
                        'x' => (2, 16),
                        'o' => (3, 8),
                        'b' => (8, 2),
                        _ => unreachable!("guarded by outer branch"),
                    };
                    let num_start = base_len;
                    let end = num_start + digits; // ASCII digits
                    let len = end - num_start;
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

fn escape_seq(src: &str, i: usize) -> Result<(Range<usize>, char), ErrorType<'_>> {
    escape_char(&src[i..])
        .ok_or(ErrorType::InvalidEscape(&src[i..])) // no remaining characters
        .and_then(|(len, res)| {
            let range = Range::from(i..i + len);
            res.map(|ch| (range, ch))
                .map_err(|()| ErrorType::InvalidEscape(&src[range]))
        })
}

impl<'a> Token<'a> {
    #[cfg(test)]
    pub const fn new(src: &'a str, ty: TokenType) -> Self {
        Self { src, ty }
    }

    /// Obtains the value of a token without allocating
    ///
    /// **Warning:** String literals will be incorrect because of the "no alloc" rule.
    fn value_noalloc(self) -> Result<Option<TokenValue<'a>>, ErrorType<'a>> {
        const VALID_TOKENS: &str = "Token::value() expects vaild tokens";
        match self.ty {
            TokenType::Whitespace | TokenType::Comment => Ok(None),

            TokenType::NumberLiteral => {
                if self.src.contains('.') {
                    self.src
                        .parse()
                        .map(|x| Some(TokenValue::FltLiteral(x)))
                        .map_err(|e| ErrorType::InvalidNumLiteral(NumLitError::Flt(e)))
                } else {
                    let stripped = self.src.strip_prefix('-');
                    let is_negative = stripped.is_some();
                    let magnitude = stripped.unwrap_or(self.src);

                    let (digits, radix) = if let Some(n) = magnitude.strip_prefix("0x") {
                        (n, 16)
                    } else if let Some(n) = magnitude.strip_prefix("0o") {
                        (n, 8)
                    } else if let Some(n) = magnitude.strip_prefix("0b") {
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

            TokenType::StringLiteral => {
                let src = self
                    .src
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .expect("string literal tokens should include delimiters (`\"`)");
                if src.contains('\\')
                    && let Some(e) = src
                        .match_indices('\\')
                        .find_map(|(i, _)| escape_seq(src, i).err())
                {
                    Err(e)
                } else {
                    Ok(Some(TokenValue::StringLiteral(Cow::Borrowed(src))))
                }
            }

            TokenType::CharLiteral => {
                let src = self
                    .src
                    .strip_prefix('\'')
                    .and_then(|s| s.strip_suffix('\''))
                    .expect("string literal tokens should include delimiters (`'`)");
                if let Some((len, res)) = escape_char(src) {
                    // escape sequence
                    res.map_err(|()| ErrorType::InvalidEscape(src))
                        .and_then(|ch| {
                            (len == src.len())
                                .then_some(Some(TokenValue::CharLiteral(ch)))
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
                                .then_some(Some(TokenValue::CharLiteral(ch)))
                                .ok_or(ErrorType::MultiCharLiteral)
                        })
                }
            }

            TokenType::InterpolatedString => {
                println!("not yet implemented: interpolated string");
                Err(ErrorType::UnknownToken)
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
    pub fn value(self) -> Result<Option<TokenValue<'a>>, ErrorType<'a>> {
        let res = self.value_noalloc();
        if let Ok(Some(TokenValue::StringLiteral(Cow::Borrowed(src)))) = res
            && src.contains('\\')
        {
            let replacements = src
                .match_indices('\\')
                .map(|(i, _)| escape_seq(src, i))
                .collect::<Result<Vec<_>, _>>()?;
            let mut unescaped = src.to_string();
            for (range, repl) in replacements.into_iter().rev() {
                unescaped.replace_range(range, repl.encode_utf8(&mut [0; char::MAX_LEN_UTF8]));
            }
            Ok(Some(TokenValue::StringLiteral(Cow::Owned(unescaped))))
        } else {
            res
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scanner<'a> {
    /// A reference to the original source code. Since this is only a copy, it will get ripped apart and fed to the tokens.
    /// The next token will always be at the start of this string.
    source: &'a str,

    offset: usize,

    /// The most recent non-whitespace, non-comment token was either the start of the source code or [`TokenType::Punctuation`]
    /// **and not** `)`, `]`, or `}`.
    can_be_negative: bool,
}

impl<'a> Scanner<'a> {
    pub const fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
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
        self.offset += len;
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
    const fn error_here(&mut self, len: usize, err: ErrorType<'a>) -> Error<'a> {
        let err = Error {
            range: Range {
                start: self.offset,
                end: self.offset + len,
            },
            err,
        };
        _ = self.split_off(1);
        err
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Result<Token<'a>, Error<'a>>;

    #[allow(
        clippy::too_many_lines,
        reason = "don't care. I don't see a need to make an entire function only to call it in one place."
    )]
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
                        } else if self.source.starts_with('(')
                        /* assumes the token has already been split off */
                        {
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
                    const DECIMAL: char = '.';
                    let mut is_first_decimal = true; // at most one decimal
                    let mut len = self.source[ch.len_utf8()..]
                        .find(|ch: char| {
                            !(ch.is_alphanumeric()
                                || ch == DECIMAL && std::mem::take(&mut is_first_decimal))
                        })
                        .map_or(self.source.len(), |n| n + ch.len_utf8());
                    // no trailing decimal
                    if self.source[..len].ends_with(DECIMAL) {
                        len -= DECIMAL.len_utf8();
                    }
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
            .map(|token| {
                token.and_then(|tkn| match tkn.value_noalloc() {
                    Ok(_) => Ok(tkn),
                    Err(err) => Err(Error {
                        range: Range {
                            start: self.offset - tkn.src.len(),
                            end: self.offset,
                        },
                        err,
                    }),
                })
            })
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

pub type TokenResult<'a> = Result<(Token<'a>, Option<TokenValue<'a>>), ContextError<'a>>;

/// Create a [`Scanner`] for the provided source code, and contextualize errors if there are any
pub fn tokenize(source: &str) -> impl Iterator<Item = TokenResult<'_>> {
    Scanner::new(source).map(|item| {
        item.map(|token| {
            (
                token,
                token.value().expect("should have been caught by scanner"),
            )
        })
        .map_err(|e| e.add_context(source))
    })
}
