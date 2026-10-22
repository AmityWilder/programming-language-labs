{
    let ch = inptch;
    prnt(ch + "\n");

    let i = 0;
    .outer: rep i < 5 {
        let j = 0;
        rep j < 5 {
            prnt("(" + i + ", " + j + ")\n");
            if i == 3 & j == 3 {
                prnt("halt\n");
                halt .outer;
            }
            j = j + 1;
        }
        i = i + 1;
    }
}
