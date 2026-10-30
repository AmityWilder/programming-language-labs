use super::*;
use crate::{
    error::ExpectedToken,
    scanner::{
        Bracket, Scanner,
        token::value::{Keyword, LexValue, Punctuation},
    },
};
use std::range::Range;

/// Outputs the provided pattern, or a `_` if no pattern is present.
///
/// Intended for use with optional (`$()?`) patterns.
macro_rules! or_discard {
    () => {
        _
    };

    ($pattern:pat) => {
        $pattern
    };
}

/// Outputs the provided tokens wrapped in a [`Some`], or a [`None`] if no tokens are present.
///
/// Intended for use with optional (`$()?` or `$()*`) patterns.
macro_rules! optional {
    () => {
        None
    };

    ($($tokens:tt)+) => {
        Some($($tokens)+)
    };
}

/// 1. Concatenates the lexemes of the first array of tokens and then asserts that a scanner will produce them.
/// 2. Defines any requested local variables.
/// 3. Calls [`preprocess`] on the token stream, asserting the output is equal to the second array of tokens.
/// 4. Finally, performs any additional assertions provided after the second array.
///
/// Tokens may be preceded with `@ name:` to create a binding to that token, for aid in creating assertions.
///
/// ```no_run
/// spoof_tokens! {
///     [/* tokens for scanner */]
///
///     /* local variables */
///
///     [/* expected preprocessor result */]
///
///     (/* binding for source code variable */) {
///         /* extra assertions */
///     }
/// }
/// ```
macro_rules! spoof_tokens {
    (
        [$(
            $(@ $marker:ident:)?
            (
                $lex:expr,
                $Variant:ident$(($val:expr))?$(::$SubVariant:ident)? $(,)?
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
                        $PpVariant:ident$(($pp_val:expr))?$(::$PpSubVariant:ident)?
                        $(, in $macro_range:expr $(, as ($idx:expr, $name:expr $(,)?))? $(,)?)?
                    )
                )?
                // Err
                $(ContextError::error($($err:tt)+))?
            )
        ),* $(,)?],

        ($source:ident) $checks:block
    ) => {{
        #[allow(unused_imports, reason = "only here to enable proper semantic highlighting")] {
            $($({ use $crate::scanner::token::value::$Variant::$SubVariant; })?)*
            $({ use $crate::scanner::token::value::LexValue::$Variant; })*
            $({ use std::result::Result::$Result; })*
        }

        let $source = concat!($($lex),*);

        let tokens: Vec<Result<Token<'_>, ContextError<'_>>> =
            Scanner::new($source).collect();

        let expected = [$(Ok(Token {
            lex: $crate::scanner::token::Lexeme {
                src: $lex,
                mac: None,
            },
            val: LexValue::$Variant$(($val))?$(($Variant::$SubVariant))?,
        })),*];

        for (i, (actual, expected)) in tokens.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                actual,
                expected,
                "scanner failure at item {i}\n  left list: {tokens:#?}\n right list: {expected:#?}",
            );
        }
        assert_eq!(
            tokens.len(),
            expected.len(),
            "scanner failure: token count mismatch\n  left list: {tokens:#?}\n right list: {expected:#?}",
        );

        let [$(Ok(or_discard!($($marker)?))),*] = tokens.as_slice() else {
            unreachable!("guarded by assert");
        };

        $(let $local = $local_def;)*

        let preproc: Vec<Result<Token<'_>, ContextError<'_>>> =
            preprocess($source, tokens.iter().cloned()).collect();

        let expected = [$(
            $Result(
                $(Token {
                    lex: $crate::scanner::token::Lexeme {
                        src: $pp_lex,
                        mac: optional!($(ExpansionData {
                            range: std::range::Range::from($macro_range),
                            arg: optional!($(($idx, $name))?),
                        })?),
                    },
                    val: LexValue::$PpVariant$(($pp_val))?$(($PpVariant::$PpSubVariant))?,
                })?
                $(ContextError::error($($err)+))?
            )
        ),*];

        for (i, (actual, expect)) in preproc.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                actual,
                expect,
                "preproc failure at item {i}\n  left list: {preproc:#?}\n right list: {expected:#?}",
            );
        }
        assert_eq!(
            preproc.len(),
            expected.len(),
            "preproc failure: token count mismatch\n  left list: {preproc:#?}\n right list: {expected:#?}",
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
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("6", SIntLiteral(6)),
            ("}", Punctuation::RBrace),
        ],

        [],

        (source) {}
    };
}

#[test]
fn test_noarg() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            @ body_token: ("6", SIntLiteral(6)),
            ("}", Punctuation::RBrace),
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
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            @ macro_start: (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            @ arg_token: ("9", SIntLiteral(9)),
            @ macro_end: ("}", Punctuation::RBrace),
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
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (" ", Whitespace),
            ("$y", MacroParam),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("+", Punctuation::Add),
            ("$y", MacroParam),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            @ macro_start: (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            @ arg1_token: ("a", Identifier),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            @ arg2_token: ("b", Identifier),
            @ macro_end: ("}", Punctuation::RBrace),
        ],

        let expansion_range = Range {
            start: macro_start.lex_range(source).start,
            end: macro_end.lex_range(source).end,
        };

        [
            Ok(("\n", Whitespace)),
            Ok(@ preproc_arg1: ("a", Identifier, in expansion_range, as (0, "$x"))),
            Ok(("+", Punctuation::Add, in expansion_range)),
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
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (" ", Whitespace),
            ("$y", MacroParam),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("+", Punctuation::Add),
            ("$y", MacroParam),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            @ macro_start: (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            @ nest_start: (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            @ arg1_token: ("a", Identifier),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            @ arg2_token: ("b", Identifier),
            @ nest_end: ("}", Punctuation::RBrace),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            @ arg3_token: ("c", Identifier),
            @ macro_end: ("}", Punctuation::RBrace),
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
            Ok(("+", Punctuation::Add, in nested_range)),
            Ok(@ preproc_arg2: ("b", Identifier, in nested_range, as (1, "$y"))),
            Ok(("+", Punctuation::Add, in expansion_range)),
            Ok(@ preproc_arg3: ("c", Identifier, in expansion_range, as (1, "$y"))),
        ],

        (source) {
            assert!(preproc_arg1.lex_eq(arg1_token));
            assert!(preproc_arg2.lex_eq(arg2_token));
            assert!(preproc_arg3.lex_eq(arg3_token));
        }
    };
}

#[test]
fn test_no_param_group() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            @head: (r"\foo", Macro),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            @x: ("$x", MacroParam),
            ("+", Punctuation::Add),
            @y: ("$y", MacroParam),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            @macro_call: (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            ("a", Identifier),
            ("}", Punctuation::RBrace),
            ("+", Punctuation::Add),
            ("5", SIntLiteral(5)),
        ],

        [
            Err(ContextError::error(
                source,
                Some(x.lex_range(source)),
                None,
                ErrorType::MacroParamUndefined {
                    head_range: Some(head.lex_range(source)),
                    param_name: "$x",
                },
            )),
            Err(ContextError::error(
                source,
                Some(y.lex_range(source)),
                None,
                ErrorType::MacroParamUndefined {
                    head_range: Some(head.lex_range(source)),
                    param_name: "$y",
                },
            )),
            Ok(("\n", Whitespace)),
            Err(ContextError::error(
                source,
                Some(macro_call.lex_range(source)),
                None,
                ErrorType::MacroUndefined,
            )),
            Ok(("{", Punctuation::LBrace)),
            Ok(("a", Identifier)),
            Ok(("}", Punctuation::RBrace)),
            Ok(("+", Punctuation::Add)),
            Ok(("5", SIntLiteral(5))),
        ],

        (source) {}
    };
}

#[test]
fn test_endless_param_group() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            @open: ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (" ", Whitespace),
            ("$y", MacroParam),
            (" ", Whitespace),
            @unexpected: ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("+", Punctuation::Add),
            ("$y", MacroParam),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            @macro_call: (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            ("a", Identifier),
            ("}", Punctuation::RBrace),
            ("+", Punctuation::Add),
            ("5", SIntLiteral(5)),
        ],

        [
            Err(ContextError::error(
                source,
                Some(unexpected.lex_range(source)),
                None,
                ErrorType::MissingCloseBracket {
                    open_range: open.lex_range(source),
                    expect: Bracket::Paren,
                },
            )),
            Ok((" ", Whitespace)),
            Ok(("{", Punctuation::LBrace)),
            Ok(("$x", MacroParam)),
            Ok(("+", Punctuation::Add)),
            Ok(("$y", MacroParam)),
            Ok(("}", Punctuation::RBrace)),
            Ok(("\n", Whitespace)),
            Err(ContextError::error(
                source,
                Some(macro_call.lex_range(source)),
                None,
                ErrorType::MacroUndefined,
            )),
            Ok(("{", Punctuation::LBrace)),
            Ok(("a", Identifier)),
            Ok(("}", Punctuation::RBrace)),
            Ok(("+", Punctuation::Add)),
            Ok(("5", SIntLiteral(5))),
        ],

        (source) {}
    };
}

#[test]
fn test_endless_body() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (" ", Whitespace),
            ("$y", MacroParam),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            @open: ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("+", Punctuation::Add),
            ("$y", MacroParam),
            ("\n", Whitespace),
            (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            ("a", Identifier),
            ("}", Punctuation::RBrace),
            ("+", Punctuation::Add),
            ("5", SIntLiteral(5)),
        ],

        [
            Err(ContextError::error(
                source,
                None,
                None,
                ErrorType::MissingCloseBracket {
                    open_range: open.lex_range(source),
                    expect: Bracket::Brace,
                },
            )),
        ],

        (source) {}
    };
}

#[test]
fn test_endless_arg() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (" ", Whitespace),
            ("$y", MacroParam),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("+", Punctuation::Add),
            ("$y", MacroParam),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            (r"\foo", Macro),
            @open: ("{", Punctuation::LBrace),
            ("a", Identifier),
            ("+", Punctuation::Add),
            ("5", SIntLiteral(5)),
        ],

        [
            Ok(("\n", Whitespace)),
            Err(ContextError::error(
                source,
                None,
                None,
                ErrorType::MissingCloseBracket {
                    open_range: open.lex_range(source),
                    expect: Bracket::Brace,
                },
            )),
        ],

        (source) {}
    };
}

#[test]
fn test_bad_arg_count() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (" ", Whitespace),
            ("$y", MacroParam),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("+", Punctuation::Add),
            ("$y", MacroParam),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            ("a", Identifier),
            ("}", Punctuation::RBrace),
            @ unexpected: ("+", Punctuation::Add),
            ("5", SIntLiteral(5)),
        ],

        [
            Ok(("\n", Whitespace)),
            Err(ContextError::error(
                source,
                Some(unexpected.lex_range(source)),
                None,
                ErrorType::UnexpectedToken { expect: ExpectedToken::MacroArgLBrace, actual: "+" },
            )),
            Ok(("+", Punctuation::Add)),
            Ok(("5", SIntLiteral(5))),
        ],

        (source) {}
    };
}

#[test]
fn test_recursive_macro() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            @macro_name: (r"\foo", Macro),
            ("(", Punctuation::LParen),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            @recursive_call: (r"\foo", Macro),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            @normal_call: (r"\foo", Macro),
        ],

        [
            Err(ContextError::error(
                source,
                Some(recursive_call.lex_range(source)),
                None,
                ErrorType::RecursiveMacro {
                    head_range: macro_name.lex_range(source),
                },
            )),
            Ok(("\n", Whitespace)),
            Err(ContextError::error(
                source,
                Some(normal_call.lex_range(source)),
                None,
                ErrorType::MacroUndefined,
            )),
        ],

        (source) {}
    };
}

#[test]
fn test_multi_recursive_macro() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            @macro_name: (r"\foo", Macro),
            ("(", Punctuation::LParen),
            (")", Punctuation::RParen),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            @recursive_call1: (r"\foo", Macro),
            ("+", Punctuation::Add),
            @recursive_call2: (r"\foo", Macro),
            ("}", Punctuation::RBrace),
            ("\n", Whitespace),
            @normal_call: (r"\foo", Macro),
        ],

        [
            Err(ContextError::error(
                source,
                Some(recursive_call1.lex_range(source)),
                None,
                ErrorType::RecursiveMacro {
                    head_range: macro_name.lex_range(source),
                },
            )),
            Err(ContextError::error(
                source,
                Some(recursive_call2.lex_range(source)),
                None,
                ErrorType::RecursiveMacro {
                    head_range: macro_name.lex_range(source),
                },
            )),
            Ok(("\n", Whitespace)),
            Err(ContextError::error(
                source,
                Some(normal_call.lex_range(source)),
                None,
                ErrorType::MacroUndefined,
            )),
        ],

        (source) {}
    };
}

#[test]
fn test_conditional_skip() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("fals", BoolLiteral(false)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
        ],

        [],

        (source) {}
    };
}

#[test]
fn test_conditional_keep() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("true", BoolLiteral(true)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("5", SIntLiteral(5)))
        ],

        (source) {}
    };
}

#[test]
fn test_conditional_else0() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("fals", BoolLiteral(false)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("8", SIntLiteral(8)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("8", SIntLiteral(8)))
        ],

        (source) {}
    };
}

#[test]
fn test_conditional_else1() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("true", BoolLiteral(true)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("8", SIntLiteral(8)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("5", SIntLiteral(5)))
        ],

        (source) {}
    };
}

#[test]
fn test_conditional_elseif0() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("fals", BoolLiteral(false)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            (" ", Whitespace),
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("fals", BoolLiteral(false)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("8", SIntLiteral(8)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("3", SIntLiteral(3)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("3", SIntLiteral(3)))
        ],

        (source) {}
    };
}

#[test]
fn test_conditional_elseif1() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("fals", BoolLiteral(false)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            (" ", Whitespace),
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("true", BoolLiteral(true)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("8", SIntLiteral(8)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("3", SIntLiteral(3)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("8", SIntLiteral(8)))
        ],

        (source) {}
    };
}

#[test]
fn test_conditional_elseif_skip_elses() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            (" ", Whitespace),
            ("{", Punctuation::LBrace),
            ("true", BoolLiteral(true)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            (" ", Whitespace),
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("true", BoolLiteral(true)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("8", SIntLiteral(8)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("3", SIntLiteral(3)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("5", SIntLiteral(5)))
        ],

        (source) {}
    };
}

#[test]
fn test_macro_const() {
    spoof_tokens! {
        [
            ("~", Punctuation::MacroConst),
            (r"\foo", Macro),
        ],

        [
            Ok((r"\foo", Macro)),
        ],

        (source) {}
    };
}

#[test]
fn test_macro_const_nested() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            (")", Punctuation::RParen),
            ("{", Punctuation::LBrace),
            ("~", Punctuation::MacroConst),
            (r"\bar", Macro),
            ("}", Punctuation::RBrace),
            @expansion: (r"\foo", Macro),
        ],

        [
            Ok((r"\bar", Macro, in expansion.lex_range(source))),
        ],

        (source) {}
    };
}

/// Confirms macros in arguments aren't expanded until after the macro they are arguments to expands
#[test]
fn test_macro_const_late() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (")", Punctuation::RParen),
            ("{", Punctuation::LBrace),
            ("~", Punctuation::MacroConst),
            ("$x", MacroParam),
            ("}", Punctuation::RBrace),
            @expansion_start: (r"\foo", Macro),
            ("{", Punctuation::LBrace),
            (r"\bar", Macro),
            @expansion_end: ("}", Punctuation::RBrace),
        ],

        let expansion_range = Range {
            start: expansion_start.lex_range(source).start,
            end: expansion_end.lex_range(source).end,
        };

        [
            Ok((r"\bar", Macro, in expansion_range, as (0, "$x"))),
        ],

        (source) {}
    };
}

#[test]
fn test_macro_expr_cond() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("1", SIntLiteral(1)),
            ("+", Punctuation::Add),
            ("1", SIntLiteral(1)),
            ("<", Punctuation::Lt),
            ("5", SIntLiteral(5)),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("9", SIntLiteral(9)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("2", SIntLiteral(2)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("9", SIntLiteral(9))),
        ],

        (source) {}
    };
}

#[test]
fn test_macro_ifdef_false() {
    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("!!", Punctuation::Exists),
            ("~", Punctuation::MacroConst),
            (r"\foo", Macro),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("9", SIntLiteral(9)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("2", SIntLiteral(2)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("2", SIntLiteral(2))),
        ],

        (source) {}
    };
}

#[test]
fn test_macro_ifdef_true() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            (")", Punctuation::RParen),
            ("{", Punctuation::LBrace),
            ("}", Punctuation::RBrace),
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("!!", Punctuation::Exists),
            ("~", Punctuation::MacroConst),
            (r"\foo", Macro),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("9", SIntLiteral(9)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("2", SIntLiteral(2)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("9", SIntLiteral(9))),
        ],

        (source) {}
    };
}

#[test]
fn test_macro_ifdef_true_arg() {
    spoof_tokens! {
        [
            ("def", Keyword::Def),
            (" ", Whitespace),
            (r"\foo", Macro),
            ("(", Punctuation::LParen),
            ("$x", MacroParam),
            (")", Punctuation::RParen),
            ("{", Punctuation::LBrace),
            ("$x", MacroParam),
            ("}", Punctuation::RBrace),
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("!!", Punctuation::Exists),
            ("~", Punctuation::MacroConst),
            (r"\foo", Macro),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("9", SIntLiteral(9)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("2", SIntLiteral(2)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("9", SIntLiteral(9))),
        ],

        (source) {}
    };
}

#[test]
fn test_macro_ifdef_type_size() {
    const {
        assert!(
            std::mem::size_of::<usize>() >= std::mem::size_of::<bool>(),
            "ensure the test is actually meaningful"
        );
    }

    spoof_tokens! {
        [
            ("rot", Keyword::Rot),
            ("{", Punctuation::LBrace),
            ("szof", Keyword::Szof),
            (" ", Whitespace),
            ("uint", Keyword::Uint),
            ("<", Punctuation::Lt),
            ("szof", Keyword::Szof),
            (" ", Whitespace),
            ("bool", Keyword::Bool),
            ("}", Punctuation::RBrace),
            ("{", Punctuation::LBrace),
            ("9", SIntLiteral(9)),
            ("}", Punctuation::RBrace),
            ("tor", Keyword::Tor),
            ("{", Punctuation::LBrace),
            ("2", SIntLiteral(2)),
            ("}", Punctuation::RBrace),
        ],

        [
            Ok(("2", SIntLiteral(2))),
        ],

        (source) {}
    };
}
