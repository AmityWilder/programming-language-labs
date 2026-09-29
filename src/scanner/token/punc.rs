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
    /// - "Bitwise" operations on booleans
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Punctuation = operator {
        // ----------------------------
        // 1-char
        // ----------------------------

        /// Not - Logical negation (booleans) or bitflip (integers)
        Not = a "!" ("not") as NotOp,
        /// Stringify - Replace tokens with their lexemes in a macro
        MacroStringify = a "#" ("stringify") as MacroStringifyOp,
        /// Remainder - Find the remainder of `lhs / rhs`
        Remainder = a "%" ("remainder") as RemainderOp,
        /// And - Logical AND (booleans) or bitwise AND (integers)
        And = an "&" ("and") as AndOp,
        /// Left parenthesis
        LParen = a "(" ("left parenthesis") as LParenOp,
        /// Right parenthesis
        RParen = a ")" ("right parenthesis") as RParenOp,
        /// Multiply - Find the product of `lhs` and `rhs`
        Mul = a "*" ("multiply") as MulOp,
        /// Add - Find the sum of `lhs` and `rhs`
        Add = an "+" ("add") as AddOp,
        /// Comma - Separate items in a list
        Comma = a "," ("comma") as CommaOp,
        /// Subtract - Find the difference of `lhs - rhs`
        Sub = a "-" ("subtract") as SubOp,
        /// Dot - Access a rec member
        Dot = a "." ("dot") as DotOp,
        /// Divide - Find the quotient of `lhs / rhs`
        Div = a "/" ("divide") as DivOp,
        /// Colon - Separate a variable/field/parameter from its type or requirements
        Colon = a ":" ("colon") as ColonOp,
        /// Semicolon - Conclude a statement
        Semi = a ";" ("semicolon") as SemiOp,
        /// Less than - Test if `lhs` is strictly lower value compared to `rhs`
        Lt = a "<" ("less than") as LtOp,
        /// Assign - Assign `rhs` to `lhs`
        Assign = an "=" ("assignment") as AssignOp,
        /// Greater than - Test if `lhs` is strictly higher value compared to `rhs`
        Gt = a ">" ("greater than") as GtOp,
        /// Question mark - TBD
        QMark = a "?" ("question mark") as QMarkOp,
        /// Reference - Create a pointer/reference to a value (like to `&` in other languages)
        Ref = a "@" ("reference") as RefOp,
        /// Left bracket
        LBrack = a "[" ("left bracket") as LBrackOp,
        /// Right bracket
        RBrack = a "]" ("right bracket") as RBrackOp,
        /// Xor - Logical XOR (booleans) or bitwise XOR (integers)
        Xor = an "^" ("xor") as XorOp,
        /// Left brace
        LBrace = a "{" ("left brace") as LBraceOp,
        /// Or - Logical OR (booleans) or bitwise OR (integers)
        Or = an "|" ("or") as OrOp,
        /// Right brace
        RBrace = a "}" ("right brace") as RBraceOp,

        // ----------------------------
        // 2-char
        // ----------------------------

        /// Not equal - Equivalent to `!(lhs == rhs)`
        Neq = a "!=" ("not equal") as NeqOp,
        /// Nand - Equivalent to `!(lhs & rhs)`
        Nand = a "!&" ("nand") as NandOp,
        /// Nor - Equivalent to `!(lhs | rhs)`
        Nor = a "!|" ("nor") as NorOp,
        /// Xnor - Equivalent to `!(lhs ^ rhs)`
        Xnor = an "!^" ("xnor") as XnorOp,
        /// Concatenate - Combine macro arguments without whitespace (possibly forming new tokens)
        MacroConcatenate = a "##" ("concatenate") as MacroConcatenateOp,
        /// Remainder assign - Equivalent to `lhs = lhs % rhs`
        RemAssign = a "%=" ("remainder assign") as RemAssignOp,
        /// And assign - Equivalent to `lhs = lhs & rhs`
        AndAssign = an "&=" ("and assign") as AndAssignOp,
        /// Multiply assign - Equivalent to `lhs = lhs * rhs`
        MulAssign = a "*=" ("multiply assign") as MulAssignOp,
        /// Exponent - Put `lhs` to the power of `rhs`
        Exp = an "**" ("exponent") as ExponentOp,
        /// Add assign - Equivalent to `lhs = lhs + rhs`
        AddAssign = an "+=" ("add assign") as AddAssignOp,
        /// Sub assign - Equivalent to `lhs = lhs - rhs`
        SubAssign = a "-=" ("subtract assign") as SubAssignOp,
        /// Arrow - Separate a function's parameter list from its return type
        Arrow = an "->" ("arrow") as ArrowOp,
        /// Dot dot - Range
        DotDot = a ".." ("dot dot") as DotDotOp,
        /// Divide assign - Equivalent to `lhs = lhs / rhs`
        DivAssign = a "/=" ("divide assign") as DivAssignOp,
        /// Path separator - Separate namespace path items
        PathSep = a "::" ("path separator") as PathSepOp,
        /// Colon assign - Assign definition
        ColonEq = a ":=" ("colon assign") as ColonEqOp,
        /// Less or equal - Equivalent to `lhs < rhs | lhs == rhs`
        Le = a "<=" ("less or equal") as LeOp,
        /// Bitshift left - Shift the bits in `lhs` to the left (away from 0) by `rhs` bits
        Shl = a "<<" ("left bitshift") as ShlOp,
        /// Equal - Test equality between `lhs` and `rhs`
        Eq = an "==" ("equal") as EqOp,
        /// Fat arrow - Separates `match` arm conditions from statements
        FatArrow = a "=>" ("fat arrow") as FatArrowOp,
        /// Greater or equal - Equivalent to `lhs < rhs | lhs == rhs`
        Ge = a ">=" ("greater or equal") as GeOp,
        /// Shr - Shift the bits in `lhs` to the right (towards 0) by `rhs` bits
        Shr = a ">>" ("right bitshift") as ShrOp,
        /// Xor assign - Equivalent to `lhs = lhs ^ rhs`
        XorAssign = an "^=" ("xor assign") as XorAssignOp,
        /// Or assign - Equivalent to `lhs = lhs | rhs`
        OrAssign = an "|=" ("or assign") as OrAssignOp,

        // ----------------------------
        // 3-char
        // ----------------------------

        /// Exponent assign - Equivalent to `lhs = lhs ** rhs`
        ExpAssign = an "**=" ("exponent assign") as ExpAssignOp,
        /// Bitshift left assign - Equivalent to `lhs = lhs << rhs`
        ShlAssign = a "<<=" ("left bitshift assign") as ShlAssignOp,
        /// Bitshift right assign - Equivalent to `lhs = lhs >> rhs`
        ShrAssign = a ">>=" ("right bitshift assign") as ShrAssignOp,
        /// Nand assign - Equivalent to `lhs = lhs !& rhs`
        NandAssign = a "!&=" ("nand assign") as NandAssignOp,
        /// Nor assign - Equivalent to `lhs = lhs !| rhs`
        NorAssign = a "!|=" ("nor assign") as NorAssignOp,
        /// Xnor assign - Equivalent to `lhs = lhs !^ rhs`
        XnorAssign = a "!^=" ("xnor assign") as XnorAssignOp,
    }
}
