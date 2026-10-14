let foo = 7;
let bar = 8;

if foo > bar {
    // TODO: this is not good, it exploits scopes not popping yet.
    // but also variables aren't mutable yet, so it's only fair.
    let bigger = "foo";
} or {
    let bigger = "bar";
}
prnt bigger + " is bigger";

let foobar = foo * bar;

prnt "the product of foo and bar is " + foobar;