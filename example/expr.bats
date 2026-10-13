3 * 2 + 6 - (2 + 2 / -(6) ** 9) - ((5 % (3*3)))

5 + 2 [>>] 3 <= 8 !& 6 // expect incompatible operands error

(-5)

+ 4 ** 54 // expect overflow error
(2 ** 64) // expect overflow error
(2 ** 63) // expect overflow error
(2u ** 64u) // expect overflow error

2+2+2+2+2+2+2

def \foo($x) { /* from \foo */ ($x+5) } // \foo definition
def \bar($a $b) { /* from \bar */ (3*($a)**$b) } // \bar definition
def \baz() { /* from \baz */ (200) } // \baz definition

(-(-9_223_372_036_854_775_808)) // expect overflow error
0b1111_1111__1111_1111__1111_1111__1111_1111__1111_1111__1111_1111__1111_1111__1111_1111u
0xFF_FF_FF_FF_FF_FF_FF_FFu + 0b1001_1100u // expect overflow error

\foo{\bar{\baz}{3}}

fals !| true

"i have " + (5 * 6) + " instances of what i would call an \"apple\"\n"

"squeak " + !!none

!(!5) != !!5 // expect incompatible operands error

5 + none?

// `-:>` is conversion, `=:>` is reinterpretation

5u -:> sint - 7s // should be ok
(-5 -:> uint + 3u) // error: fails, needs `uint | fail`
(5 -:> uint + 3u)  // error: COULD fail, needs `uint | fail`
(-5 -:> uint | fail + 3u) // fallible, fails at runtime
(5 -:> uint | fail + 3u)  // fallible, but succeeds

true -:> text   // stringify
"true" -:> bool // parse
none -:> uint   // error

(-5 -:> uint | fail) // fails at runtime
(-5 =:> uint) // succeeds, but changes representation

def \convert($op $ty) { (5 $op $ty) }
\convert{-:>}{uint | fail}
\convert{-:>}{fail} // expect error, you can't just convert directly to failure

def \foo($x) { $x }
5 -:> uint | \foo{none}
\foo{none}?

def \foo($x $y) { $x*$x+$y }
\foo{5*5*6}{3}

def \foo($x $y) { -5 $x $y }
def \bar($x $y) { $x | $y }
\foo{-:>}{\bar{uint}{none}}

def \likes_apples() { fals }
def \likes_oranges() { true }

rot { \likes_apples } {
    def \fruit() { "apple" }
} ~ rot { \likes_oranges } {
    def \fruit() { "orange" }
} ~ {
    def \fruit() { "banana" }
}

"i will eat a(n) " + \fruit

def \foo() { 5 }

//def \inf() { \inf + \foo + \inf }

rot { !(!!~\tbd) } {
    def \tbd() { "first" }
}

rot { !(!!~\tbd) } {
    def \tbd() { "second" }
}

"the next thing I will say is 'first': \"" + \tbd + "\""

/*
block comments can have syntax-highlighted code blocks
```
-6 -:> uint : fail
```
*/

szof sint
szof 5 // should be the size of the type the expression evaluates to
