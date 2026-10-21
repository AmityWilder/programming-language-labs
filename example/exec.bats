// todo: make this a function
{
    let foo = 7;
    foo = 9;
    //foo = "x"; // error: can't reassign
    let bar = 8;
    bar = 9;

    prnt("write something\n");
    let input = inpt;
    prnt("I read: \"" + input + "\"\n");

    let i = -5;
    loop {
        if i > 5 { halt; }
        let temp = i;
        i = i + 1;
        let i = temp;
        if i == 0 { skip; } // prevent div/0 error
        let x = 10/i;
        prnt(x + "\n");
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
