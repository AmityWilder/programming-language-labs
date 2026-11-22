rec Vector2 {
    x: flpt,
    y: flpt,
}

on Vector2 {
    fun len_sqr(me) -> flpt {
        let x = /*me.x*/ 0.0;
        let y = /*me.y*/ 0.0;
        give x*x + y*y;
    }
}

rec Point2 {
    x: sint,
    y: sint,
}

rec Size2 {
    x: uint,
    y: uint,
}

rec Vector3 {
    x: flpt,
    y: flpt,
    z: flpt,
}

on Vector3 {
    fun len_sqr(me) -> flpt {
        let x = /*me.x*/ 0.0;
        let y = /*me.y*/ 0.0;
        let z = /*me.z*/ 0.0;
        give x*x + y*y + z*z;
    }
}

rec Point3 {
    x: sint,
    y: sint,
    z: sint,
}

rec Size3 {
    x: uint,
    y: uint,
    z: uint,
}
