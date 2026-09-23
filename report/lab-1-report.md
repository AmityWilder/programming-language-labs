# Batscript

**Note:** Tests are in [src/test](src/test).

## Regular Expressions

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

Same as string literals, but substitutes double quotes (`"`) with single quotes (`'`) and produces an error if more than one unicode character is contained in the value (not the lexeme) of the token.

### Interpolated string literal

Same as string literals, but substitutes double quotes (`"`) with graves (`` ` ``).

Supports escape sequences.

Instances of `${...}` have their contents (the `...` part excluding the `${}` part) passed into another `Scanner`. Escape sequences within string/character inside of balanced `${`/`}` pairs are attributed to the inner literal, not the interpreted string.

Attempting to nest an interpolated string within an interpolated string `${}` expression is **intentionally** unsupported and will result in a "missing close brace" error, because `` `${ `inner` }` `` is indistinguishable from \[`` `${ ` ``, `inner`, `` ` }` ``\]. This *could* be solved by choosing delimiters that aren't identical to each other, but this would then require recursion to parse instead of a fixed depth. I have decided against that.

### Identifiers

```re
[\p{letter}_][\p{letter}\p{number}'_]*
```

Identifiers must start with a letter (not restricted to ASCII) or underscore (`_`). The rest of the characters in the token can be letters (not restricted to ASCII), numbers (not restricted to ASCII), apostrophes (`'`), or underscores (`_`).

## Execution

1. Install [The Rust Programming Language](https://rust-lang.org/tools/install/)
2. Navigate to [the project folder](../) in a terminal (this can be the vscode terminal)
3. Execute one of the following commands:
    - `cargo run` to run in interactive mode
    - `cargo run -- <FILE>` to run the contents of a file (the `--` is needed to distinguish `cargo` arguments from `batscript` arguments; this is equivalent to running `batscript <FILE>`)
4. To exit interactive mode, input `exit` or `quit`.

To run tests, execute the command `cargo test`.

## Unit Tests

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

- Interpolated string expression open delimiters (`${`) cannot be escaped and will always open an interpolated expression.
- Error snippets do not display token styling.
- Escape sequence errors are identified as errors for the entire literal, not just the range of the escape sequence itself.
- Graves (`` ` ``) must be escaped (`` \` ``) in string literals within interpolated expression.
- Interpolated string expressions count `{`/`}` towards balancing even if they are within a comment or string literal.
- Escaping the `$` in an interpolated string expression delimiter crashes the language.
- `/* /*/ */` acts like a nested block even though it should be indistinguishable from `/* /* / */`
