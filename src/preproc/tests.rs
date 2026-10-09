use super::*;
use crate::scanner::Scanner;
use std::range::Range;

macro_rules! or_discard {
    () => {
        _
    };

    ($pattern:pat) => {
        $pattern
    };
}

macro_rules! optional {
    () => {
        None
    };

    ($($tokens:tt)+) => {
        Some($($tokens)+)
    };
}

macro_rules! spoof_tokens {
    (
        [$(
            $(@ $marker:ident:)?
            (
                $lex:expr,
                $Variant:ident$(($val:expr))?$(.$SubVariant:ident)? $(,)?
            )
        ),* $(,)?],

        $(let $local:pat = $local_def:expr;)*

        [$(
            $Result:ident(
                // Ok
                $(
                    $(@ $pp_marker:ident:)?
                    (
                        $pp_lex:expr,
                        $PpVariant:ident$(($pp_val:expr))?$(.$PpSubVariant:ident)?
                        $(, in $macro_range:expr $(, as ($idx:expr, $name:expr $(,)?))? $(,)?)?
                    )
                )?
                // Err
                $(ContextError::error($($err:tt)+))?
            )
        ),* $(,)?],

        ($source:ident) $checks:block
    ) => {{
        let $source = concat!($($lex),*);

        let tokens: Vec<Result<Token<'_>, ContextError<'_>>> =
            Scanner::new($source).collect();

        assert_eq!(
            &tokens,
            &[$(Ok(Token {
                lex: $lex,
                val: LexValue::$Variant$(($val))?$(($Variant::$SubVariant))?,
                mac: None,
            })),*],
            "scanner failure",
        );

        let [$(Ok(or_discard!($($marker)?))),*] = tokens.as_slice() else {
            unreachable!("guarded by assert");
        };

        $(let $local = $local_def;)*

        let preproc: Vec<Result<Token<'_>, ContextError<'_>>> =
            preprocess($source, tokens.iter().cloned()).collect();

        assert_eq!(
            &preproc,
            &[$(
                $Result(
                    $(Token {
                        lex: $pp_lex,
                        val: LexValue::$PpVariant$(($pp_val))?$(($PpVariant::$PpSubVariant))?,
                        mac: optional!($(ExpansionData {
                            range: std::range::Range::from($macro_range),
                            arg: optional!($(($idx, $name))?),
                        })?),
                    })?
                    $(ContextError::error($($err)+))?
                )
            ),*],
            "preproc failure",
        );

        let [$(or_discard!($($(Ok($pp_marker))?)?)),*] = preproc.as_slice() else {
            unreachable!("guarded by assert");
        };
        $($($(let $pp_marker = $pp_marker;)?)?)*

        $checks
    }};
}

#[test]
fn test_strip_definitions() {
    spoof_tokens! {
        [
            ("def", Keyword.Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation.LParen),
            (")", Punctuation.RParen),
            (" ", Whitespace),
            ("{", Punctuation.LBrace),
            ("6", SIntLiteral(6)),
            ("}", Punctuation.RBrace),
        ],

        [],

        (source) {}
    };
}

#[test]
fn test_noarg() {
    spoof_tokens! {
        [
            ("def", Keyword.Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation.LParen),
            (")", Punctuation.RParen),
            (" ", Whitespace),
            ("{", Punctuation.LBrace),
            @ body_token: ("6", SIntLiteral(6)),
            ("}", Punctuation.RBrace),
            ("\n", Whitespace),
            @ macro_call: (r"\foo", Macro),
        ],

        [
            Ok(("\n", Whitespace)),
            Ok(@ preproc_body: ("6", SIntLiteral(6), in macro_call.lex_range(source))),
        ],

        (source) {
            assert!(preproc_body.lex_eq(body_token));
        }
    };
}

#[test]
fn test_arg() {
    spoof_tokens! {
        [
            ("def", Keyword.Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation.LParen),
            ("$x", MacroParam),
            (")", Punctuation.RParen),
            (" ", Whitespace),
            ("{", Punctuation.LBrace),
            ("$x", MacroParam),
            ("}", Punctuation.RBrace),
            ("\n", Whitespace),
            @ macro_start: (r"\foo", Macro),
            ("{", Punctuation.LBrace),
            @ arg_token: ("9", SIntLiteral(9)),
            @ macro_end: ("}", Punctuation.RBrace),
        ],

        let expansion_range = Range::from(macro_start.lex_range(source).start..macro_end.lex_range(source).end);

        [
            Ok(("\n", Whitespace)),
            Ok(@ preproc_body: ("9", SIntLiteral(9), in expansion_range, as (0, "$x"))),
        ],

        (source) {
            assert!(preproc_body.lex_eq(arg_token));
        }
    };
}

#[test]
fn test_multiarg() {
    spoof_tokens! {
        [
            ("def", Keyword.Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation.LParen),
            ("$x", MacroParam),
            (",", Punctuation.Comma),
            (" ", Whitespace),
            ("$y", MacroParam),
            (")", Punctuation.RParen),
            (" ", Whitespace),
            ("{", Punctuation.LBrace),
            ("$x", MacroParam),
            ("+", Punctuation.Add),
            ("$y", MacroParam),
            ("}", Punctuation.RBrace),
            ("\n", Whitespace),
            @ macro_start: (r"\foo", Macro),
            ("{", Punctuation.LBrace),
            @ arg1_token: ("a", Identifier),
            ("}", Punctuation.RBrace),
            ("{", Punctuation.LBrace),
            @ arg2_token: ("b", Identifier),
            @ macro_end: ("}", Punctuation.RBrace),
        ],

        let expansion_range = Range {
            start: macro_start.lex_range(source).start,
            end: macro_end.lex_range(source).end,
        };

        [
            Ok(("\n", Whitespace)),
            Ok(@ preproc_arg1: ("a", Identifier, in expansion_range, as (0, "$x"))),
            Ok(("+", Punctuation.Add, in expansion_range)),
            Ok(@ preproc_arg2: ("b", Identifier, in expansion_range, as (1, "$y"))),
        ],

        (source) {
            assert!(preproc_arg1.lex_eq(arg1_token));
            assert!(preproc_arg2.lex_eq(arg2_token));
        }
    };
}

#[test]
fn test_nested() {
    spoof_tokens! {
        [
            ("def", Keyword.Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation.LParen),
            ("$x", MacroParam),
            (",", Punctuation.Comma),
            (" ", Whitespace),
            ("$y", MacroParam),
            (")", Punctuation.RParen),
            (" ", Whitespace),
            ("{", Punctuation.LBrace),
            ("$x", MacroParam),
            ("+", Punctuation.Add),
            ("$y", MacroParam),
            ("}", Punctuation.RBrace),
            ("\n", Whitespace),
            @ macro_start: (r"\foo", Macro),
            ("{", Punctuation.LBrace),
            @ nest_start: (r"\foo", Macro),
            ("{", Punctuation.LBrace),
            @ arg1_token: ("a", Identifier),
            ("}", Punctuation.RBrace),
            ("{", Punctuation.LBrace),
            @ arg2_token: ("b", Identifier),
            @ nest_end: ("}", Punctuation.RBrace),
            ("}", Punctuation.RBrace),
            ("{", Punctuation.LBrace),
            @ arg3_token: ("c", Identifier),
            @ macro_end: ("}", Punctuation.RBrace),
        ],

        let expansion_range = Range {
            start: macro_start.lex_range(source).start,
            end: macro_end.lex_range(source).end,
        };
        let nested_range = Range {
            start: nest_start.lex_range(source).start,
            end: nest_end.lex_range(source).end,
        };

        [
            Ok(("\n", Whitespace)),
            Ok(@ preproc_arg1: ("a", Identifier, in nested_range, as (0, "$x"))),
            Ok(("+", Punctuation.Add, in nested_range)),
            Ok(@ preproc_arg2: ("b", Identifier, in nested_range, as (1, "$y"))),
            Ok(("+", Punctuation.Add, in expansion_range)),
            Ok(@ preproc_arg3: ("c", Identifier, in expansion_range, as (1, "$y"))),
        ],

        (source) {
            assert!(preproc_arg1.lex_eq(arg1_token));
            assert!(preproc_arg2.lex_eq(arg2_token));
            assert!(preproc_arg3.lex_eq(arg3_token));
        }
    };
}
