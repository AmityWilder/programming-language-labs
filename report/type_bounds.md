# Type System

## Concrete

A concrete type is a specific, post-monomorphization type. It can be one of the following:

- [Primitive](#primitive)
- [Array](#array)
- [View](#view)
- [Pointer](#pointer)
- [Fallible](#or-type)
- [Record](#rec)
- [Supertype](#super)
- [Subtype](#sub)
- [Categorical](#categorical)
- [Function](#function)

### Primitive

Primitive types are the shortlist of types specifically defined by the language author. In this case,

- [`none`](#none)
- [`fail`](#fail)
- [`nevr`](#nevr)
- [`bool`](#bool)
- [`byte`](#byte)
- [`uint`](#uint)
- [`sint`](#sint)
- [`flpt`](#flpt)
- [`char`](#char)
- [`text`](#text)

#### None

The return type of an empty block. This is equivalent to `()` in Rust, or `void` in C/C++.
It is *also* used to represent `null` (or `Option::None` in Rust) in the case of [or-types](#or-type).

It is valid to store a `none` in a variable. It is also reasonable to access it. However: it has no value and no members, thus doing so is pointless.

A `none` instance takes up 0 bytes.

#### Fail

The type of an exception. Trying to divide by 0, parse an invalid string, convert a negative to a uint, etc. will produce a `fail`.
Additionally, user-generated code may produce a `fail` with the `fail` keyword, which will act as an alternative means of returning from the method without providing a valid result.

It is valid to store a `fail` in a variable, or even to simply ignore it. However, trying to access it will immediately cause the current function to `fail` as well.

If a function is **capable** of outputting a `fail`, unless it is behind an unreachable branch (determined by the presence of a [`nevr`](#nevr)-producing statement; not through any magical halt-detection), the entire function must include `fail` as an [or-type](#or-type) in its return.

#### Nevr

Similar to [`none`](#none), except that this type represents a promise from the user that the type will not actually produce `none`.
In another sense, `nevr` is the return value of `loop {}` (because without breaking from the `loop`, it will never actually output a value).

Unlike `none` and [`fail`](#fail), if a `nevr` is so much as *constructed*, the program may crash at runtime.
There is no error-handling or recovery for a `nevr` type, as the use of `nevr` in an [or-type](#or-type) is a promise that the or-case is *unreachable*, not simply a failure.

If `nevr` could be constructed, it would take up 0 bytes.

#### Bool

A boolean: either `true` or `fals` (the trailing 'e' of "false" is truncated so that both variants have the same number of letters).

Due to hardware requirements, a boolean must be at least 1 byte despite only representing 1 bit of information.

#### Byte

An unsigned integer taking up exactly 1 byte. Can be any number between 0 and 255 (inclusively). Useful for bitflags.

#### Uint

An unsigned, pointer-width integer. i.e., 2 bytes on a 16-bit system, 4 bytes on a 32-bit system, 8 bytes on a 64-bit system.

#### Sint

A signed, pointer-width integer. i.e., 2 bytes on a 16-bit system, 4 bytes on a 32-bit system, 8 bytes on a 64-bit system.

#### Flpt

A pointer-width floating point. i.e., 2 bytes on a 16-bit system, 4 bytes on a 32-bit system, 8 bytes on a 64-bit system.

#### Char

A single UTF-8 codepoint. Uses 4 bytes.

#### Text

An owned, UTF-8 string.

### Array

Uses the syntax `[T; N]` where `T` is the type and `N` is a statically-known [`uint`](#uint). Stores `N` consecutive instances of `T` without needing a pointer.

### View

Like an [array](#array), but with a runtime-only size. Because the size is not known during static analysis, views must be stored indirectly behind a pointer.

### Pointer

TODO

### Or-Type

An or-type uses the syntax `T | E`, where `T` is the "then" type (success) and `E` is the "or" type (failure).

An or-type is a special language-defined anonymous union representing fallibility.

### Record

Defined with the `rec` keyword. In other languages, this is a `struct`.

### Super

Defined with the `sup` keyword. In other languages, this is a `union`. (TBD: tagged or untagged?)

### Sub

Defined with the `sub` keyword. A subtype is a type with additional restrictions in the form of a [type bound](#type-bounds). A subtype is guaranteed by static analysis to satisfy its bounds, reducing runtime checks.

### Categorical

Defined with the `cat` keyword. In other languages, this is an `enum`, with a particular difference: while `enum`s are usually syntax sugar for an integer subtype, categorical types can have *any* underlying representation--so long as its value can be evaluated during static analysis. In this way, a categorical type is closer to a collection of constant values, rather than just an integer with names.

### Function

Defined with the `fun` keyword. A function is able to be called with arguments to reuse code.

## Generic

A generic type is a type which includes one or more [generic parameter(s)](#generic-parameter) in order to metaprogram its field types.

While this type is unknown pre-monomorphization, it is still guaranteed to have its specified members; assuming those members aren't erased by their own [type bounds](#type-bounds).

A generic type uses the syntax `Foo<T, U>` where `Foo` is the generic type and `T` & `U` are its [parameters](#generic-parameter).

## Generic Parameter

A type which is unknown pre-monomorphization, used as a parameter in a [generic](#generic).

When a type is defined using `Foo<T>` where `Foo` is the [generic type](#generic), `T` is its parameter. Within the context of `Foo`'s definition, and nowhere else, `T` can be used as a substitute for whatever type is used as an argument.

Generic parameters can also be used in [functions](#function), where the generic parameter is local to the function's definition.

Generic parameters can be restricted according to [type bounds](#type-bounds), which determine what the type can/cannot be, and thus what it can "do".

## Opaque

An opaque type is a type defined purely by a [bound](#type-bounds) without a [generic parameter](#generic-parameter).
The main difference between a generic parameter and an opaque type is that a generic parameter is deduced from a type's/function's *arguments*, while an opaque type is deduced from the binding its *return* is assigned to.

For example:
```rs
fun foo() -> only char/only text {
    inpt // inpt's return is defined as `only byte/only char/only text`, so `foo` is restricting it further
}

// since x is explicitly `char`, which is one of `foo`'s variants,
// this instance of `foo` will return exactly `char`.
let x: char = foo();

fun bar(s: text) -> none {
    prnt(s);
}

let y = foo();
// since `bar` only accepts `text`, which is one of `foo`'s variants,
// `y` is implicitly `text`, making this instance of `foo` return `text`
bar(y);

fun baz<T>(s: T) -> none
    <=> T: only char/only text
{
    prnt(s)
}

// `baz` accepts either char or text, and `foo` is allowed to produce either.
// this will produce an "ambiguous type" error, as the return type of `foo` is unknown post-monomorphization.
baz(foo())
```

## Type Bounds

A type bound is a restriction placed on a [generic parameter](#generic-parameter) or [subtype](#sub) which narrows its definition to only types which satisfy the bound.

A bound can be one of the following:

- [Combo](#combo)
- [Not](#not)
- [Exactly](#exactly)
- [Has](#has)
- [Shape](#shape)
- [Supports](#supports)
- [Satisfies](#satisfies)

### Combo

A list of bounds that must be satisfied according to a combinator.

The combinator can be one of the following:

- [All](#all)
- [Any](#any)
- [One](#one)

#### All

For the bound to be satisfied, every one of its sub-bounds must be fulfilled.
Useful for cases where a generic parameter must be able to do multiple things.

Uses the syntax `A + B + ... + Z` where `A`, `B`, ..., and `Z` are bounds.

#### Any

For the bound to be satisfied, at least one of its sub-bounds must be fulfilled.

Uses the syntax `A | B | ... | Z` where `A`, `B`, ..., and `Z` are bounds.

#### One

For the bound to be satisfied, Exactly one of its sub-bounds must be fulfilled.
Almost exclusively used in [opaque types](#opaque).

Uses the syntax `A / B / ... / Z` where `A`, `B`, ..., and `Z` are bounds.

### Not

A negative bound. Requires its bound to be *un*fullfilled, rejecting the type if it is satisfied.
Potentially useful when the type system isn't smart enough to recognize that one bound being satisfied contradicts the satisfaction of another.

Uses the syntax `!B` where `B` is the bound being negated.

### Exactly

Requires the type to be exacly one type in particular.

Uses the syntax `only T` where `T` is a specific type (not a bound)

The `only` keyword is required in order to distinguish [any-combos](#any) from [or-types](#or-type).

### Has

Requires the type to have a member, by name. The member can be any of the following:

- A field (for [record types](#rec))
- An associated method ([`fun`](#function))
- An associated constant (`pvt`)
- A [categorical](#categorical) variant
- Any of the above on the base type of a [subtype](#sub) (assuming it isn't erased by the subtype bound)
- Any of the above on *every* variant of a [supertype](#super) or [categorical type](#categorical)

Uses the syntax `has foo` where `foo` is the name of the member.

If the member exists, that member can have an additional bound placed on it with the syntax `has foo: B` where `foo` is the name of the member and `B` is the bound `foo` must satisfy.

### Shape

Requires the type to be one of the following:

- A [function](#function) with the specified signature
- Any other type with the call-operator overloaded with the specified signature

Uses the syntax `fun(A, B, ..., Z) -> R` where `A`, `B`, ..., `Z`, and `R` are **types** (not bounds).

### Supports

Requires a type to overload the specified operator with the specified operands and output.

Syntax is currently undecided.

### Satisfies

Requires a type to satisfy a runtime expression as guaranteed by branching, returning, statically-evaluated math, builtin math involving literals, presence of [`nevr`](#nevr), and incoming "satisfies" bounds.

Uses the syntax `for x: expr` where `x` is a binding representing an arbitrary instance of the type, and `expr` is pure, builtin-operator expression involing `foo` that outputs a boolean. For the bound to be satisfied, `expr` must provably output `true` no matter what.

Example:
```rs
sub Positive = uint <=> for n: n > 1u; // non-zero uint

fun foo(x: Positive) -> none {
    // ...
}

fun bar(k: uint) -> none {
    if k == 0u {
        give; // return
    }
    // k now provably satisfies `k > 1u`, since uint cant be < 0 and we would have returned if it was == 0
    foo(k);
}

fun baz(k: uint) -> none {
    let h: Positive | nevr = k; // if `k` is <= 0, the program will crash
    foo(h);
}

fun bat() -> none {
    let x = 0;
    rep x < 5 {
        prnt(x);
    }
    // x is now provably >= 5, since the loop does not contain a break and therefore following lines
    // cannot be reached unless `x < 5` is false (`!(x < 5)` = `x >= 5`).

    let y = 0;
    rep y < 5 {
        if y == 3 {
            done; // break
        }
    }
    // y is provably `3`, since the loop will always break at `y == 3`, which can be reached,
    // and the loop will not repeat once broken out of.

    fun check(x: sint) -> bool {
        give x == 3;
    }

    let z = 0;
    rep z < 5 {
        if check(z) {
            done; // break
        }
    }
    // despite looking identical to the previous example, nothing about z is actually proven.
    // `check` does not place any "satisfies" bounds on x, and it isn't a pure, builtin operator.
    // in the future, it's possible `check` could be inlined or statically analyzed itself,
    // but this is not a promised feature at the moment.
}
```
