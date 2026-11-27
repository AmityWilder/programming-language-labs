let x = inpt;
// bools
pick x {
    true => { prnt("apple"); }
    fals => { prnt("banana"); }
}

let x = inpt;
// strings
pick x {
    "hello" => { prnt("hi!"); }
    "hi" => { prnt("hello!"); }
    "hi" => { prnt("hey!"); }
}

let x = inpt;
// adjacent
pick x {
    0 => { prnt("apple"); }
    1 => { prnt("banana"); }
    2 => { prnt("mango"); }
    3 => { prnt("pineapple"); }
}

let x = inpt;
// adjacent, but out of order and not starting at 0
pick x {
    12 => { prnt("apple"); }
    10 => { prnt("banana"); }
    8 => { prnt("strawberry"); }
    11 => { prnt("raspberry"); }
    9 => { prnt("pineapple"); }
    13 => { prnt("mango"); }
}

let x = inpt;
// sparse - NOT YET IMPLEMENTED
pick x {
    0 => { prnt("apple"); }
    1 => { prnt("banana"); }
    2 => { prnt("mango"); }
    //4 => { prnt("pineapple"); }
}