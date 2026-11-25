let x;
if inpt == "hi" {
    x = "hello";
}
prnt(x); // expect: error: `inpt == "hi"` may be `fals`, then `x` would be uninitialized