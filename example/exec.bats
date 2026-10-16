let foo = 7;
let bar = 8;

let bigger = "TODO";
if foo > bar {
    // TODO: this is not good, it exploits scopes not popping yet.
    // but also variables aren't mutable yet, so it's only fair.
    //bigger = "foo";
    prnt("todo: assign `bigger` with \"foo\"\n");
} or {
    //bigger = "bar";
    prnt("todo: assign `bigger` with \"bar\"\n");
}
prnt(bigger + " is bigger\n" + '\n'); // WHY AREN'T ESCAPES WORKING IN STRINGS??

let foobar = foo * bar;

prnt("the product of foo and bar is " + foobar + '\n');