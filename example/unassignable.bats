let x = "initializer"; // ok
prnt(x + "\n");
x = "post-initializer"; // ok
prnt(x + "\n");
((((((x)))))) = "grouped assignment"; // ok; still assigning to a variable, it's just in parentheses
prnt(x + "\n");
x + 1 = "copy assignment"; // expect: error: cannot assign to immutable
prnt(x + "\n"); // unreachable