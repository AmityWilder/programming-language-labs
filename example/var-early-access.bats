prnt(x); // expect "unknown variable" error
let x = 3;

{
    let y = 5;
}
prnt(y); // expect "unknown variable" error - `y` has popped off the stack