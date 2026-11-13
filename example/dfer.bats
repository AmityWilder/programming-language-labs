{
    let x = 0;
    dfer {
        prnt("even though there's another `x` in this scope later, I bind to the current one.\n" +
            "after the upcoming code, it should be 2, and is " + x + ". ");
        if x == 2 {
            prnt("hooray!\n");
        } or {
            prnt("darn.\n");
        }
    }
    {
        dfer { prnt(x + "\n"); } // won't run until scope pops
        x = x + 1;
        dfer { prnt(x + " "); } // won't run until scope pops
        x = x + 1;
    } // should print "2 2"

    prnt("------\n");

    let x = 0;
    rep x < 100 {
        dfer { prnt("exiting\n"); }
        dfer { done; }
        dfer {
            prnt("this dfer skips, but that skip will be consumed by the outer dfer due to Batscript's rules. expect a warning.\n");
            skip;
        }
        prnt(x++ + "\n"); // should only iterate once
    }
}