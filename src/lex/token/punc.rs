//! Punctuation tokens

define_token_eq! {
    /// Operators and other punctuation (but not brackets)
    ///
    /// # Where are the logical operators?
    ///
    /// No distinction is made between bitwise and logical operators.
    /// Booleans are always logical, everything else is always bitwise.
    ///
    /// The only operations that output booleans are
    /// - Boolean literals (`true`/`false`)
    /// - Comparisons
    /// - "Bitwise" (logical) operations on booleans
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Punctuation = operator {
        /// Not - Logical negation (booleans) or bitflip (integers)
        Not = "!",
        /// Exists - Logical double negation (coerce to boolean)
        Exists = "!!",
        /// Stringify - Replace tokens with their lexemes in a macro
        MacroStringify = "#",
        /// Macro Constant - Tells a macro not to expand
        MacroConst = "~",
        /// Remainder - Find the remainder of `lhs / rhs`
        Rem = "%",
        /// And - Logical AND (booleans) or bitwise AND (integers)
        And = "&",
        /// Left parenthesis
        LParen = "(",
        /// Right parenthesis
        RParen = ")",
        /// Multiply - Find the product of `lhs` and `rhs`
        Mul = "*",
        /// Add - Find the sum of `lhs` and `rhs`
        Add = "+",
        /// Increment - Iterate to the next item of `rhs`
        Incr = "++",
        /// Comma - Separate items in a list
        Comma = ",",
        /// Subtract or negate - Find the difference of `lhs - rhs` or the negation `-rhs`
        SubNeg = "-",
        /// Decrement - Iterate to the next item of `rhs` backwards
        Decr = "--",
        /// Dot - Access a rec member
        Dot = ".",
        /// Divide - Find the quotient of `lhs / rhs`
        Div = "/",
        /// Colon - Separate a variable/field/parameter from its type or requirements
        Colon = ":",
        /// Semicolon - Conclude a statement
        Semi = ";",
        /// Less than - Test if `lhs` is strictly lower value compared to `rhs`
        Lt = "<",
        /// Assign - Assign `rhs` to `lhs`
        Assign = "=",
        /// Greater than - Test if `lhs` is strictly higher value compared to `rhs`
        Gt = ">",
        /// Question mark - Causes the entire expression to output `none` if its operand is `none`
        Coalesce = "?",
        /// Reference - Create a pointer/reference to a value (like to `&` in other languages)
        Ref = "@",
        /// Left bracket
        LBrack = "[",
        /// Right bracket
        RBrack = "]",
        /// Xor - Logical XOR (booleans) or bitwise XOR (integers)
        Xor = "^",
        /// Left brace
        LBrace = "{",
        /// Or - Logical OR (booleans) or bitwise OR (integers)
        Or = "|",
        /// Right brace
        RBrace = "}",
        /// Not equal - Equivalent to `!(lhs == rhs)`
        Ne = "!=",
        /// Convert - Takes an expression on the left side and a type on the right side, and
        /// converts the expression into the type. Implementation can be fallible or infallible,
        /// depending on the overload
        Convert = "-:>",
        /// Transmute - Like [`Self::Convert`], but performs a bitwise reinterpretation as the
        /// output type. Only valid if the input and output types have the exact same data layout.
        Transmute = "=:>",
        /// Nand - Equivalent to `!(lhs & rhs)`
        Nand = "!&",
        /// Nor - Equivalent to `!(lhs | rhs)`
        Nor = "!|",
        /// Xnor - Equivalent to `!(lhs ^ rhs)`
        Xnor = "!^",
        /// Concatenate - Combine macro arguments without whitespace (possibly forming new tokens)
        MacroConcat = "##",
        /// Remainder assign - Equivalent to `lhs = lhs % rhs`
        RemAssign = "%=",
        /// And assign - Equivalent to `lhs = lhs & rhs`
        AndAssign = "&=",
        /// Multiply assign - Equivalent to `lhs = lhs * rhs`
        MulAssign = "*=",
        /// Power - Put `lhs` to the power of `rhs`
        Pow = "**",
        /// Add assign - Equivalent to `lhs = lhs + rhs`
        AddAssign = "+=",
        /// Sub assign - Equivalent to `lhs = lhs - rhs`
        SubAssign = "-=",
        /// Arrow - Separate a function's parameter list from its return type
        Arrow = "->",
        /// Dot dot - Range
        DotDot = "..",
        /// Divide assign - Equivalent to `lhs = lhs / rhs`
        DivAssign = "/=",
        /// Path separator - Separate namespace path items
        PathSep = "::",
        /// Colon assign - Assign definition
        ColonEq = ":=",
        /// Less or equal - Equivalent to `lhs < rhs | lhs == rhs`
        Le = "<=",
        /// If and only if ("where")
        ///
        /// ## In function definition
        /// Supplies requirements for function parameters.
        /// ### Syntax
        /// ```rs
        /// fn foo(v, fun) -> text
        /// <=>
        ///     v.x: flpt,
        ///     v.y: flpt,
        ///     fn mag of v: (self) -> flpt,
        ///     fn fun: (flpt) -> text,
        /// {
        ///     // ...
        /// }
        /// ```
        /// When a `<=>` clause is present, any errors that might have been emitted at
        /// the function definition but have been specified in the `<=>` clause, will
        /// instead be attributed to the caller.
        /// **Example:**
        /// ```rs
        /// fn foo(v) {
        ///     return v.x // ERROR: parameter `v` is not guaranteed to have a field `x`;
        ///                // try adding a `<=>` clause or prove `v` has such a field
        /// }
        /// fn bar(v)
        /// <=>
        ///     v has x, // INFO: requirement introduced here
        /// {
        ///     return v.x
        /// }
        /// fn main() {
        ///     foo(5);
        ///     bar(5); // ERROR: argument `v` of `bar` is expected to have a field `x`,
        ///             // but `5` (uint) has no such field
        /// }
        /// ```
        ///
        /// ## In for loops
        /// Filters an iterator.
        /// ### Syntax
        /// ```rs
        /// for /* binding */ in /* iterable */ <=> /* condition */ {
        ///     // statement
        /// }
        /// ```
        /// #### Equivalent to
        /// ```rs
        /// for /* binding */ in /* iterable */ {
        ///     if /* condition */ {
        ///         skip;
        ///     }
        ///     // statement
        /// }
        /// ```
        Iff = "<=>",
        /// Bitshift left - Shift the bits in `lhs` to the left (away from 0) by `rhs` bits
        Shl = "<<",
        /// Equal - Test equality between `lhs` and `rhs`
        Eq = "==",
        /// Fat arrow - Separates `match` arm conditions from statements
        FatArrow = "=>",
        /// Greater or equal - Equivalent to `lhs < rhs | lhs == rhs`
        Ge = ">=",
        /// Bitshift right - Shift the bits in `lhs` to the right (towards 0) by `rhs` bits
        Shr = ">>",
        /// Xor assign - Equivalent to `lhs = lhs ^ rhs`
        XorAssign = "^=",
        /// Or assign - Equivalent to `lhs = lhs | rhs`
        OrAssign = "|=",
        /// Power assign - Equivalent to `lhs = lhs ** rhs`
        PowAssign = "**=",
        /// Bitshift left assign - Equivalent to `lhs = lhs << rhs`
        ShlAssign = "<<=",
        /// Bitshift right assign - Equivalent to `lhs = lhs >> rhs`
        ShrAssign = ">>=",
        /// Nand assign - Equivalent to `lhs = !(lhs & rhs)`
        NandAssign = "!&=",
        /// Nor assign - Equivalent to `lhs = !(lhs | rhs)`
        NorAssign = "!|=",
        /// Xnor assign - Equivalent to `lhs = !(lhs ^ rhs)`
        XnorAssign = "!^=",
        /// Bit rotate left - Rotate the bits in `lhs` to the left (away from 0) by `rhs` bits
        Rotl = "[<<]",
        /// Bit rotate right - Rotate the bits in `lhs` to the right (towards 0) by `rhs` bits
        Rotr = "[>>]",
        /// Bit rotate left assign - Equivalent to `lhs = lhs [<<] rhs`
        RotlAssign = "[<<]=",
        /// Bit rotate right assign - Equivalent to `lhs = lhs [>>] rhs`
        RotrAssign = "[>>]=",
    }
}

/// A bracket character
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum Bracket {
    /// `[`/`]`
    Brack,
    /// `(`/`)`
    Paren,
    /// `{`/`}`
    Brace,
}

impl Bracket {
    /// The open partner of the bracket
    #[must_use]
    pub const fn open(self) -> char {
        match self {
            Self::Brack => '[',
            Self::Paren => '(',
            Self::Brace => '{',
        }
    }

    /// The close partner of the bracket
    #[must_use]
    pub const fn close(self) -> char {
        match self {
            Self::Brack => ']',
            Self::Paren => ')',
            Self::Brace => '}',
        }
    }
}

/// An invalid combination of brackets (order matters)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum BadBracketCombo {
    /// `[`/`)`
    BrackParen,
    /// `[`/`}`
    BrackBrace,
    /// `(`/`]`
    ParenBrack,
    /// `(`/`}`
    ParenBrace,
    /// `{`/`]`
    BraceParen,
    /// `{`/`)`
    BraceBrack,
}

impl BadBracketCombo {
    #[must_use]
    pub const fn new(open: Bracket, close: Bracket) -> Option<Self> {
        match (open, close) {
            (Bracket::Brack, Bracket::Paren) => Some(Self::BrackParen),
            (Bracket::Brack, Bracket::Brace) => Some(Self::BrackBrace),
            (Bracket::Paren, Bracket::Brack) => Some(Self::ParenBrack),
            (Bracket::Paren, Bracket::Brace) => Some(Self::ParenBrace),
            (Bracket::Brace, Bracket::Brack) => Some(Self::BraceBrack),
            (Bracket::Brace, Bracket::Paren) => Some(Self::BraceParen),

            _ => None,
        }
    }

    /// (open, close)
    #[must_use]
    pub const fn decompose(self) -> (Bracket, Bracket) {
        match self {
            Self::BrackParen => (Bracket::Brack, Bracket::Paren),
            Self::BrackBrace => (Bracket::Brack, Bracket::Brace),
            Self::ParenBrack => (Bracket::Paren, Bracket::Brack),
            Self::ParenBrace => (Bracket::Paren, Bracket::Brace),
            Self::BraceParen => (Bracket::Brace, Bracket::Brack),
            Self::BraceBrack => (Bracket::Brace, Bracket::Paren),
        }
    }
}

impl From<BadBracketCombo> for (Bracket, Bracket) {
    #[inline]
    fn from(value: BadBracketCombo) -> Self {
        value.decompose()
    }
}

/// Which side of the operator the operand is on
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpSide {
    Postfix,
    Prefix,
}

impl OpSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Postfix => "lhs",
            Self::Prefix => "rhs",
        }
    }
}

impl std::fmt::Display for OpSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
