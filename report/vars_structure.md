# Variable Structure

## Static Analysis

`StaticEnvironment`

```rs
pub struct StaticEnvironment<'src> {
    locs: Vec<MemLoc<'src>>,  // used in the cementation phase
    global: Scope<'src>,      // global scope is always available and is incapable of being popped
    layers: Vec<Layer<'src>>, // stack of layers currently being worked in - local layer is last
}

pub struct MemLoc<'src> {
    pub atop: Option<VarId>, // the variable this one will be stacked on top of - in case we are waiting on it to determine our own size
    pub key: EnvKey<'src>,   // the identifier or operator of the item (for error reporting, since `locs` will only ever push, but `layers` can pop)
    pub pos: MemPos,         // all known info about where the variable can be stored
}

pub enum MemPos {
    Unknown,             // size and position unknown
    Blocked(usize),      // size known, position unknown
    Fixed(Range<usize>), // size and position known
}

pub struct Layer<'src> {
    pub root: Scope<'src>,        // every function starts out with a root scope; popping this scope would mean popping the entire layer
    pub scopes: Vec<Scope<'src>>, // stack of scopes currently being worked in - local scope is last
}

pub struct Scope<'src> {
    pub items: Vec<ScopeItem<'src>>, // list of all items currently in scope - names can be repeated, most recent item is last
}

pub struct ScopeItem<'src> {
    pub id: VarId,          // monotonically-incrementing ID
    pub key: EnvKey<'src>,  // the identifier or operator of the item
    pub val: EnvItem<'src>, // the definition of the item
}
```

`'src` is the lifetime of the original source code in memory. It is present because these structures contain fields that, somewhere within them, reference the original source code to minimize expensive string copies.

During static analysis, when a new "item" (local variable, static variable, constant, function, or operator overload) is declared, it is removed from the AST structure since it doesn't actually involve evaluation. It is simply a marker indicating "this is what this name refers to for the time being". When this happens, it is inserted into the `StaticEnvironment`.

Initially, the `StaticEnvironment` has only one scope: global scope. This scope is always present, and cannot be popped. It is important to note, however, that "global scope" is not where all "global variables" are stored; it's just the visibility level of any item declared without being inside of a Block AST node. For example: the `main` function would be declared in global scope.

When a function is defined, a new layer is created. Layers ensure that, even if a function is defined *inside* a function (like in the following example), it can only see its OWN locals (and global items). Scopes can see items from outer scopes, but layers can NOT see items from outer layers (unless they are in the global scope).

```rs
// global scope

uni n = 1;

fun foo() { // start of layer

    // n is still visible: it's in the global scope

    let x = 5;

    { // start of scope

        // n is still visible: it's in the global scope
        // x is still visible: it's from a different scope, but same layer

        let y = 3;

    } // end of scope
    // y is no longer visible: its scope has popped
    // x is still visible

    fun bar() { // start of layer

        // n is still visible: it's in the global scope
        // x is NOT visible: it's from a different layer

        let z = 8;

        { // start of scope

            // n is still visible: it's in the global scope
            // z is still visible: it's from a different scope, but same layer
            let w = 2;

        } // end of scope
        // w is no longer visible: its scope has popped
        // z is still visible

    } // end of layer
    // z is no longer visible: its layer has popped
    // x is visible again: it's from this layer

} // end of layer
```

**Note:** In Batscript, a "global" item just means it is *visible* everywhere. It is in *global scope*. This is distinct from "`uni`versal" (static) variables, which might not be visible everywhere, but are **never trampled over**. Universal variables are given dedicated memory ranges that are unaffected by the stack pushing or popping, even though their visibility might be limited to one scope.

**Example:**

```rs
// A "global-scope" variable which is not static.
// If global scope could pop, this variable could be popped off the stack and trampled.
// (note: it is not actually legal to use `let` at global scope; this is just for demonstration)
let x = 5;

fun foo() {
    {
        // A local-scope variable which is static.
        // Even though this variable's visibility is limited to this scope, it will still exist when the scope pops.
        uni y = 4;
    }
    // y is no longer VISIBLE: its scope has popped - however, its memory range is still reserved

    // if y had been declared with `let`, then z would be allowed to write to y's memory range.
    // however, it was declared with `uni`, so it's not actually stored somewhere z could overwrite.
    let z = 9;
}
```

During the "cementation" phase (which turns abstract variables into concrete ones), all variables are given specific memory ranges. Cementation is performed by iterating over every unique variable and assigning their range starting (with possible offset due to alignment) at the end of the variable they are stacked `atop` and ending `size` bytes later. By cementing variables in order of their monotonically-generated IDs, we are guaranteed to never visit a variable `atop` a variable that has not been cemented, unless its implicit type was never determined (which is a static analysis error), since a variable cannot stack `atop` a variable with a higher ID than its own. This means we don't have to recursively check every variable's parent, since we'll have already visited all of its ancestors before visiting it.

This phase is not done until the end of static analysis, because:

1. A universal variable could be declared after a local variable, but needs to be stored ahead of all local variables in the `Environment` stack.

    ```rs
    uni x = 6
    let y = 5;
    uni z = 8;
    let w = 3;
    ```

    ```txt
      x z y w
    [ 6 8 5 3 ]
      ^^^ --- local variable memory
      |
      universal variable memory
    ```

2. Late initialization with implicit type means the variable size is not known until its first assignment. Without knowing the size of a variable, it's impossible to know where the next one should be stored--even if *that one's* size is known.

    ```rs
    let x;             // MemPos::Unknown: unknown size, position 0
    let y;             // MemPos::Unknown: unknown size, unknown position, but stacks atop x
    let z = 5;         // MemPos::Blocked: 8 bytes (align 8), unknown position, but stacks atop y
    y = true;          // MemPos::Blocked: 1 byte (align 1), unknown position
    x = "hello world"; // MemPos::Fixed: 24 bytes* (align 8), position 0
    // so
    // - x is  0..24
    // - y is 24..25
    // - z is 32..56 (doesn't start immediately after y due to alignment)
    ```

    \*String literals are currently implemented as `String` (`Vec<u8>`), as opposed to being an array of their characters. This will not be true in the future.

After cementation, variable names and types **no longer exist** (except when debugging the language).

## Runtime

`Environment`

```rs
pub struct Environment<'stk> {
    stack: &'stk mut [MaybeUninit<u8>], // arbitrary memory for storing variables at runtime
    sp: usize,                          // stack pointer
}
```

`'stk` is the lifetime of the `stack`'s memory pool. The bytestring pool is allocated at the start of the runtime and never changes. This lifetime is present so that the `Environment` does not need to worry about its ownership, only its memory.

The `Environment` has no awareness of variable names or types. It is just an arbitrary bytestring that can have segments indexed and reinterpreted as other types. Because the `Environment` is not aware of types, it trusts the static analysis to have baked the type interpretations into the operators.

When accessing a segment of the `Environment`, the functions `get()`, `get_mut()`, or `get_disjoint_mut()` can be used to access arbitrary ranges of the `stack`. Then, those bytestrings can be `cast_into()` or `cast_from()` baked-in types by the built-in operators. User-defined functions do not have the ability to cast the memory, and are limited to simply passing arbitrary bytestrings representing user-types in such a way that the only time any subset of one's memory casts into an actual Rust type is within the built-in operators.

`sp` represents the divider between used and unused memory in `stack`. All variables whose ranges are ahead of `sp` are in active use, and overwriting them (except within during an Assignment operation) can cause UB. Variables after `sp` are not currently in use, and are free to be overwritten with temporary/intermediate results (so long as those temporary values will not be accessed after the variables they are occupying the memory of are pushed to the stack). In other words, `sp` is the position of the first not-in-use memory location in the stack. Everything before it is a "saved" and everything after it is "temporary".

When a variable is accessed, it simply indexes into `stack` with the assumption that the range it was assigned[^1] during cementation is legitimiate. If this assumption is false, the language has failed.

[^1]: If the variable is from within a `Layer`, its range is actually an *offset* from what `sp` was at the time its layer was entered into, rather than directly indexing into `stack` (since function calls can be recursive and therefore don't have permanent local variable ranges).

**Example:**

```rs
let x = 5;
let y = 2;
{ let a = 7; }
let z = x + y * 2;
uni w = x * z;
```

(note: `( )` means uninitialized and after `sp`, `{ }` means uninitialzed but ahead of `sp`)

```txt
 (_)(_)(_)(_)(_)(_)(_)(_)
[ 0  0  0  0  0  0  0  0 ]
  ^ sp

// let x = 5;
 {w} x (_)(_)(_)(_)(_)(_)
[ 0  5  0  0  0  0  0  0 ]
        ^ sp

// let y = 2;
 {w} x  y (_)(_)(_)(_)(_)
[ 0  5  2  0  0  0  0  0 ]
           ^ sp

// { let a = 7;
 {w} x  y  a (_)(_)(_)(_)
[ 0  5  2  7  0  0  0  0 ]
              ^ sp

// }
 {w} x  y (_)(_)(_)(_)(_)
[ 0  5  2  7  0  0  0  0 ]
           ^ sp

// v--- let z = x + y * 2; ---v

// t = y * 2
 {w} x  y {z} t (_)(_)(_)
[ 0  5  2  7  4  0  0  0 ]
                 ^ sp

// u = x + t
 {w} x  y {z} t  u (_)(_)
[ 0  5  2  7  4  9  0  0 ]
                    ^ sp

// z = u
 {w} x  y  z (_)(_)(_)(_)
[ 0  5  2  9  4  9  0  0 ]
              ^ sp

// ^--------------------------^

// v--- let w = x * z; ---v

// x * z
 {w} x  y  z  t (_)(_)(_)
[ 0  5  2  9  45 9  0  0 ]
                 ^ sp

// w = t
  w  x  y  z (_)(_)(_)(_)
[ 45 5  2  9  45 9  0  0 ]
              ^ sp

// ^----------------------^
```

**Note:** this example is an idealization. I am not certain if Batscript is currently functioning correctly at this time.
