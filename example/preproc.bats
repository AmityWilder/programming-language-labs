def \foo {5}// definition of `\foo`

\foo; // expected: `5`

def \bar($x) {$x+1}// definition of `\bar`

\bar{7}; // expected: `7+1`

\bar{\foo}; // expected: `5+1`

def \baz($x $y) {def \bat($x) {$y*$x}}// definition of `\baz`

\baz{$u}{7}// expected: `def \bat($u) {7*$u}` (silent)

\bat{x}; // expected: `7*x`
2 + \bat{x} - 5; // expected: `2 + 7*x - 5`

def \foo($x) {def $x {8}} // redefines `\foo`

def \bar {\hidden} // redefines `\bar`

// expansion arguments are lazy and don't expand until the call expands,
// at which point `def \bar` has no reason to expand `\bar` and will simply redefine it.
\foo{\bar}// expected: `def \bar {8}` -> `def \hidden {8}` (silent)

//\hidden; // expected: macro undefined error

\bar; // expected: `8`

def \call($x) {3*$x}//definition of `\call`
def \args($x) {{7}}//definition of `\args`
def \wrap {\call}//definition of `\wrap`

//\call\args; // expected: macro missing arguments error

\wrap{8}; // expected: `\wrap` expands to `\call` without consuming the argument, then `\call{8}` expands to `3*8`
