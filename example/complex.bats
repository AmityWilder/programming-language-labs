{
    cord {
        prnt("input a number between 0 and 99 (inclusive)\n");
        let x = inpt;
        let success = fals;
        let is_odd;
        cord {
            // trivial case
            if x == "0" | x == "1" {
                is_odd = x == "1";
                halt; // early-exits the cord
            }
            // performs more expensive string concatenations,
            // so we want to avoid it if we can
            // find a number in 0..100 that is equal to x
            let i = 0u;
            rep i < 100u {
                let i = i++; // for loops dont exist yet :/
                if x == "" + i {
                    success = true;
                    is_odd = (i & 1u) == 1u;
                    halt; // exits the loop, and then the cord ends naturally
                }
            }
        }
        if !success {
            prnt("try again\n");
            skip;
        }
        let even_or_odd;
        if is_odd {
            even_or_odd = "odd";
        } or {
            even_or_odd = "even";
        }
        prnt(x + " is " + even_or_odd + "\n");
    }
}
