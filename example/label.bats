{
    let i = 0;
    //let n = if true { 2 } or { 3 };
    .outer: rep i < 5 {
        let i = i++;
        let j = 0;
        rep j < 5 {
            let j = j++;
            prnt("(" + i + ", " + j + ")\n");
            if i == 3 & j == 3 {
                prnt("halt\n");
                halt .outer;
            }
        }
    }
}
