.a: cord {
    .b: cord {
        done .a;
    }
    .b: cord {
        .a: cord {} // expect warning: shares a name with an ancestor
    }
}