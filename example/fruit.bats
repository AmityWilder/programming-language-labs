mod apples;

{
    prnt("name a fruit\n");
    cord {
        let fruit = inpt;
        if fruit == "apple" {
            prnt("what color?\n");
            let color;
            cord {
                color = inpt;
                let apple = Apple { color };
                if is_valid(apple) {
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
