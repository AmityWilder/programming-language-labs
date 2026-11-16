fun is_valid(color: text) -> bool {
    give color == "red" | color == "yellow" | color == "green";
}

fun foo<T, U>(x: T) -> U
<=>
    T: only sint + for u: u >= 5,
    U: only sint + for v: v <= !0 - 5,
{
    give u - 5;
}