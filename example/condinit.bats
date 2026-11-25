let x;
if 5 > 3 {
    x = "always";
}
prnt(x + '\n'); // expect: `5 > 3` is always true, so `x` will be initialized

let y;
if 5 < 3 {
    y = "never";
}
prnt(y + '\n'); // expect: error: `5 < 3` is never true, so `y` will be uninitialized

let z;
if inpt == "hi" {
    z = "hello";
}
prnt(z + '\n'); // expect: error: `inpt == "hi"` may be `fals`, then `z` would be uninitialized