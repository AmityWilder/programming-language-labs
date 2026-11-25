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
// outer `b` pops, global `b` is still in scope and no longer shadowed
// outer `a` pops, global `a` is still in scope and no longer shadowed
prnt(a+'\n'); // expect "global a"
prnt(b+'\n'); // expect "global b"
prnt(c+'\n'); // expect "global c"
