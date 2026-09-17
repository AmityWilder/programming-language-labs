use std::range::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    UnknownToken,
    EndlessBlockComment,
    EndlessStringLiteral,
    EscapedStringLiteralEnd,
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
        })
    }
}

impl std::error::Error for ErrorType {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        let Self { source, range, err } = *self;
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
pub enum Keyword {
    // Definitions
    /// `fn`
    Fn,
    /// `struct`
    Struct,
    /// `type`
    Type,

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

    // Value
    /// `let`
    Let,
    /// `const`
    Const,
    /// `static`
    Static,
}

impl Keyword {
    pub const fn is_definition(self) -> bool {
        matches!(self, Self::Fn | Self::Struct | Self::Type)
    }

    pub const fn is_control(self) -> bool {
        matches!(self, |Self::If| Self::Else
            | Self::For
            | Self::While
            | Self::With
            | Self::Where
            | Self::Loop
            | Self::In)
    }

    pub const fn is_value(self) -> bool {
        matches!(self, |Self::Let| Self::Const | Self::Static)
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "fn" => Some(Self::Fn),
            "struct" => Some(Self::Struct),
            "type" => Some(Self::Type),

            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "for" => Some(Self::For),
            "while" => Some(Self::While),
            "with" => Some(Self::With),
            "where" => Some(Self::Where),
            "loop" => Some(Self::Loop),
            "in" => Some(Self::In),

            "let" => Some(Self::Let),
            "const" => Some(Self::Const),
            "static" => Some(Self::Static),

            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::Struct => "struct",
            Self::Type => "type",

            Self::If => "if",
            Self::Else => "else",
            Self::For => "for",
            Self::While => "while",
            Self::With => "with",
            Self::Where => "where",
            Self::Loop => "loop",
            Self::In => "in",

            Self::Let => "let",
            Self::Const => "const",
            Self::Static => "static",
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
    pub fn from_prefix(s: &str) -> Option<Self> {
        // descending length, so bigger tokens aren't broken apart by subset tokens
        const OPTIONS: [(&str, Punctuation); 43] = [
            // 3-char
            ("<<=", Punctuation::ShlAssign),
            (">>=", Punctuation::ShrAssign),
            // 2-char
            ("!=", Punctuation::Neq),
            ("%=", Punctuation::RemAssign),
            ("&=", Punctuation::AndAssign),
            ("*=", Punctuation::MulAssign),
            ("**", Punctuation::Exponent),
            ("+=", Punctuation::AddAssign),
            ("-=", Punctuation::SubAssign),
            ("->", Punctuation::Arrow),
            ("/=", Punctuation::DivAssign),
            ("::", Punctuation::PathSep),
            ("<=", Punctuation::Le),
            ("<<", Punctuation::Shl),
            ("==", Punctuation::Eq),
            ("=>", Punctuation::FatArrow),
            (">=", Punctuation::Ge),
            (">>", Punctuation::Shr),
            ("^=", Punctuation::XorAssign),
            ("|=", Punctuation::OrAssign),
            // 1-char
            ("!", Punctuation::Not),
            ("%", Punctuation::Remainder),
            ("&", Punctuation::And),
            ("(", Punctuation::LParen),
            (")", Punctuation::RParen),
            ("*", Punctuation::Mul),
            ("+", Punctuation::Add),
            (",", Punctuation::Comma),
            ("-", Punctuation::Sub),
            (".", Punctuation::Dot),
            ("/", Punctuation::Div),
            (":", Punctuation::Colon),
            (";", Punctuation::Semi),
            ("<", Punctuation::Lt),
            ("=", Punctuation::Assign),
            (">", Punctuation::Gt),
            ("?", Punctuation::QMark),
            ("[", Punctuation::LBrack),
            ("]", Punctuation::RBrack),
            ("^", Punctuation::Xor),
            ("{", Punctuation::LBrace),
            ("|", Punctuation::Or),
            ("}", Punctuation::RBrace),
        ];

        OPTIONS
            .into_iter()
            .find(|(pat, _)| s.starts_with(pat))
            .map(|(_, punc)| punc)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Punctuation::Not => "!",
            Punctuation::Remainder => "%",
            Punctuation::And => "&",
            Punctuation::LParen => "(",
            Punctuation::RParen => ")",
            Punctuation::Mul => "*",
            Punctuation::Add => "+",
            Punctuation::Comma => ",",
            Punctuation::Sub => "-",
            Punctuation::Dot => ".",
            Punctuation::Div => "/",
            Punctuation::Colon => ":",
            Punctuation::Semi => ";",
            Punctuation::Lt => "<",
            Punctuation::Assign => "=",
            Punctuation::Gt => ">",
            Punctuation::QMark => "?",
            Punctuation::LBrack => "[",
            Punctuation::RBrack => "]",
            Punctuation::Xor => "^",
            Punctuation::LBrace => "{",
            Punctuation::Or => "|",
            Punctuation::RBrace => "}",

            Punctuation::Neq => "!=",
            Punctuation::RemAssign => "%=",
            Punctuation::AndAssign => "&=",
            Punctuation::MulAssign => "*=",
            Punctuation::Exponent => "**",
            Punctuation::AddAssign => "+=",
            Punctuation::SubAssign => "-=",
            Punctuation::Arrow => "->",
            Punctuation::DivAssign => "/=",
            Punctuation::PathSep => "::",
            Punctuation::Le => "<=",
            Punctuation::Shl => "<<",
            Punctuation::Eq => "==",
            Punctuation::FatArrow => "=>",
            Punctuation::Ge => ">=",
            Punctuation::Shr => ">>",
            Punctuation::XorAssign => "^=",
            Punctuation::OrAssign => "|=",

            Punctuation::ShlAssign => "<<=",
            Punctuation::ShrAssign => ">>=",
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
    StringLiteral(String),
    Direct(&'a str),
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
                    const OPEN: &str = "//";
                    // take the first line (excluding newline/return)
                    let len = self
                        .source
                        .lines()
                        .next()
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
                            if kw.is_control() {
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
