//! Definitions of tokens and their values.

use crate::{
    error::{ErrorType, NumLitError},
    scanner::symbols::{BIN_PREFIX, CHAR_DELIM, ESCAPE, HEX_PREFIX, OCT_PREFIX, STR_DELIM},
};
use std::range::Range;

/// Helper macro for preventing issues with missed variants when adding new ones
///
/// Variants should be in the order they should be tested
macro_rules! define_token_eq {
    (
        $(#[$em:meta])*
        $vis:vis enum $Enum:ident = $name:ident {$(
            $(#[$vm:meta])*
            $Variant:ident = $article:ident $value:literal $(($val_name:literal))? as $rule:ident
        ),+ $(,)?}
    ) => {
        $(#[$em])*
        $vis enum $Enum {$(
            #[doc = concat!("`", $value, "`\n")]
            $(#[$vm])*
            $Variant,
        )+}

        #[allow(dead_code, reason = "not always used in all expressions of this macro")]
        impl $Enum {
            /// Descending length, so bigger tokens aren't broken apart by subset tokens
            pub const OPTIONS: [(&str, Self); [$(Self::$Variant),+].len()] = [
                $(($value, Self::$Variant),)+
            ];

            /// Matches the prefix of `s` to a [`Self`]. Tries to find the longest one possible.
            pub fn from_prefix(s: &str) -> Option<Self> {
                Self::OPTIONS
                    .into_iter()
                    .find(|(pat, _)| s.starts_with(pat))
                    .map(|(_, punc)| punc)
            }

            /// Like [`Self::from_prefix`] but matches the full string
            pub fn try_from_str(s: &str) -> Option<Self> {
                match s {
                    $($value => Some(Self::$Variant),)+
                    _ => None,
                }
            }

            /// The constant string name of the token
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

        pub mod $name {
            use super::*;
            $(
                $crate::terminal_rule!{
                    #[doc = concat!("`\"", $value, "\"`")]
                    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
                    #[allow(dead_code)]
                    pub struct $rule<'a>(pub &'a str)
                        := (lex, val: TokenValue::$Enum($Enum::$Variant)) => (Self(lex))
                        as $article concat!("`", $value, "` ", $("(", $val_name, ") ",)? stringify!($name));
                }
            )+
        }
    };
}

define_token_eq! {
    /// Language-defined reserved words for defining behavior or form
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Keyword = keyword {
        // ----------------------------
        // Builtin values
        // ----------------------------

        /// The value a method is being called on
        SelfKw = a "self" as SelfKeyword,

        // ----------------------------
        // Builtin types
        // ----------------------------

        /// ## As a value
        /// Absence of value
        /// ## As a type
        /// Absence of a result (return type of an empty-bodied function)
        None = a "none" as NoneKeyword,
        /// ## As a value
        /// Unreachable (crash if accessed)
        /// ## As a type
        /// Unreachable (return type of `stop`less `loop {}`)
        Nevr = a "nevr" as NevrKeyword,
        /// Boolean type
        Bool = a "bool" as BoolKeyword,
        /// Unsigned integer type (represented as `usize` (`size_t`))
        Uint = a "uint" as UintKeyword,
        /// Signed integer type (represented as `isize` (`isize_t`))
        Sint = a "sint" as SintKeyword,
        /// Floating point type (represented as `f64`)
        Frac = a "frac" as FracKeyword,
        /// Character type
        Char = a "char" as CharKeyword,
        /// String type
        Text = a "text" as TextKeyword,

        // ----------------------------
        // Definitions
        // ----------------------------

        /// Define a record structure.
        /// ### Syntax
        /// ```rs
        /// rec /* name */ {
        ///     // fields
        /// }
        /// ```
        Rec = a "rec" as RecKeyword,
        /// Define a superset type. A superset type can be any one of the types it is defined with.
        /// ### Syntax
        /// ```rs
        /// sup /* name */ = /* type 1 */ | /* type 2 */ | /* ... */ | /* type n */;
        /// ```
        /// **Example:**
        /// ```rs
        /// sup MaybeNumber = uint | none;
        /// ```
        Sup = a "sup" as SupKeyword,
        /// Define a categorical type. Categorical types are related, disjoint constants that can
        /// be used as literal-subset type with named variants.
        /// ### Syntax
        /// ```rs
        /// cat /* name */ {
        ///     // variants
        /// }
        /// ```
        /// **Example:**
        /// ```rs
        /// cat Foo {
        ///     Apple,
        ///     Orange,
        ///     Banana,
        ///     Mango,
        /// }
        /// ```
        /// A categorical type can have any type as its discriminant, as long as each variant has
        /// the same type and supports inequality.
        /// **Example:**
        /// ```rs
        /// cat Foo {
        ///     Apple = "red",
        ///     Orange = "orange",
        ///     Banana = "yellow",
        ///     Mango = "gold",
        /// }
        /// ```
        Cat = a "cat" as CatKeyword,
        /// Define a type alternative (alias). A type alias is identical to the existing type,
        /// but with a new name.
        /// ### Syntax
        /// ```rs
        /// alt /* alias */ = /* type */;
        /// ```
        Alt = a "alt" as AltKeyword,
        /// Define a subset type. A subset type can fit into any slot where its original type fits,
        /// but its original type cannot fit into a subset type slot without proving the value fits.
        /// Use `only` to give a whitelist, `xcpt` to give a blacklist, or `where` to provide a
        /// attern applied to each item. If no item satisfies the `where` clause, the subset type
        /// will be incidentally equivalent (not through enforcement) to `nevr`/`none`.
        /// ### Syntax
        /// ```rs
        /// sub /* name */ of /* cat/union */ only {
        ///     /* items */
        /// }
        /// // or
        /// sub /* name */ of /* cat/union */ xcpt {
        ///     /* items */
        /// }
        /// // or
        /// sub /* name */ of /* cat/union */ where /* requirements */;
        /// ```
        /// **Example:**
        /// ```rs
        /// cat Fruit {
        ///     Apple,
        ///     Orange,
        ///     Banana,
        ///     Mango,
        /// }
        /// sub YellowFruit of Fruit {
        ///     Banana,
        ///     Mango,
        /// }
        /// ```
        Sub = a "sub" as SubKeyword,
        /// Define a macro.
        /// ### Syntax
        /// ```rs
        /// def \/* name */($/* param 1 */, $/* param 2 */, /* ... */, $/* param n */) {
        ///     // definition
        /// }
        /// ```
        Def = a "def" as DefKeyword,
        /// Define a function.
        /// ### Syntax
        /// ```rs
        /// fn /* name */(/* param 1 */, /* param 2 */, /* ... */, /* param n */) -> /* return type */ {
        ///     // definition
        /// }
        /// ```
        Fn = a "fn" as FnKeyword,
        /// Define member items of a `rec`/`union`/`cat`.
        /// ### Syntax
        /// ```rs
        /// mem /* rec/union/cat */ {
        ///     // types
        ///     // constants
        ///     // methods
        /// }
        /// ```
        Mem = an "mem" as MemKeyword,

        // ----------------------------
        // Value
        // ----------------------------

        /// Create a local variable.
        /// ### Syntax
        /// ```rs
        /// let /* name */;
        /// // or
        /// let /* name */ = /* initial value */;
        /// ```
        Let = a "let" as LetKeyword,
        /// Create a universal variable.
        /// ### Syntax
        /// ```rs
        /// uni /* name */;
        /// // or
        /// uni /* name */ = /* initial value */;
        /// ```
        Uni = a "uni" as UniKeyword,
        /// Create a pivot (constant) value.
        /// ### Syntax
        /// ```rs
        /// pvt /* name */ = /* constant value */;
        /// ```
        Pvt = a "pvt" as PvtKeyword,

        // ----------------------------
        // Interface
        // ----------------------------

        /// ## In function definition
        /// Supplies requirements for function parameters.
        /// ### Syntax
        /// ```rs
        /// fn foo(v, fun) -> text
        /// where
        ///     v.x: frac,
        ///     v.y: frac,
        ///     fn mag of v: (self) -> frac,
        ///     fn fun: (frac) -> text,
        /// {
        ///     // ...
        /// }
        /// ```
        /// When a `where` clause is present, any errors that might have been emitted at
        /// the function definition but have been specified in the `where` clause, will
        /// instead be attributed to the caller.
        /// **Example:**
        /// ```rs
        /// fn foo(v) {
        ///     return v.x // ERROR: parameter `v` is not guaranteed to have a field `x`;
        ///                // try adding a `where` clause or prove `v` has such a field
        /// }
        /// fn bar(v)
        /// where
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
        /// for /* binding */ in /* iterable */ where /* condition */ {
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
        Where = a "where" as WhereKeyword,
        /// Used in a `where` clause to specify that a parameter must possess some field/method,
        /// without specifying its format.
        /// ### Syntax
        /// ```rs
        /// fn foo(v)
        /// where
        ///     v has x, // `v.x` is defined
        ///     v has y, // `v.y` is defined
        /// {
        ///     // ...
        /// }
        /// ```
        /// Type restraints can be added to the member by following it with a colon
        /// **Example:**
        /// ```rs
        /// fn foo(v)
        /// where
        ///     v has x: uint, // `v.x` is defined as a uint
        ///     v has fn f: (self) -> frac, // `v.y` is defined as a frac-returning method
        /// {
        ///     // ...
        /// }
        /// ```
        Has = a "has" as HasKeyword,

        // ----------------------------
        // Flow
        // ----------------------------

        // Conditional
        /// Only perform the statement if the condition holds.
        /// ### Syntax
        /// ```rs
        /// if /* condition */ {
        ///     // statement
        /// }
        /// ```
        /// #### Equivalent to
        /// ```mips
        ///     b after # !condition
        ///     # statement
        /// after:
        /// ```
        If = an "if" as IfKeyword,
        /// When following an `if` statement, only performs the statement if the condition does not hold.
        /// ### Syntax
        /// ```rs
        /// if /* ... */ {
        ///     // ...
        /// } or {
        ///     // statement
        /// }
        /// ```
        /// #### Equivalent to
        /// ```mips
        ///     b else # !condition
        ///     # ...
        ///     j after
        /// else:
        ///     # statement
        /// after:
        /// ```
        /// Can be followed by an `if` to add an additional condition.
        /// ```rs
        /// if /* ... */ {
        ///     // ...
        /// } or if /* extra condition */ {
        ///     // statement
        /// }
        /// ```
        Or = an "or" as OrKeyword,
        /// Choose a branch based on pattern.
        /// ### Syntax
        /// ```rs
        /// match /* expression */ {
        ///     /* pattern */ => /* statement or expression */,
        ///     // ...
        /// }
        /// ```
        Match = a "match" as MatchKeyword,

        // ----------------------------
        // Loop
        // ----------------------------

        /// Repeat as long as a condition is true.
        /// ### Syntax
        /// ```rs
        /// rep /* condition */ {
        ///     // statement
        /// }
        /// ```
        /// #### Equivalent to
        /// ```mips
        /// loop:
        ///     b after # !condition
        ///     # statement
        ///     j loop
        /// after:
        /// ```
        Rep = a "rep" as RepKeyword,
        /// Repeat for each item in an iterator.
        /// ### Syntax
        /// ```rs
        /// for /* binding */ in /* iterable */ {
        ///     // statement
        /// }
        /// ```
        /// #### Equivalent to
        /// ```rs
        /// let __iter = /* iterable */
        /// let __item = __iter.next();
        /// rep __item.is_some() {
        ///     let /* binding */ = __item;
        ///     // statement
        ///     __item = __iter.next();
        /// }
        /// ```
        For = a "for" as ForKeyword,
        /// Separates the binding from the iterator in a for loop.
        /// ### Syntax
        /// ```rs
        /// for /* binding */ in /* iterable */ {
        ///     // ...
        /// }
        /// ```
        In = an "in" as InKeyword,
        /// Repeat forever (or until a `stop`/`give`/`fail`).
        /// ### Syntax
        /// ```rs
        /// loop {
        ///     // statement
        /// }
        /// ```
        /// #### Equivalent to
        /// ```rs
        /// rep true {
        ///     // statement
        /// }
        /// ```
        /// or
        /// ```mips
        /// loop:
        ///     # statement
        ///     j loop
        /// ```
        Loop = a "loop" as LoopKeyword,

        /// A conditionless, single-iteration loop that can be "early-returned" from (using `stop`)
        /// without exiting the function. Saves from having to make a new function that would only
        /// be used in one place, just for the sake of returning if there's an error. Named after a
        /// "bungee cord" or lifeline.
        /// ### Syntax
        /// ```rs
        /// cord {
        ///     // statement
        /// }
        /// ```
        /// #### Equivalent to
        /// ```rs
        /// rep true {
        ///     // statement
        ///     stop;
        /// }
        /// ```
        /// or
        /// ```mips
        ///     # statement
        /// after:
        /// ```
        /// ```mips
        ///     # statement
        ///     b after # !condition
        ///     # statement
        ///     b after # !condition
        ///     # statement
        /// after:
        /// ```
        Cord = a "cord" as CordKeyword,

        // Loop control
        /// Quit the loop.
        /// ### Syntax
        /// ```rs
        /// /* for/rep/loop/cord */ {
        ///     if /* condition */ { halt; }
        /// }
        /// ```
        Halt = a "halt" as HaltKeyword,
        /// Stop the current loop and skip to the next iteration.
        /// ### Syntax
        /// ```rs
        /// /* for/rep/loop */ {
        ///     if /* condition */ { skip; }
        /// }
        /// ```
        Skip = a "skip" as SkipKeyword,

        // ----------------------------
        // Exit
        // ----------------------------

        /// End the function and output the value.
        /// ### Syntax
        /// ```rs
        /// give /* value */;
        /// ```
        Give = a "give" as GiveKeyword,
        /// ## As a type
        /// Builtin error type produced by `fail`
        /// ## As a keyword
        /// Return with a failure, like an exception. Accessing the return of a `fail`ed function
        /// will immediately `fail` the accessing function, unless handled with `match`ed.
        /// ### Syntax
        /// ```rs
        /// fail /* error */;
        /// ```
        Fail = a "fail" as FailKeyword,
        /// Return the value within a loop without ending the function, to allow for iterable functions.
        /// Turns the function into a mutable closure.
        /// ### Syntax
        /// ```rs
        /// emit /* value */;
        /// ```
        /// `emit` can be combined with `fail` to indicate an error that only impacts the current item.
        /// ```rs
        /// emit fail /* error */;
        /// ```
        Emit = an "emit" as EmitKeyword,
    }
}

impl Keyword {
    /// Test if a keyword is a "flow" keyword
    pub const fn is_flow(self) -> bool {
        matches!(
            self,
            Self::If
                | Self::Or
                | Self::Match
                | Self::Rep
                | Self::For
                | Self::In
                | Self::Loop
                | Self::Cord
                | Self::Halt
                | Self::Skip
                | Self::Give
                | Self::Fail
                | Self::Emit
        )
    }

    /// Test if a keyword is a language defined type
    pub const fn is_type(self) -> bool {
        matches!(
            self,
            Self::None
                | Self::Nevr
                | Self::Bool
                | Self::Uint
                | Self::Sint
                | Self::Frac
                | Self::Char
                | Self::Text
        )
    }
}

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
        // 3-char
        // ----------------------------

        /// Exponent assign - Equivalent to `lhs = lhs ** rhs`
        ExponentAssign = an "**=" ("exponent assign") as ExponentAssignOp,
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
        Exponent = an "**" ("exponent") as ExponentOp,
        /// Add assign - Equivalent to `lhs = lhs + rhs`
        AddAssign = an "+=" ("add assign") as AddAssignOp,
        /// Sub assign - Equivalent to `lhs = lhs - rhs`
        SubAssign = a "-=" ("subtract assign") as SubAssignOp,
        /// Arrow - Separate a function's parameter list from its return type
        Arrow = an "->" ("arrow") as ArrowOp,
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
    }
}

/// Produces a `IntErrorKind::NegOverflow`, since those are private :/
fn number_underflow() -> std::num::TryFromIntError {
    {
        #[allow(clippy::as_conversions, reason = "into is not const")]
        #[expect(clippy::invalid_upcast_comparisons, reason = "further proves my point")]
        const {
            assert!(i16::MIN < i8::MIN as i16, "proof. i16::MIN < i8::MIN");
        }
        // SAFETY: `i16::MIN < i8::MIN`. Because it is `<` and not `<=`, and both are integers,
        // there must be a difference of at least 1.
        i8::try_from(unsafe { i16::from(i8::MIN).unchecked_sub(1) })
            .expect_err("should result in negative overflow")
    }
}

/// Information about a character literal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CharLiteral {
    /// The character being represented
    pub ch: char,

    /// Whether the character is an escape sequence in the lexeme
    pub is_escaped: bool,
}

/// Information about a string literal
///
/// Allocating version of [`StrLiteral`]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StringLiteral<'a> {
    /// The string literal doesn't have any escape sequences
    NoEscapes {
        /// No escape sequences are present
        text: &'a str,
    },
    /// The string literal has at least one escape sequence
    HasEscaped {
        /// The text content of the string literal; escape sequences converted and delimiters excluded.
        text: String,

        /// Ranges of the original lexeme (quote delimiters excluded) that refer to escape sequences
        escapes: Vec<Range<usize>>,
    },
}

impl Default for StringLiteral<'_> {
    fn default() -> Self {
        Self::NoEscapes {
            text: Default::default(),
        }
    }
}

/// No-alloc version of [`StringLiteral`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StrLiteral<'a> {
    /// The string literal without delimiters - the value if it has no escapes.
    /// May contain unconverted escape sequences
    pub src: &'a str,
}

impl StrLiteral<'_> {
    /// Identify whether a string literal contains escape sequences.
    pub fn has_escapes(&self) -> bool {
        self.src.contains(ESCAPE)
    }
}

impl<'a> TryFrom<StrLiteral<'a>> for StringLiteral<'a> {
    type Error = ErrorType<'a>;

    fn try_from(value: StrLiteral<'a>) -> Result<Self, Self::Error> {
        if value.has_escapes() {
            let replacements = Escapes::new(value.src).collect::<Result<Vec<_>, _>>()?;
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
                value
                    .src
                    .len()
                    .checked_sub(byte_diff)
                    .expect("should only be removing bytes, not adding"),
            );
            let mut prev_end = 0;
            for (range, repl) in replacements {
                processed.push_str(value.src.get(prev_end..range.start)
                    .expect("range should never start/end within a UTF-8 character, and prev_end should always be from such a range (or 0)"));
                processed.push(repl);
                prev_end = range.end;
            }

            Ok(StringLiteral::HasEscaped {
                text: processed,
                escapes,
            })
        } else {
            Ok(StringLiteral::NoEscapes { text: value.src })
        }
    }
}

/// The value represented by a [`Token`]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TokenValue<'a> {
    // ignored
    /// An entire chunk of whitespace, not just one character
    #[default]
    Whitespace,
    /// A comment (either block or line)
    Comment,

    // number literal
    /// Unsigned integer literal
    UIntLiteral(usize),
    /// Signed integer literal
    SIntLiteral(isize),
    /// Floating point literal
    FltLiteral(f64),

    // strlike literal
    /// Character literal
    CharLiteral(CharLiteral),
    /// String literal
    StringLiteral(StrLiteral<'a>),

    // language builtin
    /// Boolean literal
    BoolLiteral(bool),

    // identifiers
    /// An identifier - its value is the lexeme itself
    Identifier,
    /// An identifier followed by `(` or following a `fn` keyword
    #[deprecated(note = "will be determined by parser in future versions")]
    Callable,
    /// An identifier prefixed with `\`
    Macro,
    /// An identifier prefixed with `$`
    MacroParam,

    /// A language keyword
    Keyword(Keyword),
    /// Operators and other non-alphanumeric tokens
    Punctuation(Punctuation),
}

impl<'a> TokenValue<'a> {
    /// Parses a number literal lexeme into its value
    pub fn number_literal(src: &'a str) -> Result<Self, ErrorType<'a>> {
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
                                    ErrorType::InvalidNumLiteral(NumLitError::SInt(
                                        number_underflow(),
                                    ))
                                })
                            }))
                        .map(Self::SIntLiteral)
                    } else {
                        Ok(Self::UIntLiteral(value))
                    }
                })
        }
    }

    /// Parses a character literal lexeme into its value
    pub fn char_literal(src: &'a str) -> Result<Self, ErrorType<'a>> {
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

impl<'a> TokenValue<'a> {
    /// Parses a string literal lexeme into its value, without allocating
    pub fn string_literal(src: &'a str) -> Result<Self, ErrorType<'a>> {
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
            Ok(Self::StringLiteral(StrLiteral { src }))
        }
    }
}

/// An iterator over unescaped escape characters ([`ESCAPE`]) in a string
#[derive(Debug, Clone)]
pub struct Escapes<'a> {
    /// Source code
    source: &'a str,
    /// Tracking of offset into [`Self::source`]
    offset: usize,
    /// Whether the upcoming character is escaped
    is_esc: bool,
}

impl<'a> Escapes<'a> {
    /// Construct a new [`Escapes`] iterator from a source string
    #[must_use]
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
        self.source
            .get(self.offset..)
            .expect("offset should never be within a UTF-8 character")
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

/// A single token - its lexeme ([`Self::src`]) and type ([`Self::ty`]).
/// Does not contain the token's value, but can have the value obtained with [`Self::value_noalloc`].
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Token<'a> {
    /// Because this is a pointer into the original source string, we can use pointer arithmetic to find its location.
    /// If a program has a thousand tokens, why allocate a new string and store two additional integers in case of error
    /// when we can just keep the original string around and calculate those integers *on demand*?
    pub lex: &'a str,

    /// The value of the token
    pub val: TokenValue<'a>,
}

impl std::fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { lex, val } = self;
        let (name, val): (&str, Option<&dyn std::fmt::Debug>) = match val {
            TokenValue::Whitespace => ("Whitespace", None),
            TokenValue::Comment => ("Comment", None),
            TokenValue::UIntLiteral(x) => ("UIntLiteral", Some(x)),
            TokenValue::SIntLiteral(x) => ("SIntLiteral", Some(x)),
            TokenValue::FltLiteral(x) => ("FltLiteral", Some(x)),
            TokenValue::CharLiteral(x) => ("CharLiteral", Some(x)),
            TokenValue::StringLiteral(x) => ("StringLiteral", Some(x)),
            TokenValue::BoolLiteral(x) => ("BoolLiteral", Some(x)),
            TokenValue::Identifier | TokenValue::Callable => ("Identifier", None),
            TokenValue::Macro => ("Macro", None),
            TokenValue::MacroParam => ("MacroParam", None),
            TokenValue::Keyword(x) => ("Keyword", Some(x)),
            TokenValue::Punctuation(x) => ("Punctuation", Some(x)),
        };
        write!(f, "{name}({lex:?})")?;
        if let Some(val) = val {
            write!(f, ": {val:?}")
        } else {
            Ok(())
        }
    }
}

/// Returns [`None`] if `src` does not start with `\`
///
/// # Panics
/// This method can panic if its internal assumptions prove false
#[must_use]
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
            // SAFETY: As shown above, `char::MAX_LEN_UTF8 * 2` fits in usize.
            // By definition of `char::MAX_LEN_UTF8`, `c.len_utf8()` is at most `char::MAX_LEN_UTF8` for all `c: char`.
            // Therefore, `c.len_utf8() + d.len_utf8()` fits in usize for all `c,d: char`.
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
    let esc_rest = src
        .get(i..)
        .expect("`i` should not be within a UTF-8 character");
    escape_char(esc_rest)
        .ok_or(ErrorType::InvalidEscape(esc_rest)) // no remaining characters
        .and_then(|(len, res)| {
            let esc = esc_rest.get(..len).expect(
                "return of escape_char, starting at `i`, should not be within a UTF-8 character",
            );
            let range = Range::from(
                i..i.checked_add(len)
                    .expect("should be at most the length of a string already in memory"),
            );
            res.map(|ch| (range, ch))
                .map_err(|()| ErrorType::InvalidEscape(esc))
        })
}
