{
    let i = 0;
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
