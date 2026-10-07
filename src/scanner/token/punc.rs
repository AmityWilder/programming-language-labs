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
        #[op_desc([Bool] => "logical not", [_] => "bitflip")]
        Not = "!",
        /// Exists - Logical double negation (coerce to boolean)
        #[op_desc([_] => "existence")]
        Exists = "!!",
        /// Stringify - Replace tokens with their lexemes in a macro
        #[op_desc([_] => "token stringification")]
        MacroStringify = "#",
        /// Remainder - Find the remainder of `lhs / rhs`
        #[op_desc([_, _] => "remainder")]
        Rem = "%",
        /// And - Logical AND (booleans) or bitwise AND (integers)
        #[op_desc([Bool, Bool] => "logical AND", [_, _] => "bitwise AND")]
        And = "&",
        /// Left parenthesis
        LParen = "(",
        /// Right parenthesis
        RParen = ")",
        /// Multiply - Find the product of `lhs` and `rhs`
        #[op_desc([_, _] => "multiplication")]
        Mul = "*",
        /// Add - Find the sum of `lhs` and `rhs`
        #[op_desc([_, _] => "addition")]
        Add = "+",
        /// Comma - Separate items in a list
        Comma = ",",
        /// Subtract or negate - Find the difference of `lhs - rhs` or the negation `-rhs`
        #[op_desc([_] => "arithmetic negation", [_, _] => "subtraction")]
        SubNeg = "-",
        /// Dot - Access a rec member
        Dot = ".",
        /// Divide - Find the quotient of `lhs / rhs`
        #[op_desc([_, _] => "division")]
        Div = "/",
        /// Colon - Separate a variable/field/parameter from its type or requirements
        Colon = ":",
        /// Semicolon - Conclude a statement
        Semi = ";",
        /// Less than - Test if `lhs` is strictly lower value compared to `rhs`
        #[op_desc([_, _] => "comparison")]
        Lt = "<",
        /// Assign - Assign `rhs` to `lhs`
        #[op_desc([_, _] => "assignment")]
        Assign = "=",
        /// Greater than - Test if `lhs` is strictly higher value compared to `rhs`
        #[op_desc([_, _] => "comparison")]
        Gt = ">",
        /// Question mark - Causes the entire expression to output `none` if its operand is `none`
        #[op_desc([_] => "coalescence")]
        Coalesce = "?",
        /// Reference - Create a pointer/reference to a value (like to `&` in other languages)
        #[op_desc([_] => "referencing")]
        Ref = "@",
        /// Left bracket
        LBrack = "[",
        /// Right bracket
        RBrack = "]",
        /// Xor - Logical XOR (booleans) or bitwise XOR (integers)
        #[op_desc([Bool, Bool] => "logical XOR", [_, _] => "bitwise XOR")]
        Xor = "^",
        /// Left brace
        LBrace = "{",
        /// Or - Logical OR (booleans) or bitwise OR (integers)
        #[op_desc([Bool, Bool] => "logical OR", [_, _] => "bitwise OR")]
        Or = "|",
        /// Right brace
        RBrace = "}",
        /// Not equal - Equivalent to `!(lhs == rhs)`
        #[op_desc([_, _] => "comparison")]
        Ne = "!=",
        /// Convert - Takes an expression on the left side and a type on the right side, and
        /// converts the expression into the type. Implementation can be fallible or infallible,
        /// depending on the overload
        #[op_desc([_, _] => "conversion")]
        Convert = "-:>",
        /// Transmute - Like [`Self::Convert`], but performs a bitwise reinterpretation as the
        /// output type. Only valid if the input and output types have the exact same data layout.
        #[op_desc([_, _] => "reinterpretation")]
        Transmute = "=:>",
        /// Nand - Equivalent to `!(lhs & rhs)`
        #[op_desc([Bool, Bool] => "logical NAND", [_, _] => "bitwise NAND")]
        Nand = "!&",
        /// Nor - Equivalent to `!(lhs | rhs)`
        #[op_desc([Bool, Bool] => "logical NOR", [_, _] => "bitwise NOR")]
        Nor = "!|",
        /// Xnor - Equivalent to `!(lhs ^ rhs)`
        #[op_desc([Bool, Bool] => "logical XNOR", [_, _] => "bitwise XNOR")]
        Xnor = "!^",
        /// Concatenate - Combine macro arguments without whitespace (possibly forming new tokens)
        #[op_desc([_, _] => "token concatenation")]
        MacroConcat = "##",
        /// Remainder assign - Equivalent to `lhs = lhs % rhs`
        #[op_desc([_, _] => "remainder assignment")]
        RemAssign = "%=",
        /// And assign - Equivalent to `lhs = lhs & rhs`
        #[op_desc([Bool, Bool] => "logical AND assignment", [_, _] => "bitwise AND assignment")]
        AndAssign = "&=",
        /// Multiply assign - Equivalent to `lhs = lhs * rhs`
        #[op_desc([_, _] => "product assignment")]
        MulAssign = "*=",
        /// Power - Put `lhs` to the power of `rhs`
        #[op_desc([_, _] => "power")]
        Pow = "**",
        /// Add assign - Equivalent to `lhs = lhs + rhs`
        #[op_desc([_, _] => "sum assignment")]
        AddAssign = "+=",
        /// Sub assign - Equivalent to `lhs = lhs - rhs`
        #[op_desc([_, _] => "difference assignment")]
        SubAssign = "-=",
        /// Arrow - Separate a function's parameter list from its return type
        Arrow = "->",
        /// Dot dot - Range
        DotDot = "..",
        /// Divide assign - Equivalent to `lhs = lhs / rhs`
        #[op_desc([_, _] => "quotient assignment")]
        DivAssign = "/=",
        /// Path separator - Separate namespace path items
        PathSep = "::",
        /// Colon assign - Assign definition
        ColonEq = ":=",
        /// Less or equal - Equivalent to `lhs < rhs | lhs == rhs`
        #[op_desc([_, _] => "comparison")]
        Le = "<=",
        /// If and only if ("where")
        ///
        /// ## In function definition
        /// Supplies requirements for function parameters.
        /// ### Syntax
        /// ```rs
        /// fn foo(v, fun) -> text
        /// <=>
        ///     v.x: frac,
        ///     v.y: frac,
        ///     fn mag of v: (self) -> frac,
        ///     fn fun: (frac) -> text,
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
        #[op_desc([_, _] => "left bitshift")]
        Shl = "<<",
        /// Equal - Test equality between `lhs` and `rhs`
        #[op_desc([_, _] => "comparison")]
        Eq = "==",
        /// Fat arrow - Separates `match` arm conditions from statements
        FatArrow = "=>",
        /// Greater or equal - Equivalent to `lhs < rhs | lhs == rhs`
        #[op_desc([_, _] => "comparison")]
        Ge = ">=",
        /// Bitshift right - Shift the bits in `lhs` to the right (towards 0) by `rhs` bits
        #[op_desc([_, _] => "right bitshift")]
        Shr = ">>",
        /// Xor assign - Equivalent to `lhs = lhs ^ rhs`
        #[op_desc([Bool, Bool] => "logical XOR assignment", [_, _] => "bitwise XOR assignment")]
        XorAssign = "^=",
        /// Or assign - Equivalent to `lhs = lhs | rhs`
        #[op_desc([Bool, Bool] => "logical OR assignment", [_, _] => "bitwise OR assignment")]
        OrAssign = "|=",
        /// Power assign - Equivalent to `lhs = lhs ** rhs`
        #[op_desc([_, _] => "power assignment")]
        PowAssign = "**=",
        /// Bitshift left assign - Equivalent to `lhs = lhs << rhs`
        #[op_desc([_, _] => "left bitshift assignment")]
        ShlAssign = "<<=",
        /// Bitshift right assign - Equivalent to `lhs = lhs >> rhs`
        #[op_desc([_, _] => "right bitshift assignment")]
        ShrAssign = ">>=",
        /// Nand assign - Equivalent to `lhs = !(lhs & rhs)`
        #[op_desc([Bool, Bool] => "logical NAND assignment", [_, _] => "bitwise NAND assignment")]
        NandAssign = "!&=",
        /// Nor assign - Equivalent to `lhs = !(lhs | rhs)`
        #[op_desc([Bool, Bool] => "logical NOR assignment", [_, _] => "bitwise NOR assignment")]
        NorAssign = "!|=",
        /// Xnor assign - Equivalent to `lhs = !(lhs ^ rhs)`
        #[op_desc([Bool, Bool] => "logical XNOR assignment", [_, _] => "bitwise XNOR assignment")]
        XnorAssign = "!^=",
        /// Bit rotate left - Rotate the bits in `lhs` to the left (away from 0) by `rhs` bits
        #[op_desc([_, _] => "left bit rotation")]
        Rotl = "[<<]",
        /// Bit rotate right - Rotate the bits in `lhs` to the right (towards 0) by `rhs` bits
        #[op_desc([_, _] => "right bit rotation")]
        Rotr = "[>>]",
        /// Bit rotate left assign - Equivalent to `lhs = lhs [<<] rhs`
        #[op_desc([_, _] => "left bit rotation assignment")]
        RotlAssign = "[<<]=",
        /// Bit rotate right assign - Equivalent to `lhs = lhs [>>] rhs`
        #[op_desc([_, _] => "right bit rotation assignment")]
        RotrAssign = "[>>]=",
    }
}
