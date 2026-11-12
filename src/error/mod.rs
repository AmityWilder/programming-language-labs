use crate::{
    SYNTAX_STYLE_ANSI,
    analyzer::types::{AbstractType, SizeReq},
    error::{
        fail::Failure,
        inline::{CodeRef, CodeReference, MessageKind, render_code_refs},
    },
    highlight::{
        style::{Color, Style, StyleWrapper},
        syntax::Syntax,
    },
    lex::{
        symbols::{
            BLOCK_COMMENT_CLOSE, BLOCK_COMMENT_OPEN, CHAR_DELIM, ESCAPE, MACRO_PARAM_PREFIX,
            MACRO_PREFIX, TEXT_DELIM,
        },
        token::{Operator, TokenKind},
    },
    old_error::{ImmutableAssignment, NumErrorKind},
    preproc::eval::{PpType, PpValue},
    scanner::token::{
        escape_char,
        punc::{BadBracketCombo, Bracket, OpSide},
        value::Keyword,
    },
    src_code::{code::CreateSourceCodeError, map::SourceMap, span::Span},
};
use std::fmt::{self, Display, Formatter};

pub mod fail;
pub mod inline;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum NumberBase {
    Bin,
    Oct,
    #[default]
    Dec,
    Hex,
}

impl NumberBase {
    pub const fn as_num(self) -> u8 {
        match self {
            Self::Bin => 2,
            Self::Oct => 8,
            Self::Dec => 10,
            Self::Hex => 16,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Bin => "binary",
            Self::Oct => "octal",
            Self::Dec => "decimal",
            Self::Hex => "hexadecimal",
        }
    }
}

impl From<NumberBase> for u8 {
    fn from(value: NumberBase) -> Self {
        value.as_num()
    }
}

fn invalid_num_suffix(src: &str) -> (&str, NumberBase) {
    #![expect(
        clippy::string_slice,
        reason = "find should not be within a UTF-8 character"
    )]
    let src = src.strip_prefix('-').unwrap_or(src);
    let mut chars = src.chars();
    let base = match chars.nth(1) {
        Some('x') => NumberBase::Hex,
        Some('o') => NumberBase::Oct,
        Some('b') => NumberBase::Bin,
        _ => NumberBase::Dec,
    };
    let digits = if base == NumberBase::Dec {
        src
    } else {
        chars.as_str()
    };
    let mid = digits
        .find(|ch: char| !ch.is_digit(base.as_num().into()))
        .expect("should contain an invalid digit");
    let suffix = &digits[mid..];
    (suffix, base)
}

macro_rules! expected_token {
    (
        $(#[$meta:meta])*
        $vis:vis enum $Enum:ident {$(
            $(#[$vm:meta])*
            $Variant:ident = $desc:expr
        ),* $(,)?}
    ) => {
        $(#[$meta])*
        $vis enum $Enum {
            Verbatim(TokenKind),
        $(
            $(#[$vm])*
            $Variant
        ),*}

        impl $Enum {
            pub const fn as_str(self) -> &'static str {
                match self {
                    Self::Verbatim(x) => x.expectation_name(),
                    $(Self::$Variant => $desc),*
                }
            }
        }
    };
}

expected_token! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
    pub enum ExpectedToken {
        MacroIdent = "a macro identifier (like a regular identifier, but starts with `\\`",
        MacroParamOrRParen = "a macro parameter (like a regular identifier, but starts with `$`) or `)`",
        MacroDef = "a macro definition (should be enclosed in `{...}`)",
        MacroArg = "a macro argument (should be enclosed in `{...}`)",
        MacroPred = "a macro predicate (should be enclosed in `{...}`)",
        MacroCond = "a macro condition body (should be enclosed in `{...}`)",
        MacroCondContinuation = "a macro condition body (should be enclosed in `{...}`) or `rot`",

        #[deprecated = "use ExpectedToken::Verbatim"]
        Colon = "a `:`",
        #[deprecated = "use ExpectedToken::Verbatim"]
        LParen = "a `(`",
        CommaOrRParen = "a `,` or `)`",
        Literal = "a literal",
        ParenExpr = "a parenthesized expression",
        ExprOrRParen = "an expression or `)`",
        BoundOrRParen = "a type bound or `)`",
        TypeOrRAngle = "a type or `>`",
        ExprOrRBrack = "an expression or `]`",
        Expr = "an expression",
        TypeExpr = "a type",
        Typename = "a typename (identifier, optionally prefixed with a `::`-separated path)",
        TypeBound = "a type bound",
        OrType = "an \"or-type\" (`fail`, `none`, or `nevr`)",
        FnSource = "a function name or parenthesized function object expression",
        #[deprecated = "use ExpectedToken::Verbatim"]
        Semi = "a `;`",
        Declaration = "a declaration or statement",
        #[deprecated = "use ExpectedToken::Verbatim"]
        Ident = "an identifier",
        Pattern = "a binding pattern",
        BlockStmt = "a block statement",
        BlockOrIf = "an `if` keyword or a block statement",
        #[deprecated = "use ExpectedToken::Verbatim"]
        InKeyword = "an `in` keyword",
        Statement = "a statement",
        LoopBlock = "a loop block after the label",
        #[deprecated = "use ExpectedToken::Verbatim"]
        Arrow = "a `->`",
        RecBody = "rec body, starting with `{`",
    }
}

impl std::fmt::Display for ExpectedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Remove an article ("a ", "an ", "a(n) ", "the ", or "") from the beginning of a string
#[expect(
    dead_code,
    reason = "reserved for if error expectations ever need to replace \"a\"/\"an\" with \"the\""
)]
fn trim_article(s: &str) -> &str {
    const ARTICLES: [&str; 4] = ["a ", "an ", "a(n) ", "the "];
    for article in ARTICLES {
        if let Some(trimmed) = s.strip_prefix(article) {
            return trimmed;
        }
    }
    s
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DiagnosticKind {
    // ----------------------------
    // lex
    // ----------------------------
    /// Token type could not be identified from the initial character, and so is not a valid token
    UnknownToken,

    /// A block comment has no `*/` to end it
    EndlessBlockComment,

    /// A macro name that is just `\`
    EmptyMacroName,

    /// A macro parameter that is just `$`
    EmptyMacroParam,

    /// A character literal that is just `''`
    EmptyCharLiteral,

    /// A character literal with multiple codepoints
    MultiCharLiteral,

    /// A character literal has no `'` to end it
    EndlessCharLiteral,

    /// A character literal has no `'` to end it, but contains a `\'`
    EscapedCharLiteralEnd,

    /// A string literal has no `"` to end it
    EndlessStringLiteral,

    /// A string literal has no `"` to end it, but contains a `\"`
    EscapedStringLiteralEnd,

    /// A string/character literal contains an escape sequence (identified by a `\`) that does not exist
    InvalidEscape,

    /// A number literal could not be evaluated as a number
    InvalidNumLiteral(NumErrorKind),

    // ----------------------------
    // preprocessor
    // ----------------------------
    /// A macro was encountered that has not been defined
    MacroUndefined,

    MacroParamUndefined {
        /// [`None`] if not in a macro definition
        head: Option<Span>,
    },

    RecursiveMacro {
        head: Span,
    },

    OversizePreproc(CreateSourceCodeError),

    NonBoolPrePredicate {
        actual: PpValue,
    },

    InvalidPreOperator,

    /// The operands in a binary operation are of incompatible type
    PreIncompatible {
        /// The binary operator
        op: Operator,
        l_range: Span,
        l_ty: PpType,
        r_range: Span,
        r_ty: PpType,
    },

    /// The operand in a unary operation is of an unsupported type
    PreUnsupported {
        /// The unary operator
        op: Operator,
        r_range: Span,
        /// The type of the value on the right side of the operator
        r_ty: PpType,
    },

    InvalidPreLiteral,

    InvalidPreType,

    PreOrType,

    // ----------------------------
    // parse
    // ----------------------------
    /// A closing bracket is of the wrong type for the open bracket at its depth
    IncorrectCloseBracket {
        /// The bracket type being expected based on the opening side
        open_range: Span,
        failure: BadBracketCombo,
    },

    /// A closing bracket was found with no open bracket
    ExcessCloseBracket {
        /// The bracket type that was found
        actual: Bracket,
    },

    /// An open bracket was found with no close bracket
    MissingCloseBracket {
        open_range: Span,
        expect: Bracket,
    },

    /// A token was expected, but instead found EOF
    MissingToken {
        /// The token pattern expected - should start with the proper article
        /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
        expected: ExpectedToken,
    },

    /// A token was expected, but instead found `actual`
    UnexpectedToken {
        /// The token pattern expected - should start with the proper article
        /// ('a ', 'an ', 'a(n) ', 'the ', or ''), which will be stripped away
        expected: ExpectedToken,
    },

    // ----------------------------
    // analysis
    // ----------------------------
    /// Attempted to find the quotient or remainder with a denominator of 0
    DivByZero {
        /// The range of the expression evaluating to zero
        zero: Span,
    },

    /// The operands in a binary operation are of incompatible type
    Incompatible {
        /// The binary operator
        op: Operator,
        l_range: Span,
        l_ty: AbstractType<'static>,
        r_range: Span,
        r_ty: AbstractType<'static>,
        candidates: Vec<(
            (AbstractType<'static>, AbstractType<'static>),
            AbstractType<'static>,
        )>,
    },

    /// The operand in a unary operation is of an unsupported type
    Unsupported {
        /// The unary operator
        op: Operator,
        side: OpSide,
        /// The type of the value on the right side of the operator
        ty: AbstractType<'static>,
        range: Span,
        candidates: Vec<((AbstractType<'static>,), AbstractType<'static>)>,
    },

    /// Unsigned cannot be negated
    UnsignedNeg,

    // TODO: make this more helpful
    UnknownVar,

    // TODO: make this more helpful
    /// `.0`: expected
    /// `.1`: actual
    TypeMismatch(
        Box<(
            AbstractType<'static>, // expected
            AbstractType<'static>, // actual
        )>,
    ),

    // TODO: make this more helpful
    TypeAmbiguity,

    AssignImmutable(ImmutableAssignment),

    // TODO: make this more helpful
    FallibleBlockExpr,

    TypeTooBig, // TODO: recursive type should have a separate error

    // TODO: make this more helpful
    IllegalUnsized(SizeReq),

    // TODO: need an error for undefined conversion

    // ----------------------------
    // runtime
    // ----------------------------
    Fail(Failure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCategory {
    Lexical,
    Preprocessor,
    Grammatical,
    StaticAnalysis,
    Runtime,
}

impl DiagnosticCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lexical => "LEX",
            Self::Preprocessor => "PRE",
            Self::Grammatical => "GRA",
            Self::StaticAnalysis => "CMP",
            Self::Runtime => "RUN",
        }
    }
}

impl Display for DiagnosticCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Severity {
    #[default]
    Info,
    Suggestion,
    Warning,
    Error,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Suggestion => "suggestion",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    pub const fn is_err(self) -> bool {
        matches!(self, Self::Error)
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl DiagnosticKind {
    pub const fn severity(&self) -> Severity {
        match self {
            DiagnosticKind::UnknownToken
            | DiagnosticKind::EndlessBlockComment
            | DiagnosticKind::EmptyMacroName
            | DiagnosticKind::EmptyMacroParam
            | DiagnosticKind::EmptyCharLiteral
            | DiagnosticKind::MultiCharLiteral
            | DiagnosticKind::EndlessCharLiteral
            | DiagnosticKind::EscapedCharLiteralEnd
            | DiagnosticKind::EndlessStringLiteral
            | DiagnosticKind::EscapedStringLiteralEnd
            | DiagnosticKind::InvalidEscape
            | DiagnosticKind::InvalidNumLiteral { .. }
            | DiagnosticKind::MacroUndefined
            | DiagnosticKind::MacroParamUndefined { .. }
            | DiagnosticKind::RecursiveMacro { .. }
            | DiagnosticKind::OversizePreproc { .. }
            | DiagnosticKind::NonBoolPrePredicate { .. }
            | DiagnosticKind::InvalidPreOperator
            | DiagnosticKind::PreIncompatible { .. }
            | DiagnosticKind::PreUnsupported { .. }
            | DiagnosticKind::InvalidPreLiteral
            | DiagnosticKind::InvalidPreType
            | DiagnosticKind::PreOrType
            | DiagnosticKind::IncorrectCloseBracket { .. }
            | DiagnosticKind::ExcessCloseBracket { .. }
            | DiagnosticKind::MissingCloseBracket { .. }
            | DiagnosticKind::MissingToken { .. }
            | DiagnosticKind::UnexpectedToken { .. }
            | DiagnosticKind::DivByZero { .. }
            | DiagnosticKind::Incompatible { .. }
            | DiagnosticKind::Unsupported { .. }
            | DiagnosticKind::UnsignedNeg
            | DiagnosticKind::UnknownVar
            | DiagnosticKind::TypeMismatch { .. }
            | DiagnosticKind::TypeAmbiguity
            | DiagnosticKind::AssignImmutable { .. }
            | DiagnosticKind::FallibleBlockExpr
            | DiagnosticKind::TypeTooBig
            | DiagnosticKind::IllegalUnsized { .. }
            | DiagnosticKind::Fail { .. } => Severity::Error,
        }
    }

    pub const fn category(&self) -> DiagnosticCategory {
        match self {
            Self::UnknownToken { .. }
            | Self::EndlessBlockComment { .. }
            | Self::EmptyMacroName { .. }
            | Self::EmptyMacroParam { .. }
            | Self::EmptyCharLiteral { .. }
            | Self::MultiCharLiteral { .. }
            | Self::EndlessCharLiteral { .. }
            | Self::EscapedCharLiteralEnd { .. }
            | Self::EndlessStringLiteral { .. }
            | Self::EscapedStringLiteralEnd { .. }
            | Self::InvalidEscape { .. }
            | Self::InvalidNumLiteral { .. } => DiagnosticCategory::Lexical,

            Self::MacroUndefined { .. }
            | Self::MacroParamUndefined { .. }
            | Self::RecursiveMacro { .. }
            | Self::OversizePreproc { .. }
            | Self::NonBoolPrePredicate { .. }
            | Self::InvalidPreOperator { .. }
            | Self::PreIncompatible { .. }
            | Self::PreUnsupported { .. }
            | Self::InvalidPreLiteral { .. }
            | Self::InvalidPreType { .. }
            | Self::PreOrType { .. } => DiagnosticCategory::Preprocessor,

            Self::IncorrectCloseBracket { .. }
            | Self::ExcessCloseBracket { .. }
            | Self::MissingCloseBracket { .. }
            | Self::MissingToken { .. }
            | Self::UnexpectedToken { .. } => DiagnosticCategory::Grammatical,

            Self::DivByZero { .. }
            | Self::Incompatible { .. }
            | Self::Unsupported { .. }
            | Self::UnsignedNeg { .. }
            | Self::UnknownVar { .. }
            | Self::TypeMismatch { .. }
            | Self::TypeAmbiguity { .. }
            | Self::AssignImmutable { .. }
            | Self::FallibleBlockExpr { .. }
            | Self::TypeTooBig { .. }
            | Self::IllegalUnsized { .. } => DiagnosticCategory::StaticAnalysis,

            Self::Fail { .. } => DiagnosticCategory::Runtime,
        }
    }

    pub const fn num(&self) -> u32 {
        #![expect(clippy::arithmetic_side_effects, reason = "nuh uh")]
        const START: u32 = line!() + 1; // +1 because of `match` line
        (match self {
            Self::UnknownToken { .. } => line!(),
            Self::EndlessBlockComment { .. } => line!(),
            Self::EmptyMacroName { .. } => line!(),
            Self::EmptyMacroParam { .. } => line!(),
            Self::EmptyCharLiteral { .. } => line!(),
            Self::MultiCharLiteral { .. } => line!(),
            Self::EndlessCharLiteral { .. } => line!(),
            Self::EscapedCharLiteralEnd { .. } => line!(),
            Self::EndlessStringLiteral { .. } => line!(),
            Self::EscapedStringLiteralEnd { .. } => line!(),
            Self::InvalidEscape { .. } => line!(),
            Self::InvalidNumLiteral { .. } => line!(),
            Self::MacroUndefined { .. } => line!(),
            Self::MacroParamUndefined { .. } => line!(),
            Self::RecursiveMacro { .. } => line!(),
            Self::OversizePreproc { .. } => line!(),
            Self::NonBoolPrePredicate { .. } => line!(),
            Self::InvalidPreOperator { .. } => line!(),
            Self::PreIncompatible { .. } => line!(),
            Self::PreUnsupported { .. } => line!(),
            Self::InvalidPreLiteral { .. } => line!(),
            Self::InvalidPreType { .. } => line!(),
            Self::PreOrType { .. } => line!(),
            Self::IncorrectCloseBracket { .. } => line!(),
            Self::ExcessCloseBracket { .. } => line!(),
            Self::MissingCloseBracket { .. } => line!(),
            Self::MissingToken { .. } => line!(),
            Self::UnexpectedToken { .. } => line!(),
            Self::DivByZero { .. } => line!(),
            Self::Incompatible { .. } => line!(),
            Self::Unsupported { .. } => line!(),
            Self::UnsignedNeg { .. } => line!(),
            Self::UnknownVar { .. } => line!(),
            Self::TypeMismatch { .. } => line!(),
            Self::TypeAmbiguity { .. } => line!(),
            Self::AssignImmutable { .. } => line!(),
            Self::FallibleBlockExpr { .. } => line!(),
            Self::TypeTooBig { .. } => line!(),
            Self::IllegalUnsized { .. } => line!(),
            Self::Fail { .. } => line!(),
        }) - START
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Diagnostic {
    pub span: Span,
    pub kind: Box<DiagnosticKind>,
}

impl Diagnostic {
    pub fn new(span: Span, kind: DiagnosticKind) -> Self {
        Self {
            span,
            kind: Box::new(kind),
        }
    }

    pub const fn contextualize<'smap, 'err>(
        &'err self,
        source: &'smap SourceMap,
    ) -> ContextDiagnostic<'smap, 'err> {
        ContextDiagnostic::new(source, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextDiagnostic<'smap, 'err> {
    source: &'smap SourceMap,
    error: &'err Diagnostic,
}

impl<'smap, 'err> ContextDiagnostic<'smap, 'err> {
    pub const fn new(source: &'smap SourceMap, error: &'err Diagnostic) -> Self {
        Self { source, error }
    }

    fn code(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let cat = self.error.kind.category();
        let num = self.error.kind.num();
        write!(f, "err[{cat}.{num:>03}]")
    }

    fn err(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.error.kind {
            DiagnosticKind::UnknownToken => write!(f, "unknown token"),
            DiagnosticKind::EndlessBlockComment => {
                write!(
                    f,
                    "block comment opens (`{BLOCK_COMMENT_OPEN}`) but never closes (missing `{BLOCK_COMMENT_CLOSE}`)"
                )
            }
            DiagnosticKind::EmptyMacroName => write!(f, "invalid macro name"),
            DiagnosticKind::EmptyMacroParam => write!(f, "invalid macro parameter name"),
            DiagnosticKind::EmptyCharLiteral => write!(f, "empty character literal"),
            DiagnosticKind::MultiCharLiteral => {
                write!(f, "character literal may only contain one codepoint")
            }
            DiagnosticKind::EndlessCharLiteral | DiagnosticKind::EscapedCharLiteralEnd => {
                write!(
                    f,
                    "char literal opens (`{CHAR_DELIM}`) but never closes (missing unescaped `{CHAR_DELIM}`)"
                )
            }
            DiagnosticKind::EndlessStringLiteral | DiagnosticKind::EscapedStringLiteralEnd => {
                write!(
                    f,
                    "string literal opens (`{TEXT_DELIM}`) but never closes (missing unescaped `{TEXT_DELIM}`)"
                )
            }
            DiagnosticKind::InvalidEscape => {
                write!(
                    f,
                    "unknown character escape: {:?}",
                    self.source.span_str(self.error.span)
                )
            }
            DiagnosticKind::InvalidNumLiteral(e) => {
                write!(f, "invalid number literal: {e}")
            }
            DiagnosticKind::MacroUndefined => {
                write!(f, "macro has not been defined at this point")
            }
            DiagnosticKind::MacroParamUndefined { .. } => {
                write!(f, "macro parameter not in scope")
            }
            DiagnosticKind::RecursiveMacro { .. } => {
                write!(f, "macro call cannot be recursive")
            }
            DiagnosticKind::OversizePreproc(e) => {
                write!(f, "preprocessor generated an invalid file: {e}")
            }
            DiagnosticKind::NonBoolPrePredicate { .. } => {
                write!(f, "preprocessor predicate must evaluate to a boolean")
            }
            DiagnosticKind::InvalidPreOperator => {
                write!(f, "invalid preprocessor operator")
            }
            DiagnosticKind::PreIncompatible { op, l_ty, r_ty, .. } => {
                write!(f, "{l_ty} is not compatible with {r_ty} for `{op}`")
            }
            DiagnosticKind::PreUnsupported { op, r_ty, .. } => {
                write!(f, "`{op}` is not supported for {r_ty}")
            }
            DiagnosticKind::InvalidPreLiteral => {
                write!(f, "invalid preprocessor literal")
            }
            DiagnosticKind::InvalidPreType | DiagnosticKind::PreOrType => {
                write!(f, "invalid preprocessor type")
            }
            DiagnosticKind::IncorrectCloseBracket { failure, .. } => {
                let (expect, actual) = failure.decompose();
                write!(
                    f,
                    "incorrect close bracket: expected `{}`, found `{}`",
                    expect.close(),
                    actual.close(),
                )
            }
            DiagnosticKind::ExcessCloseBracket { actual, .. } => {
                write!(
                    f,
                    "too many close brackets: expected none, found `{}`",
                    actual.close()
                )
            }
            DiagnosticKind::MissingCloseBracket { expect, .. } => {
                write!(
                    f,
                    "missing close bracket: expected `{}`, found none",
                    expect.close()
                )
            }
            DiagnosticKind::MissingToken {
                expected: expect, ..
            } => {
                write!(f, "missing {expect}")
            }
            DiagnosticKind::UnexpectedToken {
                expected: expect, ..
            } => {
                write!(
                    f,
                    "expected {expect}, found `{}`",
                    self.source.span_str(self.error.span)
                )
            }
            DiagnosticKind::DivByZero { .. } => {
                write!(f, "divide by zero")
            }
            DiagnosticKind::Incompatible { op, l_ty, r_ty, .. } => {
                write!(f, "{l_ty} is not compatible with {r_ty} for `{op}`")
            }
            DiagnosticKind::Unsupported { op, ty, .. } => {
                write!(f, "`{op}` is not supported for {ty}")
            }
            DiagnosticKind::UnsignedNeg => {
                write!(f, "unsigned integer cannot be negated")
            }
            DiagnosticKind::UnknownVar => {
                write!(f, "cannot find variable within this scope")
            }
            DiagnosticKind::TypeMismatch(ts) => {
                write!(f, "type mismatch: expected {}, found {}", ts.0, ts.1)
            }
            DiagnosticKind::TypeAmbiguity => {
                write!(f, "type is ambiguous")
            }
            DiagnosticKind::AssignImmutable(_) => {
                write!(f, "cannot assign to immutable")
            }
            DiagnosticKind::FallibleBlockExpr => {
                write!(
                    f,
                    "only `{}` and `{}` are supported loop expressions",
                    Keyword::Loop,
                    Keyword::Cord
                )
            }
            DiagnosticKind::TypeTooBig => {
                write!(
                    f,
                    "type size exceeds {} bytes; too big to store on a {}-bit system",
                    std::mem::size_of::<usize>(),
                    usize::BITS
                )
            }
            DiagnosticKind::IllegalUnsized(_) => {
                write!(
                    f,
                    "runtime-sized type stored in a statically-sized location"
                )
            }
            DiagnosticKind::Fail(_) => {
                write!(f, "unhandled failure")
            }
        }
    }

    fn inline(&self) -> impl fmt::Display {
        match &self.error.kind {
            DiagnosticKind::UnknownToken => "what is this?",
            DiagnosticKind::EndlessBlockComment
            | DiagnosticKind::EndlessCharLiteral
            | DiagnosticKind::EndlessStringLiteral => "never ends",
            DiagnosticKind::EmptyMacroName => "macro prefix, but no name",
            DiagnosticKind::EmptyMacroParam => "macro parameter prefix, but no name",
            DiagnosticKind::EmptyCharLiteral => "empty",
            DiagnosticKind::MultiCharLiteral => "a char should be 1 char",
            DiagnosticKind::EscapedCharLiteralEnd | DiagnosticKind::EscapedStringLiteralEnd => {
                "never ends, unless you remove the `\\`"
            }
            DiagnosticKind::InvalidEscape => "invalid escape sequence",
            DiagnosticKind::InvalidNumLiteral(_) => "not a valid number",
            DiagnosticKind::MacroUndefined => "not defined",
            DiagnosticKind::MacroParamUndefined { .. } => "unknown macro parameter",
            DiagnosticKind::RecursiveMacro { .. } => "macro calls itself here",
            DiagnosticKind::OversizePreproc { .. } => "invalid file",
            DiagnosticKind::NonBoolPrePredicate { .. } => "expected a boolean value",
            DiagnosticKind::InvalidPreOperator => {
                "this operator is not defined within preprocessing"
            }
            DiagnosticKind::PreIncompatible { .. } => {
                "operation is not supported between operands of these types in preprocessing"
            }
            DiagnosticKind::Incompatible { .. } => {
                "operation is not supported between operands of these types"
            }
            DiagnosticKind::PreUnsupported { .. } => {
                "operation is not supported for operand of this type in preprocessing"
            }
            DiagnosticKind::Unsupported { .. } => {
                "operation is not supported for operand of this type"
            }
            DiagnosticKind::InvalidPreLiteral
            | DiagnosticKind::InvalidPreType
            | DiagnosticKind::PreOrType => "not valid within preprocessing",
            DiagnosticKind::IncorrectCloseBracket { .. } => "incorrect partner",
            DiagnosticKind::ExcessCloseBracket { .. } => "missing a partner",
            DiagnosticKind::MissingCloseBracket { .. } => "missing close bracket",
            DiagnosticKind::MissingToken { .. } => "missing token",
            DiagnosticKind::UnexpectedToken { .. } => "unexpected",
            DiagnosticKind::DivByZero { .. } => "dividing by 0",
            DiagnosticKind::UnsignedNeg => "uint can't be negated",
            DiagnosticKind::UnknownVar => "unknown variable",
            DiagnosticKind::TypeMismatch(_) => "incorrect type",
            DiagnosticKind::TypeAmbiguity => "type must be known at this point",
            DiagnosticKind::AssignImmutable(_) => "does not resolve to a mutable value",
            DiagnosticKind::FallibleBlockExpr => "may finish without a result",
            DiagnosticKind::TypeTooBig => "type too big",
            DiagnosticKind::IllegalUnsized(_) => "must have static size",
            DiagnosticKind::Fail(_) => "failed operation",
        }
    }

    fn help(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let src = self.source.span_str(self.error.span);
        match &self.error.kind {
            DiagnosticKind::UnknownToken => {
                write!(f, "try removing the character")
            }
            DiagnosticKind::EndlessBlockComment => {
                write!(f, "try adding `{BLOCK_COMMENT_CLOSE}`")
            }
            DiagnosticKind::EmptyMacroName => write!(f, "try adding an identifier after the `{MACRO_PREFIX}`"),
            DiagnosticKind::EmptyMacroParam => write!(f, "try adding an identifier after the `{MACRO_PARAM_PREFIX}`"),
            DiagnosticKind::EmptyCharLiteral => {
                write!(
                    f,
                    "chars can't be empty, try replacing `{CHAR_DELIM}{CHAR_DELIM}` with `{TEXT_DELIM}{TEXT_DELIM}` or insert a character"
                )
            }
            DiagnosticKind::MultiCharLiteral => {
                    let inner = src
                        .strip_circumfix(CHAR_DELIM, CHAR_DELIM)
                        .expect("char literals should include delimiters");
                    let (first, rest) = inner.split_at(match escape_char(inner) {
                        Some((len, _)) => len,
                        None => inner.chars().next().expect("should have at least one character if MultiCharLiteral instead of EmptyCharLiteral").len_utf8(),
                    });
                    write!(f,
                        "try removing the character(s) after `{first}` (remove trailing `{rest}`) \
                    or change this to a string ({TEXT_DELIM}{inner}{TEXT_DELIM})"
                    )

            }
            DiagnosticKind::EndlessCharLiteral => {
                write!(f, "try adding a `{CHAR_DELIM}` to the end of the char")
            }
            DiagnosticKind::EscapedCharLiteralEnd => {
                    let substr = src.strip_prefix(CHAR_DELIM).expect("string literal should include at least the open delimiter, in EscapedCharLiteralEnd").split_once("\\'").expect("should be EscapedCharLiteralEnd if this is not present").0;
                    write!(f, "there is a closing single-quote candidate, but it is escaped (`{ESCAPE}{CHAR_DELIM}`). \n\
                        char literals cannot end with an unescaped backslash (`{ESCAPE}`), \
                        it is indistinguishable from an escaped single-quote (`{ESCAPE}{CHAR_DELIM}`). \n\
                        try adding a `{CHAR_DELIM}` to the end of the char or remove the `{ESCAPE}` from `{ESCAPE}{CHAR_DELIM}` \
                        to make the char `{CHAR_DELIM}{substr}{CHAR_DELIM}`")

            }
            DiagnosticKind::EndlessStringLiteral => {
                write!(f, "try adding a `{TEXT_DELIM}` to the end of the string")
            }
            DiagnosticKind::EscapedStringLiteralEnd => {
                    let substr = src
                        .strip_prefix(TEXT_DELIM)
                        .expect("string literal should include delimiter")
                        .split_once("\\\"")
                        .expect("should be EndlessStringLiteral if this is not present")
                        .0;
                    write!(f, "there is a closing double-quote candidate, but it is escaped (`{ESCAPE}{TEXT_DELIM}`).\n\
                string literals cannot end with an unescaped backslash (`{ESCAPE}`), \
                it is indistinguishable from an escaped double-quote (`{ESCAPE}{TEXT_DELIM}`).\n\
                try adding a `{TEXT_DELIM}` to the end of the string or remove the `{ESCAPE}` from `{ESCAPE}{TEXT_DELIM}` \
                to make the string `{TEXT_DELIM}{substr}{TEXT_DELIM}`")

            }
            DiagnosticKind::InvalidEscape => {
                    let mut iter = src
                        .strip_prefix(ESCAPE)
                        .expect("InvalidEscape should include `\\`")
                        .chars();
                    let ch = iter.next().expect(
                        "should have at least 2 characters or else be an EscapedStringLiteralEnd",
                    );
                    if ch == 'x' {
                        let n = iter.take(2).filter(char::is_ascii_hexdigit).count();
                        debug_assert_ne!(n, 2, "should be an error");
                        write!(f, "`\\x` should be followed by 2 hexadecimal digits ([0-9a-fA-F]), this escape sequence has {n}")
                    } else if ch == 'o' {
                        let n = iter.take(3).filter(|ch| ch.is_digit(8)).count();
                        assert_ne!(n, 3, "should be an error");
                        write!(f, "`\\o` should be followed by 3 octal digits ([0-7]), this escape sequence has {n}")
                    } else if ch.is_alphabetic() {
                        write!(f, "`\\a`, `\\b`, `\\e`, `\\f`, `\\n`, `\\r`, `\\t`, and `\\v` are the only supported ASCII letters that can be escape sequences")
                    } else if ch.is_numeric() {
                        write!(f, "only ascii digits (0-9) are supported for decimal (base-10) numeric escape sequences")
                    } else {
                        write!(f, "supported escape sequences: `\\a`, `\\b`, `\\e`, `\\f`, `\\n`, `\\r`, `\\t`, `\\v`, `\\0`-`\\9`, `\\x##` (where # is a hexadecimal digit), `\\o###` (where # is an octal digit)")
                    }

            }
            DiagnosticKind::InvalidNumLiteral(e) => {
                match e {
                    NumErrorKind::InvalidInt => {
                        let (suffix, base) = invalid_num_suffix(src);
                        write!(f, "the suffix `{suffix}` is not valid for {} unsigned integer literals", base.name())
                    }
                    NumErrorKind::PosOverflow(kind) => write!(f,
                        "the largest supported {} value is {}",
                        kind.name(),
                        kind.max()
                    ),
                    NumErrorKind::NegOverflow(kind) => write!(f,
                        "the smallest supported {} value is {}",
                        kind.name(),
                        kind.min()
                    ),
                    NumErrorKind::NegUnsigned => {
                        f.write_str("unsigned integers cannot be negative")
                    }
                    NumErrorKind::InvalidFlpt => write!(f,
                        "I'm not sure how to help with this yet"
                    ),
                }
            }
            DiagnosticKind::MacroUndefined => {
                write!(
                    f,
                    "try moving the definition ahead of this usage or fix the errors in the definition"
                )
            }
            DiagnosticKind::MacroParamUndefined { head, .. } => {
                    if head.is_some() {
                        write!(f,
                            "try adding this parameter to the macro, "
                        )?;
                    }
                    write!(f, "remove the `$` if it isn't for a macro, or check if a macro is expanding incorrectly")

            }
            DiagnosticKind::RecursiveMacro { .. } => {
                write!(
                    f,
                    "macros can't be defined recursively. they do not have base cases, so it will always be infinite recursion"
                )
            }
            DiagnosticKind::OversizePreproc { .. } => {
                write!(
                    f,
                    "check if a macro is expanding to something larger than you may have expected"
                )
            }
            DiagnosticKind::NonBoolPrePredicate { actual, .. } => {
                match actual {
                    PpValue::Bool|PpValue::Uint|PpValue::Sint|PpValue::Flpt|PpValue::Char|PpValue::Text => f.write_str("typenames are not values, and especially not booleans. \
                        if you are trying to compare the size of a type with some number, \
                        use the `szof` keyword to get the type's size, and add a comparison operator to get a boolean.\n\n\
                        if you are trying to convert a value to a boolean, you would normally need a `-:>` or `=:>` operator;
                        conversions/reinterpretations are not supported in the preprocessor however, so this is not possible."),
                    PpValue::UIntLiteral(_)|PpValue::SIntLiteral(_) => f.write_str("integers do not coerce to booleans on their own. \
                        try comparing it (`==`, `!=`, `<`, `>`, `<=`, or `>=`) with another number to get a boolean."),
                    PpValue::Macro(_) => f.write_str("if you meant to expand this macro to an expression, remove the `~` operator. \
                        if you are trying to test if the macro exists, add an `!!` (\"exists\") operator in front of the `~`."),
                    PpValue::BoolLiteral(_) => {
                        unimplemented!("should not produce this error on a boolean value");
                    },
                }
            }
            DiagnosticKind::InvalidPreOperator => {
                write!(
                    f,
                    "some operations that are valid at runtime are not valid in preprocessing. check if the operation is supported, or if you may have made a typo."
                )
            }
            DiagnosticKind::PreIncompatible { op, l_ty, r_ty, .. } => {

                    write!(f, "try changing operators, converting types, or wrapping something in parentheses")?;
                    match op {
                        Operator::And|Operator::Xor|Operator::Or|Operator::Nand|Operator::Nor|Operator::Xnor if (*l_ty == PpType::Bool) != (*r_ty == PpType::Bool) => write!(f, "\n\nit looks like you may have been trying to perform a bitwise operation, but one side resolves to a boolean. bitwise/logic operations have lower precedence than comparisons, so you may need to wrap the bitwise operation in parentheses."),
                        _ => Ok(())
                    }

            }
            DiagnosticKind::PreUnsupported { .. } => {
                write!(f, "try a different operator or convert the type")
            }
            DiagnosticKind::InvalidPreLiteral => {
                write!(
                    f,
                    "only bool, uint, sint, and macro names are legal literal types within preprocessing."
                )
            }
            DiagnosticKind::InvalidPreType => {
                write!(
                    f,
                    "only `bool`, `uint`, `sint`, `flpt`, `char`, and `text` are valid typenames within preprocessing."
                )
            }
            DiagnosticKind::PreOrType => {
                write!(f, "\"or-types\" are not legal within preprocessing.")
            }
            DiagnosticKind::IncorrectCloseBracket { failure, .. } => {
                    let (expect, actual) = failure.decompose();
                    write!(f, "try inserting a `{}` before the `{}`, add a `{}` before it and after the `{}`, or remove either the `{}` or the `{}`", expect.close(), actual.close(), actual.open(), expect.open(), expect.open(), actual.close())

            }
            DiagnosticKind::ExcessCloseBracket { actual, .. } => {
                write!(
                    f,
                    "try removing the `{}` or add a `{}` before it",
                    actual.close(),
                    actual.open()
                )
            }
            DiagnosticKind::MissingCloseBracket { expect, .. } => {
                write!(
                    f,
                    "try inserting a `{}` or remove the `{}`",
                    expect.close(),
                    expect.open(),
                )
            }
            DiagnosticKind::MissingToken { expected: expect, .. } => {
                write!(f, "try inserting {expect}")
            }
            DiagnosticKind::UnexpectedToken { expected: expect, .. } => {
                let actual = self.source.span_str(self.error.span);
                write!(
                    f,
                    "try inserting {expect} before the `{actual}` or remove the `{actual}`"
                )
            }
            DiagnosticKind::DivByZero { .. } => {
                write!(f, "ensure the right side cannot be 0")
            }
            DiagnosticKind::Incompatible { candidates, op, .. } => {
                write!(f, "try changing operators, converting types, or wrapping something in parentheses")?;
                if candidates.is_empty() {
                    writeln!(f,
                        "\nno overload exists for `{op}` in this position"
                    )
                } else {
                    writeln!(f, "\nthe following overloads exist:")?;
                    let mut buf = String::new();
                    for ((lhs, rhs), ret) in candidates {
                        use std::fmt::Write;
                        buf.clear();
                        _ = buf
                            .write_fmt(core::format_args!("fun {op}({lhs}, {rhs}) -> {ret}"));
                        write!(f, "  ")?;
                        // TODO: just use the token table
                        // for (lexeme, syntax) in highlight(tokenize(&buf)) {
                        //     _ = write!(f,
                        //         "{}",
                        //         SYNTAX_STYLE_ANSI[syntax].style(lexeme)
                        //     );
                        // }
                        writeln!(f)?;
                    }
                    Ok(())
                }
            }
            DiagnosticKind::Unsupported {
                candidates,
                op,
                side,
                ..
            } => {
                write!(f,
                    "try a different operator or convert the type"
                )?;
                if candidates.is_empty() {
                    writeln!(f,
                        "\nno overload exists for `{op}` in this position"
                    )
                } else {
                    writeln!(f, "\nthe following overloads exist:")?;
                    for ((arg,), ret) in candidates {
                        SYNTAX_STYLE_ANSI[Syntax::Keyword].style(Keyword::Fun).fmt(f)?;

                        f.write_str(" ")?;
                        if *side == OpSide::Prefix {
                            write!(f, "{op:#}")?; // TODO: highlight
                        }

                        f.write_str("(")?;
                        write!(f, "{arg:#}")?;
                        f.write_str(")")?;

                        if *side == OpSide::Postfix {
                            write!(f, "{op:#}")?; // TODO: highlight
                        }

                        writeln!(f, " -> {ret:#}")?;
                    }
                    Ok(())
                }
            }
            DiagnosticKind::UnsignedNeg => {
                write!(f, "remove the `-` or convert the integer to signed")
            }
            DiagnosticKind::UnknownVar => {
                write!(
                    f,
                    "check if the variable is in a tighter scope than where you are trying to use it"
                )
            }
            DiagnosticKind::TypeMismatch(_) => {
                write!(
                    f,
                    "check if you are using a different type from what you intended, or create a new variable to shadow this one"
                )
            }
            DiagnosticKind::TypeAmbiguity => {
                write!(
                    f,
                    "try annotating the type or use an expression with an explicit type"
                )
            }
            DiagnosticKind::AssignImmutable(x) => {
                match x {
                    ImmutableAssignment::Expression => write!(f, "try replacing this with an expression that resolves to a mutable reference"),
                    ImmutableAssignment::Assignment => write!(f, "assignments cannot be chained, try breaking them up into separate statements"),
                    ImmutableAssignment::Literal => write!(f, "literals cannot be assigned, perhaps you meant to put this on the right side?"),
                    ImmutableAssignment::Keyword(kw) => write!(f, "`{kw}` is a reserved keyword, and doesn't resolve to a mutable value"),
                }
            }
            DiagnosticKind::FallibleBlockExpr => {
                write!(
                    f,
                    "try changing the expression to a `loop` or `cord` or remove the assignment"
                )
            }
            DiagnosticKind::TypeTooBig => {
                write!(
                    f,
                    "consider making the type smaller (I have no idea how it got so big)"
                )
            }
            DiagnosticKind::IllegalUnsized(_) => {
                write!(
                    f,
                    "try adding indirection, or replace the type with a statically-sized one"
                )
            }
            DiagnosticKind::Fail(_) => {
                write!(f, "try handling the error")
            }
        }
    }

    // fn info(&self) -> Vec<CodeRef<_, _>> {
    //     match &self.error.kind {
    //         ErrorKind::MacroParamUndefined { head, .. } => {
    //             for (range, msg) in (head.iter().map(|&range| (range, "macro defined here"))) {
    //                 visit(LineRef::new(
    //                     self.source,
    //                     RefStyleKind::Info,
    //                     range,
    //                     msg.into(),
    //                 ));
    //             }
    //         }
    //         ErrorKind::RecursiveMacro { head, .. } => {
    //             for (range, msg) in [(*head, "macro defined here")] {
    //                 visit(LineRef::new(
    //                     self.source,
    //                     RefStyleKind::Info,
    //                     range,
    //                     msg.into(),
    //                 ));
    //             }
    //         }
    //         ErrorKind::IncorrectCloseBracket { open_range, .. } => {
    //             for (range, msg) in [(*open_range, "bracket type introduced here")] {
    //                 visit(LineRef::new(
    //                     self.source,
    //                     RefStyleKind::Info,
    //                     range,
    //                     msg.into(),
    //                 ));
    //             }
    //         }
    //         ErrorKind::MissingCloseBracket { open_range, .. } => {
    //             for (range, msg) in [(*open_range, "missing a partner")] {
    //                 visit(LineRef::new(
    //                     self.source,
    //                     RefStyleKind::Info,
    //                     range,
    //                     msg.into(),
    //                 ));
    //             }
    //         }
    //         ErrorKind::DivByZero { zero, .. } => {
    //             for (range, msg) in [(*zero, "this expression evaluates to 0")] {
    //                 visit(LineRef::new(
    //                     self.source,
    //                     RefStyleKind::Info,
    //                     range,
    //                     msg.into(),
    //                 ));
    //             }
    //         }
    //         ErrorKind::Incompatible {
    //             l_ty,
    //             l_range,
    //             r_ty,
    //             r_range,
    //             ..
    //         } => {
    //             for (range, msg) in [
    //                 (
    //                     *l_range,
    //                     TypeResolutionMsg {
    //                         side: OpSide::Postfix,
    //                         ty: l_ty.clone(),
    //                     },
    //                 ),
    //                 (
    //                     *r_range,
    //                     TypeResolutionMsg {
    //                         side: OpSide::Prefix,
    //                         ty: r_ty.clone(),
    //                     },
    //                 ),
    //             ] {
    //                 visit(LineRef::new(
    //                     self.source,
    //                     RefStyleKind::Info,
    //                     range,
    //                     msg.into(),
    //                 ));
    //             }
    //         }
    //         ErrorKind::Unsupported {
    //             ty, range, side, ..
    //         } => {
    //             for (range, msg) in [(
    //                 *range,
    //                 TypeResolutionMsg {
    //                     side: *side,
    //                     ty: ty.clone(),
    //                 },
    //             )] {
    //                 visit(LineRef::new(
    //                     self.source,
    //                     RefStyleKind::Info,
    //                     range,
    //                     msg.into(),
    //                 ));
    //             }
    //         }
    //         _ => {}
    //     }
    // }
}

struct DisplayWith<F>(F);

impl<F: Fn(&mut Formatter<'_>) -> fmt::Result> fmt::Display for DisplayWith<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0(f)
    }
}

impl Display for ContextDiagnostic<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}error{}: ",
            Style::new().foreground(Color::BrightRed).bold(true).begin(),
            Style::new().foreground(Color::BrightWhite).begin()
        )?;
        self.err(f)?;
        writeln!(f, "\x1b[0m")?;
        write!(
            f,
            "  {}-->\x1b[0m ",
            Style::new()
                .foreground(Color::BrightCyan)
                .bold(true)
                .begin()
        )?;
        self.error.span.display_loc(self.source).fmt(f)?;
        writeln!(f)?;

        // render
        {
            let mut refs: Vec<Box<dyn CodeReference>> = vec![Box::new(CodeRef::new(
                MessageKind::Error,
                self.error.span,
                self.inline(),
            ))];

            // TODO: recursive macro lookup
            // if let Some(macro_range) = self.0.mac {
            //     refs.push(CodeRef::new(MessageKind::Info, self.error.span, todo!()));
            // }

            // self.0.info_line(|item| {
            //     refs.push(item);
            // });

            render_code_refs(self.source, refs, f)?;
        }

        write!(
            f,
            "{}help\x1b[0m: ",
            Style::new()
                .foreground(Color::BrightCyan)
                .bold(true)
                .begin()
        )?;
        self.help(f)?;
        Ok(())
    }
}
