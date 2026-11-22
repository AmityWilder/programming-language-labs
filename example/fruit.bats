mod apples;

fun is_valid(color: text) -> bool {
    give color == "red" | color == "yellow" | color == "green";
}

fun main() {
    prnt(fruit::apples::NUMBER + "\n");
    prnt("name a fruit\n");
    cord {
        let fruit = inpt;
        if fruit == "apple" {
            prnt("what color?\n");
            let color;
            cord {
                color = inpt;
                if !is_valid(color) {
                    prnt("I don't know any " + color + " apples\n");
                    skip;
                }
            }
            prnt(color + " ");
        } or if fruit == "grapefruit" {
            prnt("try again\n");
            skip;
        }
        prnt(fruit + "? yum!\n");
    }
}
