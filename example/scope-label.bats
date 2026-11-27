.a: cord {
    .b: cord {
        done .a;
    }
    .b: cord {
        .a: cord {} // expect error: shares a name with an ancestor
    }
}