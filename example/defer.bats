{
    let x = 0;
    loop {
        defer { if x >= 10 { halt; } }
        x++;
        prnt(x + "\n");
    }

    let x = 0;
    defer { prnt(x + "\n"); }
    x = x + 1;
    defer { prnt(x + "\n"); }
    x = x + 1;
    // should print "2 2"
}