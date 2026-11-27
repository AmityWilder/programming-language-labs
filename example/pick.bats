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
// adjacent
pick x {
    8 => { prnt("apple"); }
    9 => { prnt("banana"); }
    10 => { prnt("mango"); }
    11 => { prnt("pineapple"); }
}

let x = inpt;
// sparse - NOT YET IMPLEMENTED
pick x {
    0 => { prnt("apple"); }
    1 => { prnt("banana"); }
    2 => { prnt("mango"); }
    //4 => { prnt("pineapple"); }
}