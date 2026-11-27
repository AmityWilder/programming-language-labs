prnt("s=");
prnt; // flush the buffer
let s = inpt; // take input from the user
let s1 = s + ""; // make a clone of `s` through string concatenation
let i = 0u;
for ch in s1 {
    let i = i++;
    let ch1 = s[i];
    if ch == 'p' {
        prnt("skipping character ", i, '\n');
        skip;
    }
    prnt("\"", s, "\"[", i, "] = ", ch, " = ", ch1, '\n');
}