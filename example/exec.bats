// todo: make this a function
{
    let foo = 7;
    foo = 9;
    //foo = "x"; // error: can't reassign
    let bar = 8;
    bar = 9;

    let i = 0;
    loop {
        if i >= 10 {
            halt;
        }
        prnt(i + "\n");
        i = i + 1;
    }

    let bigger;
    if foo > bar {
        bigger = "foo";
    } or if foo < bar {
        bigger = "bar";
    } or {
        bigger = "neither";
    }
    prnt(bigger + " is bigger\n");

    let foobar = foo * bar + 5 * (2 - 8 + bar ** 2u) / 3;
    prnt("foobar = " + foobar + '\n');
}
