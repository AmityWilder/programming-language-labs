prnt("s=");
prnt; // flush the buffer
let s = inpt; // take input from the user
let s1 = s + "";
let i = 0;
loop {
    let ch = s1++;
    if !!ch {
        prnt(s + "[" + i++ + "] = " + ch + "\n");
    } or {
        done;
    }
}