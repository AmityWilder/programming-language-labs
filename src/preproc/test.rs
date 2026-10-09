use super::*;
use crate::scanner::Scanner;
use std::range::Range;

macro_rules! ident_or_discard {
    ($ident:ident) => {
        $ident
    };
    () => {
        _
    };
}

macro_rules! spoof_tokens {
    (
        $source:ident:
        [$( $(@ $marker:ident:)? ($lex:expr, $Variant:ident$(($val:expr))?)),* $(,)?],
        [$( $(@ $pp_marker:ident:)? $item:expr),* $(,)?],
        $checks:expr
    ) => {{
        let $source = concat!($($lex),*);

        let tokens: Vec<Result<Token<'_>, ContextError<'_>>> =
            Scanner::new($source).collect();

        assert_eq!(
            &tokens,
            &[$(Ok(Token {
                lex: $lex,
                val: LexValue::$Variant$(($val))?,
                mac: None,
            })),*],
            "scanner failure"
        );

        let [$(Ok(ident_or_discard!($($marker)?))),*] = tokens.as_slice() else {
            unreachable!("guarded by assert")
        };

        let preproc: Vec<Result<Token<'_>, ContextError<'_>>> =
            preprocess($source, tokens.iter().cloned()).collect();

        assert_eq!(&preproc, &[$($item),*], "preproc failure");

        let [$(ident_or_discard!($($pp_marker)?)),*] = preproc.as_slice() else {
            unreachable!("guarded by assert")
        };
        $($(let $pp_marker = $pp_marker.as_ref().expect("why do you need a marker on an error?");)?)*

        $checks
    }};
}

#[test]
fn test_strip_definitions() {
    spoof_tokens! {
        source:
        [
            ("def", Keyword(Keyword::Def)),
            (" ", Whitespace),
            ("\\foo", Macro),
            ("(", Punctuation(Punctuation::LParen)),
            (")", Punctuation(Punctuation::RParen)),
            (" ", Whitespace),
            ("{", Punctuation(Punctuation::LBrace)),
            ("6", SIntLiteral(6)),
            ("}", Punctuation(Punctuation::RBrace)),
        ],

        [],

        {}
    };
}

#[test]
fn test_same_lexeme() {
    spoof_tokens! {
        source:
        [
            ("def", Keyword(Keyword::Def)),
            (" ", Whitespace),
            ("\\foo", Macro),
            ("(", Punctuation(Punctuation::LParen)),
            (")", Punctuation(Punctuation::RParen)),
            (" ", Whitespace),
            ("{", Punctuation(Punctuation::LBrace)),
            @ body_token: ("6", SIntLiteral(6)),
            ("}", Punctuation(Punctuation::RBrace)),
            ("\n", Whitespace),
            @ macro_call: ("\\foo", Macro),
        ],

        [
            Ok(Token { lex: "\n", val: LexValue::Whitespace, mac: None }),
            @ preproc_body: Ok(Token {
                lex: "6",
                val: LexValue::SIntLiteral(6),
                mac: Some(ExpansionData {
                    range: macro_call.lex_range(source),
                    arg: None,
                })
            }),
        ],

        {
            assert!(preproc_body.lex_eq(body_token));
        }
    };
}

#[test]
fn test_arg_same_lexeme() {
    spoof_tokens! {
        source:
        [
            ("def", Keyword(Keyword::Def)),
            (" ", Whitespace),
            ("\\foo", Macro),
            ("(", Punctuation(Punctuation::LParen)),
            ("$x", MacroParam),
            (")", Punctuation(Punctuation::RParen)),
            (" ", Whitespace),
            ("{", Punctuation(Punctuation::LBrace)),
            ("$x", MacroParam),
            ("}", Punctuation(Punctuation::RBrace)),
            ("\n", Whitespace),
            @ macro_start: ("\\foo", Macro),
            ("{", Punctuation(Punctuation::LBrace)),
            @ body_token: ("9", SIntLiteral(9)),
            @ macro_end: ("}", Punctuation(Punctuation::RBrace)),
        ],

        [
            Ok(Token { lex: "\n", val: LexValue::Whitespace, mac: None }),
            @ preproc_body: Ok(Token {
                lex: "9",
                val: LexValue::SIntLiteral(9),
                mac: Some(ExpansionData {
                    range: Range {
                        start: macro_start.lex_range(source).start,
                        end: macro_end.lex_range(source).end,
                    },
                    arg: Some((0, "$x")),
                })
            }),
        ],

        {
            assert!(preproc_body.lex_eq(body_token));
        }
    };
}
