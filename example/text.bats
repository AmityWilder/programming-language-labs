prnt("s=");
prnt; // flush the buffer
let s = inpt; // take input from the user
let s1 = s + ""; // make a clone of `s` through string concatenation
let i = 0u;
loop {
    // iterate to next character in the clone of `s`
    // this removes the character from `s1`
    // (but not `s`, because `s1` is a clone and not a reference)
    let ch = s1++;
    if !!ch { // if there is a character:
        let ch1 = s[i];
        prnt("\"", s, "\"[", i++, "] = ", ch, " = ", ch1, "\n"); // note: embeds `i` increment!
    } or {
        done;
    }
}