//! Keyword tokens

define_token_eq! {
    /// Language-defined reserved words for defining behavior or form
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Keyword = keyword {
        // ----------------------------
        // Builtin values
        // ----------------------------

        /// The value a method is being called on
        SelfKw = "self",

        // ----------------------------
        // Builtin types
        // ----------------------------

        /// ## As a value
        /// Absence of value
        /// ## As a type
        /// Absence of a result (return type of an empty-bodied function)
        None = "none",
        /// ## As a value
        /// Unreachable (crash if accessed)
        /// ## As a type
        /// Unreachable (return type of `stop`less `loop {}`)
        Nevr = "nevr",
        /// Boolean type
        Bool = "bool",
        /// Unsigned integer type (represented as `usize` (`size_t`))
        Uint = "uint",
        /// Signed integer type (represented as `isize` (`isize_t`))
        Sint = "sint",
        /// Floating point type (represented as `f64`)
        Frac = "frac",
        /// Character type
        Char = "char",
        /// String type
        Text = "text",

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
        Rec = "rec",
        /// Define a superset type. A superset type can be any one of the types it is defined with.
        /// ### Syntax
        /// ```rs
        /// sup /* name */ = /* type 1 */ | /* type 2 */ | /* ... */ | /* type n */;
        /// ```
        /// **Example:**
        /// ```rs
        /// sup MaybeNumber = uint | none;
        /// ```
        Sup = "sup",
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
        Cat = "cat",
        /// Define a type alternative (alias). A type alias is identical to the existing type,
        /// but with a new name.
        /// ### Syntax
        /// ```rs
        /// alt /* alias */ = /* type */;
        /// ```
        Alt = "alt",
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
        Sub = "sub",
        /// Define a macro.
        /// ### Syntax
        /// ```rs
        /// def \/* name */($/* param 1 */, $/* param 2 */, /* ... */, $/* param n */) {
        ///     // definition
        /// }
        /// ```
        Def = "def",
        /// Define a function.
        /// ### Syntax
        /// ```rs
        /// fn /* name */(/* param 1 */, /* param 2 */, /* ... */, /* param n */) -> /* return type */ {
        ///     // definition
        /// }
        /// ```
        Fn = "fn",
        /// Define member items of a `rec`/`union`/`cat`.
        /// ### Syntax
        /// ```rs
        /// mem /* rec/union/cat */ {
        ///     // types
        ///     // constants
        ///     // methods
        /// }
        /// ```
        Mem = "mem",

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
        Let = "let",
        /// Create a universal variable.
        /// ### Syntax
        /// ```rs
        /// uni /* name */;
        /// // or
        /// uni /* name */ = /* initial value */;
        /// ```
        Uni = "uni",
        /// Create a pivot (constant) value.
        /// ### Syntax
        /// ```rs
        /// pvt /* name */ = /* constant value */;
        /// ```
        Pvt = "pvt",

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
        Where = "where",
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
        Has = "has",

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
        If = "if",
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
        Or = "or",
        /// Choose a branch based on pattern.
        /// ### Syntax
        /// ```rs
        /// match /* expression */ {
        ///     /* pattern */ => /* statement or expression */,
        ///     // ...
        /// }
        /// ```
        Match = "match",

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
        Rep = "rep",
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
        For = "for",
        /// Separates the binding from the iterator in a for loop.
        /// ### Syntax
        /// ```rs
        /// for /* binding */ in /* iterable */ {
        ///     // ...
        /// }
        /// ```
        In = "in",
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
        Loop = "loop",

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
        Cord = "cord",

        // Loop control
        /// Quit the loop.
        /// ### Syntax
        /// ```rs
        /// /* for/rep/loop/cord */ {
        ///     if /* condition */ { halt; }
        /// }
        /// ```
        Halt = "halt",
        /// Stop the current loop and skip to the next iteration.
        /// ### Syntax
        /// ```rs
        /// /* for/rep/loop */ {
        ///     if /* condition */ { skip; }
        /// }
        /// ```
        Skip = "skip",

        // ----------------------------
        // Exit
        // ----------------------------

        /// End the function and output the value.
        /// ### Syntax
        /// ```rs
        /// give /* value */;
        /// ```
        Give = "give",
        /// ## As a type
        /// Builtin error type produced by `fail`
        /// ## As a keyword
        /// Return with a failure, like an exception. Accessing the return of a `fail`ed function
        /// will immediately `fail` the accessing function, unless handled with `match`ed.
        /// ### Syntax
        /// ```rs
        /// fail /* error */;
        /// ```
        Fail = "fail",
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
        Emit = "emit",
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

    /// Test if a keyword is a language defined value
    pub const fn is_value(self) -> bool {
        matches!(self, Self::None | Self::Nevr)
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
                | Self::Fail
        )
    }
}
