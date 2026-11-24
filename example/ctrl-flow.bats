if 5 < 3 {
    prnt("that's bad math\n");
} or if 1 == 0 {
    prnt("that's even worse math\n");
} or {
    prnt("math works\n");
}
let i = 0;
rep i < 5 {
    prnt(i + "\n");
    i = i + 1;
}
let i = 0;
cord {
    dfer { prnt("second\n"); }
    dfer { prnt("first\n"); }
    prnt(i + "\n");
    i = i + 1;
    if i == 1 { skip; }
}
prnt("finished\n");
