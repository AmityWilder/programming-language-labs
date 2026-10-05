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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Punctuation = operator {
        /// Not - Logical negation (booleans) or bitflip (integers)
        Not = "!",
        /// Exists - Logical double negation (coerce to boolean)
        Exists = "!!",
        /// Stringify - Replace tokens with their lexemes in a macro
        MacroStringify = "#",
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
        /// Comma - Separate items in a list
        Comma = ",",
        /// Subtract or negate - Find the difference of `lhs - rhs` or the negation `-rhs`
        SubNeg = "-",
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
        /// Question mark - TBD
        QMark = "?",
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
