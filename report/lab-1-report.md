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

Assume all of the following are successful (actual output = expected output), as failures would be in [Known limitations/Failures](#known-limitationsfailures).

I'm not sure how you expect me to test *every edge case*. That's quite a lot. But here are the 41 cases I managed to *think of*.

- `test_scan_whitespace_single`
  - purpose: confirm " " scans as whitespace
  - input: ` `
  - expected: `[Whitespace(" ")]`
- `test_scan_whitespace_multi`
  - purpose: confirm multiple whitespace characters scan as one whitespace token
  - input: ` \n\r\t `
  - expected: `[Whitespace(" \n\r\t ")]`
- `test_scan_line_comment_no_newline`
  - purpose: confirm that a line comment will scan as a comment even if ending at EOF
  - input: `// apple`
  - expected: `[Comment("// apple")]`
- `test_scan_line_comment_typical`
  - purpose: confirm the newline character is excluded from a line comment token
  - input: `// apple\n`
  - expected: `[Comment("// apple"), Whitespace("\n")]`
- `test_scan_ident_simple`
  - purpose: confirm a simple identifier will scan as an identifier
  - input: `foo`
  - expected: `[Identifier("foo")]`
- `test_scan_ident_prime`
  - purpose: confirm that "prime" notation in an identifier is valid
  - input: `x'`
  - expected: `[Identifier("x'")]`
- `test_scan_ident_apostrophe`
  - purpose: confirm that identifiers can contain apostrophes
  - input: `can't`
  - expected: `[Identifier("can't")]`
- `test_scan_number_simple`
  - purpose: confirm that a digit is scanned as a number
  - input: `5`
  - expected: `[UIntLiteral(5)]`
- `test_scan_number_multidigit`
  - purpose: confirm that multiple consecutive digits are scanned as a single number
  - input: `35`
  - expected: `[UIntLiteral(35)]`
- `test_scan_number_negative`
  - purpose: confirm that numbers can be negative
  - input: `-5`
  - expected: `[SIntLiteral(-5)]`
- `test_scan_number_decimal`
  - purpose: confirm that numbers can have decimals
  - input: `2.5`
  - expected: `[FltLiteral(2.5)]`
- `test_scan_number_multidigit_decimal`
  - purpose: confirm that decimal numbers can have multiple digits
  - input: `25.25`
  - expected: `[FltLiteral(25.25)]`
- `test_scan_number_multidigit_decimal_negative`
  - purpose: confirm that decimal numbers can be negative
  - input: `-25.25`
  - expected: `[FltLiteral(-25.25)]`
- `test_scan_number_sci_notation`
  - purpose: confirm single-digit scientific notation is valid
  - input: `5e0`
  - expected: `[FltLiteral(5e0)]`
- `test_scan_number_neg_sci_notation`
  - purpose: confirm scientific notation can have a negative exponent
  - input: `5e-5`
  - expected: `[FltLiteral(5e-5)]`
- `test_scan_number_neg_sci_notation_multidigit_exp`
  - purpose: confirm scientific notation can have a multiple-digit negative exponent
  - input: `5e-50`
  - expected: `[FltLiteral(5e-50)]`
- `test_scan_number_neg_sci_notation_multidigit_exp_negative`
  - purpose: confirm scientific notation can be negative, and have a negative exponent
  - input: `-5e-50`
  - expected: `[FltLiteral(-5e-50)]`
- `test_scan_number_neg_sci_notation_multidigit_exp_excess_negative`
  - purpose: confirm that scientific notation can only have one hyphen after the "e"
  - input: `-5e-5-3`
  - expected: `[FltLiteral(-5e-5), Punctuation(Sub), UIntLiteral(3)]`
- `test_scan_number_neg_sci_notation_multidigit_exp_double_negative`
  - purpose: confirm that scientific notation can only have one hyphen after the "e"
  - input: `-5e--5`
  - expected: `[Err(InvalidNumLiteral), Punctuation(Sub), SIntLiteral(-5)]`
- `test_scan_number_hex`
  - purpose: confirm hexadecimal literals are valid
  - input: `0x9F`
  - expected: `[UIntLiteral(0x9F)]`
- `test_scan_number_hex_bad_digit`
  - purpose: confirm hexadecimal literals must be made of hexadecimal digits
  - input: `0x9G`
  - expected: `[Err(InvalidNumLiteral)]`
- `test_scan_number_oct`
  - purpose: confirm octal literals are valid
  - input: `0o253`
  - expected: `[UIntLiteral(0o253)]`
- `test_scan_number_oct_bad_digit`
  - purpose: confirm octal literals must be made of octal digits
  - input: `0o258`
  - expected: `[Err(InvalidNumLiteral)]`
- `test_scan_number_bin`
  - purpose: confirm binary literals are valid
  - input: `0b11011011`
  - expected: `[UIntLiteral(0b1101_1011)]`
- `test_scan_number_bin_wrong_digit`
  - purpose: confirm binary literals must be made of binary digits
  - input: `0b11011012`
  - expected: `[Err(InvalidNumLiteral)]`
- `test_scan_char_simple`
  - purpose: confirm character literals are valid
  - input: `'a'`
  - expected: `[CharLiteral('a')]`
- `test_scan_char_multi`
  - purpose: confirm a character literal can only have one character
  - input: `'aa'`
  - expected: `[Err(MultiCharLiteral)]`
- `test_scan_char_empty`
  - purpose: confirm a character literal cannot be empty
  - input: `''`
  - expected: `[Err(EmptyCharLiteral)]`
- `test_scan_char_endless`
  - purpose: confirm it is an error to not have a closing delimiter for character literals
  - input: `'`
  - expected: `[Err(EndlessCharLiteral)]`
- `test_scan_char_escaped`
  - purpose: confirm it is valid to have an escape sequence in character literals
  - input: `'\\0'`
  - expected: `[CharLiteral('\0')]`
- `test_scan_char_escaped_hex`
  - purpose: confirm it is valid to have a multi-character escape sequence (representing a single codepoint) in character literals
  - input: `'\\x1b'`
  - expected: `[CharLiteral('\x1b')]`
- `test_scan_char_escaped_multi`
  - purpose: confirm a multi-character escape sequence has to be valid
  - input: `'\\1b'`
  - expected: `[Err(MultiCharLiteral)]`
- `test_scan_char_escaped_invalid`
  - purpose: confirm the close delimiter can be escaped, making the literal endless
  - input: `'\\'`
  - expected: `[Err(EscapedCharLiteralEnd)]`
- `test_scan_str_simple`
  - purpose: confirm string literals exist
  - input: `\"a\"`
  - expected: `[StrLiteral("a")]`
- `test_scan_str_multi`
  - purpose: confirm string literals can contain multiple characters
  - input: `\"aa\"`
  - expected: `[StrLiteral("aa")]`
- `test_scan_str_empty`
  - purpose: confirm strings can be empty
  - input: `\"\"`
  - expected: `[StrLiteral("")]`
- `test_scan_str_endless`
  - purpose: confirm it is an error not to have a closing delimiter for a string
  - input: `\"`
  - expected: `[Err(EndlessStringLiteral)]`
- `test_scan_str_escaped`
  - purpose: confirm string literals can contain escape sequences
  - input: `\"\\0\"`
  - expected: `[StrLiteral("\0")]`
- `test_scan_str_escaped_hex`
  - purpose: confirm string literals can contain multi-character escape sequences
  - input: `\"\\x1b\"`
  - expected: `[StrLiteral("\x1b")]`
- `test_scan_str_escaped_multi`
  - purpose: I copied this from the char tests but it doesn't necessarily do anything special for strings
  - input: `\"\\1b\"`
  - expected: `[StrLiteral("\\1b")]`
- `test_scan_str_escaped_invalid`
  - purpose: confirm the close delimiter can be escaped, making the literal endless
  - input: `\"\\\"`
  - expected: `[Err(EscapedStringLiteralEnd)]`

## Known limitations/Failures

- Error snippets do not display token styling (styling is applied using ANSI sequences not present in the source code, which interfere with lexeme ranges).
- Sub-token (ex: escape sequences) errors are identified as errors for the entire token, not just the range of the eroneous subtoken.
- The lexeme `/* /*/ */` treats has its `/*/` treated like an entire nested block comment despite having only one `*`. This does not occur outside of block comments.
