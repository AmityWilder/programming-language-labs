# AST Printer

## Syntactic Grammar Design

### Supported Operators

### Supported Literal Types

## AST Printing

There are currently ~4 implementations for printing the AST:

- Rust's builtin `Debug` derive (I didn't write this, it just come with Rust, so I'm not counting it)
- Overload of Rust's builtin `Display`: tries to reconstruct the original source code from the AST (I did write this, but it's not very helpful, so I'm not really counting this either)

1. [`print_ast`](/src/main.rs) function: recursively calls itself with incrementing indentation depth & bracket depth
2. [`LispDisplay`](/src/grammar/mod.rs) trait: prints nodes in Lisp style
3. [`PolishDisplay`](/src/grammar/mod.rs) trait: prints nodes in Polish notation (replaces unary `-rhs` with `- 0 rhs` for lack of a better idea)

### Examples

Hard-coded AST in Rust code:

```rs
Expr::binary(Binary {
    lhs: Expr::unary(Unary {
        op: Token {
            lex: "-",
            val: LexValue::Punctuation(Punctuation::SubNeg),
            mac: None,
        },
        rhs: Expr::Literal(Token {
            lex: "123",
            val: LexValue::SIntLiteral(123),
            mac: None,
        }),
    }),
    op: Token {
        lex: "*",
        val: LexValue::Punctuation(Punctuation::Mul),
        mac: None,
    },
    rhs: Expr::grouping(Grouping {
        open: Token {
            lex: "(",
            val: LexValue::Punctuation(Punctuation::LParen),
            mac: None,
        },
        expr: Expr::literal(Token {
            lex: "45.67",
            val: LexValue::FracLiteral(45.67),
            mac: None,
        }),
        close: Token {
            lex: ")",
            val: LexValue::Punctuation(Punctuation::RParen),
            mac: None,
        },
    }),
})
```

`Display`:

```text
-123 * (45.67)
```

`Debug`:

```text
Binary(
    Binary {
        lhs: Unary(
            Unary {
                op: Punctuation("-"): SubNeg,
                rhs: Literal(
                    SIntLiteral("123"): 123,
                ),
            },
        ),
        op: Punctuation("*"): Mul,
        rhs: Grouping(
            Grouping {
                open: Punctuation("("): LParen,
                expr: Literal(
                    FltLiteral("45.67"): 45.67,
                ),
                close: Punctuation(")"): RParen,
            },
        ),
    },
)
```

`print_ast`:

```text
Binary:
 lhs: Unary:
   op: Punctuation("-"): SubNeg
   rhs: Literal: SIntLiteral("123"): 123
 op: Punctuation("*"): Mul
 rhs: Grouping:
   open: Punctuation("("): LParen
   expr: Literal: FltLiteral("45.67"): 45.67
   close: Punctuation(")"): RParen
```

`LispDisplay`:

```lisp
(* (- 123) (group 45.67))
```

`PolishDisplay`:

```text
* - 0 123 45.67
```

Image:

![test output](test_ast_printer1_output.png)
