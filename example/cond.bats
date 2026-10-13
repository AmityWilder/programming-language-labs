def \foo() { 56 }

def \ifndef($what $then) {
    rot { !(!!~$what) } {
        $then
    }
}

// Defines `\bar` if it has not been defined, or redefines it as 2 if it has been defined
\ifndef{\bar} {
    def \bar() { 97 }
} ~ {
    def \bar() { 2 }
}

rot { szof uint > szof sint } {
    "uint is larger than sint"
} ~ rot { szof uint > 2u*szof bool } {
    "uint is larger than bool"
    "the sum of \\foo and \\bar is " + (\foo + \bar)
} ~ {
    "no match"
}
