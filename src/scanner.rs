use std::{borrow::Cow, range::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType {
    UnknownToken,
    EndlessBlockComment,
    EndlessStringLiteral,
    EscapedStringLiteralEnd,
    InvalidEscape,
    InvalidUIntLiteral(std::num::ParseIntError),
    InvalidSIntLiteral(std::num::TryFromIntError),
    InvalidSIntNegOverflow,
    InvalidFltLiteral(std::num::ParseFloatError),
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownToken => "unknown token pattern",
            Self::EndlessBlockComment => {
                "block comment opens (`/*`) but never closes (missing `*/`)"
            }
            Self::EndlessStringLiteral => {
                "string literal opens (`\"`) but never closes (missing unescaped `\"`)"
            }
            Self::EscapedStringLiteralEnd => {
                "string literal opens (`\"`) but never closes (missing unescaped `\"`); there is a closing double-quote candidate, but it is escaped (`\\\"`)"
            }
            Self::InvalidEscape => "unknown character escape",
            Self::InvalidUIntLiteral(_) |
            Self::InvalidSIntLiteral(_) |
            Self::InvalidSIntNegOverflow |
            Self::InvalidFltLiteral(_) => "invalid number literal",
        })
    }
}

impl std::error::Error for ErrorType {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnknownToken
            | Self::EndlessBlockComment
            | Self::EndlessStringLiteral
            | Self::EscapedStringLiteralEnd
            | Self::InvalidEscape
            | Self::InvalidSIntNegOverflow => None,

            Self::InvalidUIntLiteral(e) => Some(e),
            Self::InvalidSIntLiteral(e) => Some(e),
            Self::InvalidFltLiteral(e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub range: Range<usize>,
    pub err: ErrorType,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            range: Range { start, end },
            err,
        } = self;
        write!(f, "at {start}..{end}: {err}")
    }
}

impl std::error::Error for Error {}

impl Error {
    pub const fn add_context<'a>(self, source: &'a str) -> ContextError<'a> {
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
    pub err: ErrorType,
}

fn line_col(s: &str, position: usize) -> (usize, usize) {
    s[..position]
        .lines()
        .enumerate()
        .last()
        .map(|(row, line)| (/* 1-based index */ row + 1, line.len()))
        .unwrap_or((0, 0))
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

impl std::error::Error for ContextError<'_> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    Whitespace,
    Comment,
    NumberLiteral,
    StringLiteral,
    Identifier,
    Keyword,
    CtrlKeyword,
    Punctuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordType {
    Definition,
    Value,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    // Definitions
    /// `struct`
    Struct,
    /// `union`
    Union,
    /// `enum`
    Enum,
    /// `type`
    Type,

    // halway between Definition and Value
    /// `def`
    Def,
    /// `fn`
    Fn,

    // Value
    /// `let`
    Let,
    /// `const`
    Const,
    /// `static`
    Static,

    // Flow control
    /// `if`
    If,
    /// `else`
    Else,
    /// `for`
    For,
    /// `while`
    While,
    /// `with`
    With,
    /// `where`
    Where,
    /// `loop`
    Loop,
    /// `in`
    In,
}

impl Keyword {
    pub const fn kw_type(self) -> KeywordType {
        match self {
            Self::Struct | Self::Union | Self::Enum | Self::Type => KeywordType::Definition,

            // gray area - isolated for future decision
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "struct" => Some(Self::Struct),
            "union" => Some(Self::Union),
            "enum" => Some(Self::Enum),
            "type" => Some(Self::Type),

            "fn" => Some(Self::Fn),
            "def" => Some(Self::Def),

            "let" => Some(Self::Let),
            "const" => Some(Self::Const),
            "static" => Some(Self::Static),

            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "for" => Some(Self::For),
            "while" => Some(Self::While),
            "with" => Some(Self::With),
            "where" => Some(Self::Where),
            "loop" => Some(Self::Loop),
            "in" => Some(Self::In),

            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Enum => "enum",
            Self::Type => "type",
            Self::Def => "def",

            Self::Let => "let",
            Self::Const => "const",
            Self::Static => "static",

            Self::If => "if",
            Self::Else => "else",
            Self::For => "for",
            Self::While => "while",
            Self::With => "with",
            Self::Where => "where",
            Self::Loop => "loop",
            Self::In => "in",
        }
    }
}

impl std::fmt::Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Punctuation {
    // 1-char
    /// `!`
    Not,
    /// `#`
    MacroArgCount,
    /// `$`
    Ref,
    /// `%`
    Remainder,
    /// `&`
    And,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `*`
    Mul,
    /// `+`
    Add,
    /// `,`
    Comma,
    /// `-`
    Sub,
    /// `.`
    Dot,
    /// `/`
    Div,
    /// `:`
    Colon,
    /// `;`
    Semi,
    /// `<`
    Lt,
    /// `=`
    Assign,
    /// `>`
    Gt,
    /// `?`
    QMark,
    /// `[`
    LBrack,
    /// `]`
    RBrack,
    /// `^`
    Xor,
    /// `{`
    LBrace,
    /// `|`
    Or,
    /// `}`
    RBrace,

    // 2-char
    /// `!=`
    Neq,
    /// `%=`
    RemAssign,
    /// `&=`
    AndAssign,
    /// `*=`
    MulAssign,
    /// `**`
    Exponent,
    /// `+=`
    AddAssign,
    /// `-=`
    SubAssign,
    /// `->`
    Arrow,
    /// `/=`
    DivAssign,
    /// `::`
    PathSep,
    /// `<=`
    Le,
    /// `<<`
    Shl,
    /// `==`
    Eq,
    /// `=>`
    FatArrow,
    /// `>=`
    Ge,
    /// `>>`
    Shr,
    /// `^=`
    XorAssign,
    /// `|=`
    OrAssign,

    // 3-char
    /// `<<=`
    ShlAssign,
    /// `>>=`
    ShrAssign,
}

impl Punctuation {
    /// Descending length, so bigger tokens aren't broken apart by subset tokens
    const OPTIONS: [(&str, Self); 45] = [
        // 3-char
        ("<<=", Self::ShlAssign),
        (">>=", Self::ShrAssign),
        // 2-char
        ("!=", Self::Neq),
        ("%=", Self::RemAssign),
        ("&=", Self::AndAssign),
        ("*=", Self::MulAssign),
        ("**", Self::Exponent),
        ("+=", Self::AddAssign),
        ("-=", Self::SubAssign),
        ("->", Self::Arrow),
        ("/=", Self::DivAssign),
        ("::", Self::PathSep),
        ("<=", Self::Le),
        ("<<", Self::Shl),
        ("==", Self::Eq),
        ("=>", Self::FatArrow),
        (">=", Self::Ge),
        (">>", Self::Shr),
        ("^=", Self::XorAssign),
        ("|=", Self::OrAssign),
        // 1-char
        ("!", Self::Not),
        ("#", Self::MacroArgCount),
        ("$", Self::Ref),
        ("%", Self::Remainder),
        ("&", Self::And),
        ("(", Self::LParen),
        (")", Self::RParen),
        ("*", Self::Mul),
        ("+", Self::Add),
        (",", Self::Comma),
        ("-", Self::Sub),
        (".", Self::Dot),
        ("/", Self::Div),
        (":", Self::Colon),
        (";", Self::Semi),
        ("<", Self::Lt),
        ("=", Self::Assign),
        (">", Self::Gt),
        ("?", Self::QMark),
        ("[", Self::LBrack),
        ("]", Self::RBrack),
        ("^", Self::Xor),
        ("{", Self::LBrace),
        ("|", Self::Or),
        ("}", Self::RBrace),
    ];

    /// Matches the prefix of `s` to a [`Punctuation`]. Tries to find the longest one possible.
    pub fn from_prefix(s: &str) -> Option<Self> {
        Self::OPTIONS
            .into_iter()
            .find(|(pat, _)| s.starts_with(pat))
            .map(|(_, punc)| punc)
    }

    /// Like [`Self::from_prefix`] but matches the full string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "!" => Some(Self::Not),
            "#" => Some(Self::MacroArgCount),
            "$" => Some(Self::Ref),
            "%" => Some(Self::Remainder),
            "&" => Some(Self::And),
            "(" => Some(Self::LParen),
            ")" => Some(Self::RParen),
            "*" => Some(Self::Mul),
            "+" => Some(Self::Add),
            "," => Some(Self::Comma),
            "-" => Some(Self::Sub),
            "." => Some(Self::Dot),
            "/" => Some(Self::Div),
            ":" => Some(Self::Colon),
            ";" => Some(Self::Semi),
            "<" => Some(Self::Lt),
            "=" => Some(Self::Assign),
            ">" => Some(Self::Gt),
            "?" => Some(Self::QMark),
            "[" => Some(Self::LBrack),
            "]" => Some(Self::RBrack),
            "^" => Some(Self::Xor),
            "{" => Some(Self::LBrace),
            "|" => Some(Self::Or),
            "}" => Some(Self::RBrace),

            "!=" => Some(Self::Neq),
            "%=" => Some(Self::RemAssign),
            "&=" => Some(Self::AndAssign),
            "*=" => Some(Self::MulAssign),
            "**" => Some(Self::Exponent),
            "+=" => Some(Self::AddAssign),
            "-=" => Some(Self::SubAssign),
            "->" => Some(Self::Arrow),
            "/=" => Some(Self::DivAssign),
            "::" => Some(Self::PathSep),
            "<=" => Some(Self::Le),
            "<<" => Some(Self::Shl),
            "==" => Some(Self::Eq),
            "=>" => Some(Self::FatArrow),
            ">=" => Some(Self::Ge),
            ">>" => Some(Self::Shr),
            "^=" => Some(Self::XorAssign),
            "|=" => Some(Self::OrAssign),

            "<<=" => Some(Self::ShlAssign),
            ">>=" => Some(Self::ShrAssign),

            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Not => "!",
            Self::MacroArgCount => "#",
            Self::Ref => "$",
            Self::Remainder => "%",
            Self::And => "&",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::Mul => "*",
            Self::Add => "+",
            Self::Comma => ",",
            Self::Sub => "-",
            Self::Dot => ".",
            Self::Div => "/",
            Self::Colon => ":",
            Self::Semi => ";",
            Self::Lt => "<",
            Self::Assign => "=",
            Self::Gt => ">",
            Self::QMark => "?",
            Self::LBrack => "[",
            Self::RBrack => "]",
            Self::Xor => "^",
            Self::LBrace => "{",
            Self::Or => "|",
            Self::RBrace => "}",

            Self::Neq => "!=",
            Self::RemAssign => "%=",
            Self::AndAssign => "&=",
            Self::MulAssign => "*=",
            Self::Exponent => "**",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::Arrow => "->",
            Self::DivAssign => "/=",
            Self::PathSep => "::",
            Self::Le => "<=",
            Self::Shl => "<<",
            Self::Eq => "==",
            Self::FatArrow => "=>",
            Self::Ge => ">=",
            Self::Shr => ">>",
            Self::XorAssign => "^=",
            Self::OrAssign => "|=",

            Self::ShlAssign => "<<=",
            Self::ShrAssign => ">>=",
        }
    }
}

impl std::fmt::Display for Punctuation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The value represented by a [`Token`]
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue<'a> {
    UIntLiteral(usize),
    SIntLiteral(isize),
    FltLiteral(f64),
    /// Escape sequences are converted (unless there are none)
    StringLiteral(Cow<'a, str>),
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

impl<'a> Token<'a> {
    /// Returns [`None`] if non-code (whitespace/comment)
    pub fn value(self) -> Result<Option<TokenValue<'a>>, ErrorType> {
        const VALID_TOKENS: &str = "Token::value() expects vaild tokens";
        match self.ty {
            TokenType::Whitespace | TokenType::Comment => Ok(None),

            TokenType::NumberLiteral => {
                if self.src.contains('.') {
                    self.src
                        .parse()
                        .map(|x| Some(TokenValue::FltLiteral(x)))
                        .map_err(ErrorType::InvalidFltLiteral)
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
                        .map_err(ErrorType::InvalidUIntLiteral)
                        .and_then(|value| {
                            if is_negative {
                                (isize::try_from(value)
                                    .map_err(ErrorType::InvalidSIntLiteral)
                                    .and_then(|x| {
                                        x.checked_neg().ok_or(ErrorType::InvalidSIntNegOverflow)
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
                Ok(Some(TokenValue::StringLiteral(if src.contains('\\') {
                    let mut unescaped = src.to_string();
                    let escape_count_hint = {
                        let mut is_escaped = false;
                        src.chars()
                            .filter(|&ch| {
                                is_escaped = !is_escaped && ch == '\\';
                                is_escaped
                            })
                            .count()
                    };
                    let mut replacements = Vec::with_capacity(escape_count_hint);
                    let mut iter = src.chars().enumerate();
                    while let Some((i, ch)) = iter.find(|(_, ch)| *ch == '\\') {
                        if ch == '\\' {
                            let (j, ch) = iter.next().ok_or(ErrorType::InvalidEscape)?;
                            replacements.push(match ch {
                                repl @ ('\\' | '"') => (Range::from(i..j + ch.len_utf8()), repl),

                                'n' => (Range::from(i..j + ch.len_utf8()), '\n'),
                                'r' => (Range::from(i..j + ch.len_utf8()), '\r'),

                                prefix @ ('x' | 'o' | 'b') => {
                                    let (digits, base) = match prefix {
                                        'x' => (2, 16),
                                        'o' => (3, 8),
                                        'b' => (8, 2),
                                        _ => unreachable!("guarded by outer branch"),
                                    };
                                    let num_start = j + ch.len_utf8();
                                    let end = num_start + digits; // ASCII digits
                                    let num: u8 = src
                                        .get(num_start..end)
                                        .and_then(|n| u8::from_str_radix(n, base).ok())
                                        .ok_or(ErrorType::InvalidEscape)?;
                                    (Range::from(i..end), char::from(num))
                                }

                                _ => return Err(ErrorType::InvalidEscape),
                            });
                        }
                    }
                    for (range, repl) in replacements.into_iter().rev() {
                        unescaped
                            .replace_range(range, repl.encode_utf8(&mut [0; char::MAX_LEN_UTF8]));
                    }
                    Cow::Owned(unescaped)
                } else {
                    Cow::Borrowed(src)
                })))
            }

            TokenType::Identifier => Ok(Some(TokenValue::Direct(self.src))),

            TokenType::Keyword | TokenType::CtrlKeyword => Ok(Some(TokenValue::Keyword(
                Keyword::from_str(self.src).expect(VALID_TOKENS),
            ))),

            TokenType::Punctuation => Ok(Some(TokenValue::Punctuation(
                Punctuation::from_str(self.src).expect(VALID_TOKENS),
            ))),
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
    const fn error_here(&mut self, len: usize, err: ErrorType) -> Error {
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
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.source.chars().peekable();
        // if there are no characters remaining, this will return None and stop iterating.
        iter.next()
            // the first character
            .map(|ch| {
                if ch.is_whitespace() {
                    // starts with whitespace -> whitespace token
                    let len = self
                        .source
                        .find(|ch: char| !ch.is_whitespace())
                        .unwrap_or(self.source.len());
                    Ok(self.split_off_token(len, TokenType::Whitespace))
                } else if ch == '/' && iter.peek() == Some(&'/') {
                    // starts with double forward slashes (`//`) -> (line) comment token
                    let len = self
                        .source
                        .lines()
                        .next() // take the first line (excluding newline/return)
                        .expect("the existence of characters should imply the existence of a line")
                        .len();
                    Ok(self.split_off_token(len, TokenType::Comment))
                } else if ch == '/' && iter.peek() == Some(&'*') {
                    // starts with forward slash followed by asterisk (`/*`) -> (block) comment token
                    const OPEN: &str = "/*";
                    const CLOSE: &str = "*/";
                    let len = self.source[OPEN.len()..]
                        .find(CLOSE)
                        .map(|n| n + const { OPEN.len() + CLOSE.len() });
                    len.map(|len| self.split_off_token(len, TokenType::Comment))
                        .ok_or_else(|| {
                            self.error_here(self.source.len(), ErrorType::EndlessBlockComment)
                        })
                } else if ch.is_alphabetic() || ch == '_' {
                    // starts with letter or underscore -> identifier
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
                        } else {
                            TokenType::Identifier
                        },
                    })
                } else if ch.is_numeric()
                    || self.can_be_negative
                        && ch == '-'
                        && iter.peek().is_some_and(|ch| ch.is_numeric())
                {
                    // starts with number or hyphen (where allowed) -> number literal
                    let mut is_first_decimal = true; // at most one decimal
                    const DECIMAL: char = '.';
                    let mut len = self.source[ch.len_utf8()..]
                        .find(|ch: char| {
                            !(ch.is_alphanumeric()
                                || ch == DECIMAL && std::mem::take(&mut is_first_decimal))
                        })
                        .map(|n| n + ch.len_utf8())
                        .unwrap_or(self.source.len());
                    // no trailing decimal
                    if self.source[..len].ends_with(DECIMAL) {
                        len -= DECIMAL.len_utf8();
                    }
                    Ok(self.split_off_token(len, TokenType::NumberLiteral))
                } else if ch == '"' {
                    // starts with double quote -> string literal
                    let mut is_escaped = false;
                    let len = self.source[ch.len_utf8()..]
                        .find(|ch: char| {
                            // unescaped double-quote - end of string
                            if !is_escaped && ch == '"' {
                                return true;
                            }
                            // track escapes
                            is_escaped = !is_escaped && ch == '\\';
                            false
                        })
                        .map(|n| n + 2 * ch.len_utf8()); // 2x: first for open delimiter, second for close delimiter (both are the same character)
                    len.map(|len| self.split_off_token(len, TokenType::StringLiteral))
                        .ok_or_else(|| {
                            self.error_here(
                                self.source.len(),
                                if self.source.contains("\\\"") {
                                    ErrorType::EscapedStringLiteralEnd
                                } else {
                                    ErrorType::EndlessStringLiteral
                                },
                            )
                        })
                } else if ch.is_ascii_punctuation() {
                    // starts with ascii punctuation -> punctuation
                    let len = Punctuation::from_prefix(self.source).map(|x| x.as_str().len());
                    len.map(|len| self.split_off_token(len, TokenType::Punctuation))
                        .ok_or_else(|| self.error_here(1, ErrorType::UnknownToken))
                } else {
                    // no other matching pattern -> unknown token
                    Err(self.error_here(1, ErrorType::UnknownToken))
                }
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

/// Create a [`Scanner`] for the provided source code, and contextualize errors if there are any
pub fn tokenize<'a>(source: &'a str) -> impl Iterator<Item = Result<Token<'a>, ContextError<'a>>> {
    Scanner::new(source).map(|item| item.map_err(|e| e.add_context(source)))
}
