# Batscript

**Note:** Tests are in [src/test](src/test).

## Syntax

### Number literals

```re
-?[0-9][0-9a-zA-Z]*(?:\.[0-9a-zA-Z]+)?(?:[eE]\-?[0-9a-zA-Z]+)?
```

**Warning:** This is only a rough translation. The actual scanner I wrote does not use regex.

A hyphen (`-`) is a subtraction operator instead of a negative sign **unless** the most recent non-whitespace, non-comment token is punctuation **and not** a close-bracket (`)`, `]`, `}`).

### String literals

This one is nearly impossible for me to accurately represent with regex. Instead I will represent it roughly with python.

```py
def string_literal(source):
    """
    Checks if the very first token in `source` is a string literal
    and returns the string literal (delimiters (`"`) included) if it is.
    Escape sequences are not evaluated by the scanner.
    """
    if source[0] == '"':
        is_escaped = False
        count = 1 # not 0 because the opening '"' is included
        for ch in source[1:]:
            # the current character will always be included,
            # even if it ends the string literal
            count += 1

            # unescaped double-quote (`"`)
            if not is_escaped and ch == '"':
                return source[0:count]

            # next character is escaped if the current character is an
            # UNESCAPED backslash (`\`)
            is_escaped = not is_escaped and ch == '\\'
        # reached EOF without finding an unescaped double-quote (`"`)
        raise Exception("string literal is missing a closing double-quote (`\"`)")
    else:
        return None # not a string literal
```

If a token starts with a double-quote (`"`), it is a string literal without exception.
Everthing following that character, through the next **unescaped** double-quote (a double-quote (`"`) **not preceded** by an **odd number** of backslashes (`\`)), is part of the string literal token.

Supports escape sequences.

### Character literal

Same as string literals, but substitutes single quotes (`'`) in place of double quotes (`"`) and produces an error if 0 or multiple codepoints are present in the value (not the lexeme) of the token.

### Identifiers

```re
[\p{letter}_][\p{letter}\p{number}'_]*
```

Identifiers must start with a letter (not restricted to ASCII) or underscore (`_`). The rest of the characters in the token can be letters (not restricted to ASCII), numbers (not restricted to ASCII), apostrophes (`'`), or underscores (`_`).

#### Keywords

##### Definitions

- `struct` - Define a data structure.

    ```rs
    struct /* name */ {
        /* fields */
    }
    ```

- `union` - Define a union type.

    ```rs
    union /* name */ {
        /* variants */
    }
    ```

- `enum` - Define an enumerated type.

    ```rs
    enum /* name */ {
        /* variants */
    }
    ```

- `type` - Define a type alias.

    ```rs
    enum /* alias */ = /* type */;
    ```

- `def` - Define a macro.

    ```rs
    enum /* alias */ = /* type */;
    ```

- `fn` - Define a function.

    ```rs
    fn /* name */(/* parameter 1 */, /* parameter 2 */, /* ... */, /* parameter n */) -> /* return type */ {
        // definition
    }
    ```

##### Value

- `let` - Create a local variable.

    Declaration

    ```rs
    let /* name */;
    ```

    Definition

    ```rs
    let /* name */ = /* initial value */;
    ```

- `const` - Create a constant.

    ```rs
    const /* name */ = /* constant value */;
    ```

- `static` - Create a global variable.

    ```rs
    static /* name */ = /* initial value */;
    ```

##### Interface

- `where` - Supplies requirements for function parameters.

    ```rs
    fn foo(v, fun) -> str
    where
        v.x: flt,
        v.y: flt,
        v.mag: (self) -> flt,
        fun: (flt) -> str,
    {
        // ...
    }
    ```

    When a `where` clause is present, any errors that might have been emitted at the function definition but have been specified in the `where` , will instead be attributed to the caller.

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
        bar(5); // ERROR: argument `v` of `bar` expects a field `x`, but `5` (uint) has no such field
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

##### Flow control

###### Conditional

- `if` - Only perform the statement if the condition holds.

    ```rs
    if /* condition */ {
        // statement
    }
    ```

- `else` - When following an `if` statement, only performs the statement if the condition does not hold.

    ```rs
    if /* ... */ {
        // ...
    } else {
        // statement
    }
    ```

    Can be followed by an `if` to add an additional condition.

    ```rs
    if /* ... */ {
        // ...
    } else if /* extra condition */ {
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

###### Loop

- `while` Repeat while a condition is true.

    ```rs
    while /* condition */ {
        // statement
    }
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
    while __item.is_some() {
        let /* binding */ = __item;
        // statement
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

- `loop` Repeat forever (or until a `break`/`ret`).

    ```rs
    loop {
        // statement
    }
    ```

    Equivalent to

    ```rs
    while true {
        // statement
    }
    ```

- `do` - A conditionless, single-iteration loop that can be "early-returned" from (using `break`) without exiting the function. Saves from having to make a new function that would only be used in one place, just for the sake of returning if there's an error.

    ```rs
    do {
        // statement
    }
    ```

    Equivalent to

    ```rs
    while true {
        // statement
        break;
    }
    ```

##### Loop Control

- `break` - Exit the current loop.

    ```rs
    /* for/while/loop/do */ {
        if /* condition */ { break; }
    }
    ```

- `skip` - Stop the current loop and skip to the next iteration.

    ```rs
    /* for/while/loop */ {
        if /* condition */ { skip; }
    }
    ```

###### Exit

- `ret` - End the function and output the value.

    ```rs
    ret /* value */;
    ```

- `yeild` - Return the value within a loop without ending the function, to allow for iterable functions.

    ```rs
    yeild /* value */;
    ```

### Punctuation

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
- `/=`: DivAssign - Equivalent to `lhs = lhs / rhs`
- `::`: PathSep - Separate namespace path items
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
- `.`: Dot - Access a struct member
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

## Execution

1. Install [The Rust Programming Language](https://rust-lang.org/tools/install/)
2. Navigate to [the project folder](../) in a terminal (this can be the vscode terminal)
3. Execute one of the following commands:
    - `cargo run` to run in interactive mode
    - `cargo run -- <FILE>` to run the contents of a file (the `--` is needed to distinguish `cargo` arguments from `batscript` arguments; this is equivalent to running `batscript <FILE>`)
4. To exit interactive mode, input `exit` or `quit`.

To run tests, execute the command `cargo test`.

## Unit Tests

[lab1.rs](/src/test/lab1.rs)

- `test_scan_whitespace_single`
- `test_scan_whitespace_multi`
- `test_scan_line_comment_no_newline`
- `test_scan_line_comment_typical`
- `test_scan_ident_simple`
- `test_scan_ident_prime`
- `test_scan_ident_apostrophe`
- `test_scan_number_simple`
- `test_scan_number_multidigit`
- `test_scan_number_negative`
- `test_scan_number_decimal`
- `test_scan_number_multidigit_decimal`
- `test_scan_number_multidigit_decimal_negative`
- `test_scan_number_sci_notation`
- `test_scan_number_neg_sci_notation`
- `test_scan_number_neg_sci_notation_multidigit_exp`
- `test_scan_number_neg_sci_notation_multidigit_exp_negative`
- `test_scan_number_oct`
- `test_scan_number_oct`
- `test_scan_number_bin`
- `test_scan_char_simple`
- `test_scan_char_multi`
- `test_scan_char_empty`
- `test_scan_char_escaped`
- `test_scan_char_escaped_hex`
- `test_scan_char_escaped_multi`
- `test_scan_char_escaped_invalid`

## Known limitations/Failures

- Error snippets do not display token styling (styling is applied using ANSI sequences not present in the source code, which interfere with lexeme ranges).
- Sub-token (ex: escape sequences) errors are identified as errors for the entire token, not just the range of the eroneous subtoken.
- The lexeme `/* /*/ */` treats has its `/*/` treated like an entire nested block comment despite having only one `*`. This does not occur outside of block comments.
