# Batscript

## Keywords

### Builtins

#### Values

- `self` - The value a method is being called on
- `true` - Boolean true
- `fals` - Boolean false
- `none` - Absence of value
- `nevr` - Unreachable (crash if accessed)

#### Types

- `none` - Absence of a result (return type of an empty-bodied function)
- `nevr` - Unreachable (return type of `stop`less `loop {}`)
- `bool` - Boolean type
- `uint` - Unsigned integer type (represented as `usize` (`size_t`))
- `sint` - Signed integer type (represented as `isize` (`isize_t`))
- `frac` - Floating point type (represented as `f64`)
- `char` - Character type
- `text` - String type
- `fail` - Builtin error type produced by `fail`

### Definitions

- `rec` - Define a record structure.

    ```rs
    rec /* name */ {
        // fields
    }
    ```

- `sup` - Define a superset type. A superset type can be any one of the types it is defined with.

    ```rs
    sup /* name */ = /* type 1 */ | /* type 2 */ | /* ... */ | /* type n */;
    ```

    Example:

    ```rs
    sup MaybeNumber = uint | none;
    ```

- `cat` - Define a categorical type. Categorical types are related, disjoint constants that can be used as literal-subset type with named variants.

    ```rs
    cat /* name */ {
        // variants
    }
    ```

    Example:

    ```rs
    cat Foo {
        Apple,
        Orange,
        Banana,
        Mango,
    }
    ```

    A categorical type can have any type as its discriminant, as long as each variant has the same type and supports inequality.

    ```rs
    cat Foo {
        Apple = "red",
        Orange = "orange",
        Banana = "yellow",
        Mango = "gold",
    }
    ```

- `alt` - Define a type alternative (alias). A type alias is identical to the existing type, but with a new name.

    ```rs
    alt /* alias */ = /* type */;
    ```

- `sub` - Define a subset type. A subset type can fit into any slot where its original type fits, but its original type cannot fit into a subset type slot without proving the value fits. Use `only` to give a whitelist, `xcpt` to give a blacklist, or `where` to provide a pattern applied to each item. If no item satisfies the `where` clause, the subset type will be incidentally equivalent (not through enforcement) to `nevr`/`none`.

    ```rs
    sub /* name */ of /* cat/union */ only {
        /* items */
    }
    // or
    sub /* name */ of /* cat/union */ xcpt {
        /* items */
    }
    // or
    sub /* name */ of /* cat/union */ where /* requirements */;
    ```

    Example:

    ```rs
    cat Fruit {
        Apple,
        Orange,
        Banana,
        Mango,
    }

    sub YellowFruit of Fruit {
        Banana,
        Mango,
    }
    ```

- `def` - Define a macro.

    ```rs
    def \/* name */($/* param 1 */, $/* param 2 */, /* ... */, $/* param n */) {
        // definition
    }
    ```

- `fn` - Define a function.

    ```rs
    fn /* name */(/* param 1 */, /* param 2 */, /* ... */, /* param n */) -> /* return type */ {
        // definition
    }
    ```

- `mem` - Define member items of a `rec`/`union`/`cat`.

    ```rs
    mem /* rec/union/cat */ {
        // types
        // constants
        // methods
    }
    ```

### Value

- `let` - Create a local variable.

    ```rs
    let /* name */;
    // or
    let /* name */ = /* initial value */;
    ```

- `uni` - Create a universal variable.

    ```rs
    uni /* name */;
    // or
    uni /* name */ = /* initial value */;
    ```

- `pvt` - Create a pivot (constant) value.

    ```rs
    pvt /* name */ = /* constant value */;
    ```

### Interface

- `where` - Supplies requirements for function parameters.

    ```rs
    fn foo(v, fun) -> text
    where
        v.x: frac,
        v.y: frac,
        fn mag of v: (self) -> frac,
        fn fun: (frac) -> text,
    {
        // ...
    }
    ```

    When a `where` clause is present, any errors that might have been emitted at the function definition but have been specified in the `where` clause, will instead be attributed to the caller.

    ```rs
    fn foo(v) {
        return v.x // ERROR: parameter `v` is not guaranteed to have a field `x`; try adding a `where` clause or prove `v` has such a field
    }

    fn bar(v)
    where
        v has x, // INFO: requirement introduced here
    {
        return v.x
    }

    fn main() {
        foo(5);
        bar(5); // ERROR: argument `v` of `bar` is expected to have a field `x`, but `5` (uint) has no such field
    }
    ```

- `has` - Used in a `where` clause to specify that a parameter must possess some field/method, without specifying its format.

    ```rs
    fn foo(v)
    where
        v has x, // `v.x` is defined
        v has y, // `v.y` is defined
    {
        // ...
    }
    ```

    Type restraints can be added to the member by following it with a colon

    ```rs
    fn foo(v)
    where
        v has x: uint, // `v.x` is defined as a uint
        v has fn f: (self) -> frac, // `v.y` is defined as a frac-returning method
    {
        // ...
    }
    ```

### Flow Control

#### Conditional

- `if` - Only perform the statement if the condition holds.

    ```rs
    if /* condition */ {
        // statement
    }
    ```

    Equivalent to

    ```mips
        b after # !condition
        # statement
    after:
    ```

- `or` - When following an `if` statement, only performs the statement if the condition does not hold.

    ```rs
    if /* ... */ {
        // ...
    } or {
        // statement
    }
    ```

    Equivalent to

    ```mips
        b else # !condition
        # ...
        j after
    else:
        # statement
    after:
    ```

    Can be followed by an `if` to add an additional condition.

    ```rs
    if /* ... */ {
        // ...
    } or if /* extra condition */ {
        // statement
    }
    ```

- `match` - Choose a branch based on pattern.

    ```rs
    match /* expression */ {
        /* pattern */ => /* statement or expression */,
        // ...
    }
    ```

#### Loop

- `rep` Repeat as long as a condition is true.

    ```rs
    rep /* condition */ {
        // statement
    }
    ```

    Equivalent to

    ```mips
    loop:
        b after # !condition
        # statement
        j loop
    after:
    ```

- `for` Repeat for each item in an iterator.

    ```rs
    for /* binding */ in /* iterable */ {
        // statement
    }
    ```

    Equivalent to

    ```rs
    let __iter = /* iterable */
    let __item = __iter.next();
    rep __item.is_some() {
        let /* binding */ = __item;
        // statement
        __item = __iter.next();
    }
    ```

- `in` - Separates the binding from the iterator in a for loop.

    ```rs
    for /* binding */ in /* iterable */ {
        // ...
    }
    ```

- `where` - Filters an iterator.

    ```rs
    for /* binding */ in /* iterable */ where /* condition */ {
        // statement
    }
    ```

    Equivalent to

    ```rs
    for /* binding */ in /* iterable */ {
        if /* condition */ {
            skip;
        }
        // statement
    }
    ```

- `loop` Repeat forever (or until a `stop`/`give`/`fail`).

    ```rs
    loop {
        // statement
    }
    ```

    Equivalent to

    ```rs
    rep true {
        // statement
    }
    ```

    or

    ```mips
    loop:
        # statement
        j loop
    ```

- `cord` - A conditionless, single-iteration loop that can be "early-returned" from (using `stop`) without exiting the function. Saves from having to make a new function that would only be used in one place, just for the sake of returning if there's an error. Named after a "bungee cord" or lifeline.

    ```rs
    cord {
        // statement
    }
    ```

    Equivalent to

    ```rs
    rep true {
        // statement
        stop;
    }
    ```

    or

    ```mips
        # statement
    after:
    ```

    ```mips
        # statement
        b after # !condition
        # statement
        b after # !condition
        # statement
    after:
    ```

### Loop Control

- `halt` - Quit the loop.

    ```rs
    /* for/rep/loop/cord */ {
        if /* condition */ { halt; }
    }
    ```

- `skip` - Stop the current loop and skip to the next iteration.

    ```rs
    /* for/rep/loop */ {
        if /* condition */ { skip; }
    }
    ```

#### Exit

- `give` - End the function and output the value.

    ```rs
    give /* value */;
    ```

- `fail` - Return with a failure, like an exception. Accessing the return of a `fail`ed function will immediately `fail` the accessing function, unless handled with `match`ed.

    ```rs
    fail /* error */;
    ```

- `emit` - Return the value within a loop without ending the function, to allow for iterable functions. Turns the function into a mutable closure.

    ```rs
    emit /* value */;
    ```

    `emit` can be combined with `fail` to indicate an error that only impacts the current item.

    ```rs
    emit fail /* error */;
    ```

## Punctuation

- `**=`: Exponent assign - Equivalent to `lhs = lhs ** rhs`
- `<<=`: Bitshift left assign - Equivalent to `lhs = lhs << rhs`
- `>>=`: Bitshift right assign - Equivalent to `lhs = lhs >> rhs`
- `!&=`: Nand assign - Equivalent to `lhs = lhs !& rhs`
- `!|=`: Nor assign - Equivalent to `lhs = lhs !| rhs`
- `!^=`: Xnor assign - Equivalent to `lhs = lhs !^ rhs`
- `!=`: Not equal - Equivalent to `!(lhs == rhs)`
- `!&`: Nand - Equivalent to `!(lhs & rhs)`
- `!|`: Nor - Equivalent to `!(lhs | rhs)`
- `!^`: Xnor - Equivalent to `!(lhs ^ rhs)`
- `##`: Concatenate - Combine macro arguments without whitespace (possibly forming new tokens)
- `%=`: Remainder assign - Equivalent to `lhs = lhs % rhs`
- `&=`: And assign - Equivalent to `lhs = lhs & rhs`
- `*=`: Multiply assign - Equivalent to `lhs = lhs * rhs`
- `**`: Exponent - Put `lhs` to the power of `rhs`
- `+=`: Add assign - Equivalent to `lhs = lhs + rhs`
- `-=`: Sub assign - Equivalent to `lhs = lhs - rhs`
- `->`: Arrow - Separate a function's parameter list from its return type
- `/=`: Divide assign - Equivalent to `lhs = lhs / rhs`
- `::`: Path separator - Separate namespace path items
- `:=`: Colon assign - Assign definition
- `<=`: Less or equal - Equivalent to `lhs < rhs | lhs == rhs`
- `<<`: Bitshift left - Shift the bits in `lhs` to the left (away from 0) by `rhs` bits
- `==`: Equal - Test equality between `lhs` and `rhs`
- `=>`: FatArrow - Separates `match` arm conditions from statements
- `>=`: Greater or equal - Equivalent to `lhs < rhs | lhs == rhs`
- `>>`: Shr - Shift the bits in `lhs` to the right (towards 0) by `rhs` bits
- `^=`: XorAssign - Equivalent to `lhs = lhs ^ rhs`
- `|=`: OrAssign - Equivalent to `lhs = lhs | rhs`
- `!`: Not - Logical negation (booleans) or bitflip (integers)
- `#`: Stringify - Replace tokens with their lexemes in a macro
- `%`: Remainder - Find the remainder of `lhs / rhs`
- `&`: And - Logical AND (booleans) or bitwise AND (integers)
- `(`: Left parenthesis
- `)`: Right parenthesis
- `*`: Multiply - Find the product of `lhs` and `rhs`
- `+`: Add - Find the sum of `lhs` and `rhs
- `,`: Comma - Separate items in a list
- `-`: Subtract - Find the difference of `lhs - rhs`
- `.`: Dot - Access a rec member
- `/`: Divide - Find the quotient of `lhs / rhs`
- `:`: Colon - Separate a variable/field/parameter from its type or requirements
- `;`: Semicolon - Conclude a statement
- `<`: Less than - Test if `lhs` is strictly lower value compared to `rhs`
- `=`: Assign - Assign `rhs` to `lhs`
- `>`: Greater than - Test if `lhs` is strictly higher value compared to `rhs`
- `?`: Question mark - TBD
- `@`: Reference - Create a pointer/reference to a value (like to `&` in other languages)
- `[`: Left bracket
- `]`: Right bracket
- `^`: Xor - Logical XOR (booleans) or bitwise XOR (integers)
- `{`: Left brace
- `|`: Or - Logical OR (booleans) or bitwise OR (integers)
- `}`: Right brace
