prnt("s=");
prnt; // flush the buffer
let s = inpt; // take input from the user
rep !!s {
    let word = "";
    for ch in s {
        if ch == ' ' { done; }
        word += ch;
    }
    prnt(word, '\n');
}