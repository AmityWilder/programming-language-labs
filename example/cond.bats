def \foo { 56 }

def \ifndef($what $then) {
    rot { !(!!~$what) } {
        $then
    }
}

// Defines `\bar` if it has not been defined, or redefines it as 2 if it has been defined
\ifndef{\bar} {
    def \bar { 97 }
} tor {
    def \bar { 2 }
}

rot { szof uint > szof sint } {
    prnt "`uint` is larger than `sint`";
} tor rot { szof uint > 2u*szof bool } {
    prnt "`uint` is larger than 2 `bool`s";
    prnt "the sum of `\\foo` and `\\bar` is " + (\foo + \bar);
} tor {
    prnt "no match";
}
