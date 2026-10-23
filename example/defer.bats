{
    let x = 0;
    defer { prnt(x + "\n"); }
    x = x + 1;
    defer { prnt(x + "\n"); }
    x = x + 1;
    // should print "2 2"
}