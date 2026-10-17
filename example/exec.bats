// todo: make this a function
{
    let foo = 7;
    foo = 9;
    //foo = "x"; // error: can't reassign
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
    prnt(bigger + " is bigger\n");

    let foobar = foo * bar + 5 * (2 - 8 + bar ** 2) / 3 % x;
    prnt("foobar = " + foobar + '\n');
    prnt("the pvt FOO = " + FOO + '\n');
}
