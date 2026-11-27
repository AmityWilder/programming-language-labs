// TODO: replace this with a `cat`
let INVALID: byte = 0b;
let WSPACE: byte = 1b;
let COMMENT: byte = 2b;
let IDENT: byte = 3b;
let KEYWORD: byte = 4b;
let NUMBER: byte = 5b;
let STRING: byte = 6b;
let PUNC: byte = 7b;

prnt("paste program code\n");
let code = inpt; // take input from the user
let copy = code + "";
rep !!copy {
    let first = copy++;
    let kind;
    let lexeme = "" + first;
    // assertion
    // TODO: make this a macro once macro locations have better tracing
    if !(!!first) { fail "`!!copy` should ensure `copy++` will have an output"; }
    if first == ' ' | first == '\n' {
        kind = WSPACE;
    } or if first == '_' | first >= 'a' & first <= 'z' | first >= 'A' & first <= 'Z' {
        kind = IDENT;
        // TODO: this consumes the character, it should peek instead
        for ch in copy {
            if ch == '_' | ch >= 'a' & ch <= 'z' | ch >= 'A' & ch <= 'Z' | ch >= '0' & ch <= '9' {
                lexeme += ch;
            } or {
                done;
            }
        }
        if lexeme == "let" | lexeme == "print" | lexeme == "if" | lexeme == "else" {
            kind = KEYWORD;
        }
    } or if first >= '0' & first <= '9' {
        kind = NUMBER;
        // TODO: this consumes the character, it should peek instead
        for ch in copy {
            if ch == '_' | ch >= 'a' & ch <= 'z' | ch >= 'A' & ch <= 'Z' | ch >= '0' & ch <= '9' {
                lexeme += ch;
            } or {
                done;
            }
        }
    } or if first == '"' {
        kind = STRING;
        for ch in copy {
            lexeme += ch;
            if ch == '"' { done; }
        }
    } or if first == ';' | first == '='
        | first == '+' | first == '-' | first == '*' | first == ','
        | first == '!' | first == '|' | first == '&' | first == '^'
        | first == '>' | first == '<' | first == '{' | first == '}'
        | first == '[' | first == ']' | first == '(' | first == ')'
    {
        kind = PUNC;
    } or {
        kind = INVALID;
    }
    let kind_name;
    // TODO: replace this with a `pick` statement
    if kind == INVALID { kind_name = "\x1b[91minvalid\x1b[0m"; } or
    if kind == WSPACE { kind_name = "\x1b[90mwhitespace\x1b[0m"; } or
    if kind == COMMENT { kind_name = "\x1b[90mcomment\x1b[0m"; } or
    if kind == IDENT { kind_name = "\x1b[96midentifier\x1b[0m"; } or
    if kind == KEYWORD { kind_name = "\x1b[94mkeyword\x1b[0m"; } or
    if kind == NUMBER { kind_name = "\x1b[92mnumber literal\x1b[0m"; } or
    if kind == STRING { kind_name = "\x1b[93mstring literal\x1b[0m"; } or
    if kind == PUNC { kind_name = "punctuation"; }
    prnt("\"", lexeme, "\": ", kind_name, '\n');
}