# Variable

**Note:** "run command" suggestions assume Rust/Cargo are installed, and that you are within the root folder of the repository.

## Declaration, Access, and Assignment

### Variable access before declaration

Static analysis error. If a variable is accessed before declaration or in a wider scope than the one in which it was declared, static analysis produces an "unknown variable" error.

**Example:** [`var-early-access.bats`](../example/var-early-access.bats)

**Run command:** `cargo run -- example/var-early-access.bats`

```rs
prnt(x); // expect "unknown variable" error
let x = 3;

{
    let y = 5;
}
prnt(y); // expect "unknown variable" error - `y` has popped off the stack
```

```
error: cannot find variable within this scope
  --> example\var-early-access.bats:1:6-7
  |
1 |  prnt(x); // expect "unknown variable" error
  |       ^ unknown variable
  |
help: check if the variable is in a tighter scope than where you are trying to use it

error: cannot find variable within this scope
  --> example\var-early-access.bats:7:6-7
  |
7 |  prnt(y); // expect "unknown variable" error - `y` has popped off the stack
  |       ^ unknown variable
  |
help: check if the variable is in a tighter scope than where you are trying to use it
```

### Missing initializer in declaration

**Note:** The following (specifically everything in the [Missing initializer in declaration](#missing-initializer-in-declaration) section) is the decision, but not the *current* implementation. Variables can be late-initialized, but accessing them prior to initialization is not statically analyzed yet, and may result in unspecified behavior. Batscript runs within safe Rust, so it won't cause actual Un*defined* Behavior, but I have not specified how it should treat such situations. So for now, it may panic/error at runtime, use variables that have been default-initialized, or an arbitrary mix of the two.

If a variable declaration has no initializer, the variable is "late initialized". Its [**first assignment**](#first-assignment) is used to determine the type. Accessing the variable before it has been initialized produces a static analysis error.

If a variable is conditionally initialized, e.g.

```rs
let x;
if is_wednesday {
    x = 6;
}
prnt(x);
```

that initialization only lasts within the body of the case where it was initialized. Attempting to access a variable when the analyzer **cannot prove** it has been initialized yet will produce a static analysis error.

A late-initialized variable is allowed to be accessed outside of the branches it is initialized in, if and only if the analyzer **can prove** the variable is initialized in *every* case that can reach the variable access (this avoids the halting problem by only being a conservative estimate).

**Example:**

```rs
let x;
if inpt == "hello" {
    x = 6;
}
// error: `inpt == "hello"` might be false; then `x` would be uninitialized
prnt(x);
```

```rs
let x;
if 5 > 3 {
    x = 6;
} or if true {
    // (doesn't initialize x)
} or {
    // (doesn't initialize x)
}
// `5 > 3` will always be true. all other cases are considered "dead",
// so `x` is guaranteed to be initialized.
//
// a warning may still be produced, since using a tautology in a branch
// condition obfuscates the code's intent.
prnt(x);
```

```rs
cord {
    let x;
    pick inpt {
        "apple"  => { x = 5; }
        "orange" => { x = 3; }
        "banana" => { x = 9; }
        "tomato" => { done; } // exits the cord
        _ => { skip; } // restarts the cord
    }
    // while `x` is not initialized in every branch case,
    // it is initialized in every branch case *which can reach this line*.
    prnt(x);
}
```

```rs
cord {
    let x;
    if inpt == "i want " {
        pick inpt {
            "apple"  => { x = 5; }
            "orange" => { x = 3; }
            "banana" => { x = 9; }
            "tomato" => {}
            _ => { skip; }
        }
    } or {
        x = 0;
    }
    // error: `inpt == "i want "` might be true, then `inpt` might be `"tomato"`; then `x` would be uninitialized.
    prnt(x);
}
```

#### First assignment

If a variable is not assigned in its declaration, the analyzer will hold onto its ID and remember that it has not been assigned yet. The analyzer will continue to traverse the AST.

The **very next time** the exact same variable (determined by ID, not by name) is accessed for any reason, it **must** be initialized. `if`/`or`/`pick` statements are traversed in reading order (left to right, top to bottom), so if the variable is mentioned in multiple branches, the *first* branch it is found in will determine its type.

Example:

```rs
let x;
x = 5; // <- first assignment

let y;
loop {
    y = 5; // <- first assignment
}

let y;
if 5 < 3 {
    y = 5; // <- first assignment
} or {
    // even though the `5 < 3` branch is a contradiction
    // and can never execute, it is still visited before
    // this branch, so the analyzer will use that branch
    // to deduce `y`'s implicit type as an `sint`.

    y = "hello"; // type mismatch error: expected `sint`, found `text`
}

let z;
if /* ... */ {
    // ...
} or {
    if /* ... */ {
        // ...
    } or if /* ... */ {
        pick /* ... */ {
            1 => {
                // ...
            }
            2 => {
                if /* ... */ {
                    // ...
                } or {
                    z = none;
                }
            }
            3..6 => {
                // ...
            }
            _ => {
                // ...
            }
        }
    } or {
        // ...
    }
}

// union types cannot be implicit. even though it is perfectly
// reasonable to assume `z` would be `char | none`, since "or null" is
// a very common union, `z` will instead be implicitly deduced as
// a `none`-type unless explicitly annotated as `let z: char | none`.

// even though `z` is only initialized in one, highly-specific branch,
// that branch is still visited before this assignment, so that one
// will determine the type of `z`.

z = '&'; // type mismatch error: expected `sint`, found `char`
```

### Redeclaration

Redeclaring a variable creates a new variable with the same name as the existing one. The variables are completely separate and have nothing to do with each other except that they share an identifier (which is not stored at runtime).

## Block Scope

[`unmatched.bats`](../example/unmatched.bats)

**Run command:** `cargo run -- example/unmatched.bats`

```rs
loop {
    prnt("hello world");
// expect: error: missing close bracket
```

```
error: missing close bracket: expected `}`, found none
  --> example\unmatched.bats:3:40
  |
3 |  // expect: error: missing close bracket
  |                                          missing close bracket
  |
help: try inserting a `}` or remove the `{`
```

[`block-scope.bats`](../example/block-scope.bats)

**Run command:** `cargo run -- example/block-scope.bats`

```rs
let a = "global a";
let b = "global b";
let c = "global c";
{
    let a = "outer a"; // shadows global `a`
    let b = "outer b"; // shadows global `b`
    {
        let a = "inner a"; // shadows outer `a`
        prnt(a+'\n'); // expect "inner a"
        prnt(b+'\n'); // expect "outer b"
        prnt(c+'\n'); // expect "global b"
    }
    // inner `a` pops, outer `a` is still in scope and no longer shadowed
    prnt(a+'\n'); // expect "outer a"
    prnt(b+'\n'); // expect "outer b"
    prnt(c+'\n'); // expect "global c"
}
// outer `a` and `b` pop, global `a` and `b` are still in scope and no longer shadowed
prnt(a+'\n'); // expect "global a"
prnt(b+'\n'); // expect "global b"
prnt(c+'\n'); // expect "global c"
```

Output:

```
inner a
outer b
global c
outer a
outer b
global c
global a
global b
global c
```

## Invalid Assignment

[`unassignable.bats`](../example/unassignable.bats)

**Run command:** `cargo run -- example/unassignable.bats`

```rs
let x = "initializer"; // ok
prnt(x + "\n");
x = "post-initializer"; // ok
prnt(x + "\n");
((((((x)))))) = "grouped assignment"; // ok; still assigning to a variable, it's just in parentheses
prnt(x + "\n");
x + 1 = "copy assignment"; // expect: error: cannot assign to immutable
prnt(x + "\n"); // unreachable
```

```
initializer
post-initializer
grouped assignment
runtime error: cannot assign to immutable
```

**Note:** In the future, this will be caught by static analysis instead of at runtime.

## Failing test cases

- `tests::integration::test_complex`: Increment operator (`++`) has not yet been introduced to new executor
- `tests::integration::test_dfer`: Increment operator (`++`) has not yet been introduced to new executor
- `tests::integration::test_cond`: Preprocessor conditionals have not yet been reimplemented
- `tests::integration::test_preproc`: Preprocessor conditionals have not yet been reimplemented
- `tests::integration::test_label`: Labels have not yet been reimplemented

**Note:** a number of test cases are currently disabled, as the recent refactor has caused them to now contain Rust semantic errors. These tests will be fixed over time.
