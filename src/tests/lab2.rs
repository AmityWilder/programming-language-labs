//! AST Printer

use crate::grammar::fmt::lisp::Lisp;

macro_rules! token {
    // Special cases where the lex is in a field instead of being the entire value
    (CharLiteral($ch:expr, $is_esc:expr) $(,)?) => {
        $crate::scanner::token::Token {
            lex: stringify!($ch),
            val: $crate::scanner::token::value::LexValue::CharLiteral(
                $crate::scanner::token::value::CharLiteral { ch: $ch, is_escaped: $is_esc }),
            mac: None
        }
    };
    (TextLiteral($content:expr) $(,)?) => {
        $crate::scanner::token::Token {
            lex: stringify!($content),
            val: $crate::scanner::token::value::LexValue::TextLiteral(
                $crate::scanner::token::value::TextLiteral { content: $content }),
            mac: None
        }
    };

    // Special cases where the value is already stringified
    (Identifier($name:expr)) => {
        $crate::scanner::token::Token {
            lex: $name,
            val: $crate::scanner::token::value::LexValue::Identifier,
            mac: None
        }
    };
    // TODO: to be deprecated
    (Callable($name:expr)) => {
        $crate::scanner::token::Token {
            lex: $name,
            val: $crate::scanner::token::value::LexValue::Callable,
            mac: None
        }
    };
    (Macro($name:expr)) => {
        $crate::scanner::token::Token {
            lex: $name,
            val: $crate::scanner::token::value::LexValue::Macro,
            mac: None
        }
    };
    (MacroParam($name:expr)) => {
        $crate::scanner::token::Token {
            lex: $name,
            val: $crate::scanner::token::value::LexValue::MacroParam,
            mac: None
        }
    };

    ($Variant:ident$(($val:expr))? $(::$SubVariant:ident)? $(, $name:expr)? $(,)?) => {
        $crate::scanner::token::Token {
            lex: $(stringify!($val))? $($crate::scanner::token::value::$Variant::$SubVariant.as_str())? $($name)?,
            val: $crate::scanner::token::value::LexValue::$Variant $(($val))? $(($crate::scanner::token::value::$Variant::$SubVariant))?,
            mac: None
        }
    };
}

macro_rules! syntax_tree {
    (Ternary(
        $LhsTy:ident($($lhs:tt)*),
        Token($($lop:tt)*),
        $MhsTy:ident($($mhs:tt)*),
        Token($($rop:tt)*),
        $RhsTy:ident($($rhs:tt)*)$(,)?
    )) => {
        $crate::grammar::ast::Expr::ternary($crate::grammar::ast::Ternary {
            lhs: syntax_tree!($LhsTy($($lhs)*)),
            lop: token!($($lop)*),
            mhs: syntax_tree!($MhsTy($($mhs)*)),
            rop: token!($($rop)*),
            rhs: syntax_tree!($RhsTy($($rhs)*)),
        })
    };

    (Binary(
        $LhsTy:ident($($lhs:tt)*),
        Token($($op:tt)*),
        $RhsTy:ident($($rhs:tt)*)$(,)?
    )) => {
        $crate::grammar::ast::Expr::binary($crate::grammar::ast::Binary {
            lhs: syntax_tree!($LhsTy($($lhs)*)),
            op: token!($($op)*),
            rhs: syntax_tree!($RhsTy($($rhs)*)),
        })
    };

    // Prefix
    (Unary(
        Token($($op:tt)*),
        $RhsTy:ident($($rhs:tt)*)$(,)?
    )) => {
        $crate::grammar::ast::Expr::unary($crate::grammar::ast::Unary {
            op: token!($($op)*),
            operand: syntax_tree!($RhsTy($($rhs)*)),
            side: $crate::error::OpSide::Right,
        })
    };

    // Postfix
    (Unary(
        $LhsTy:ident($($lhs:tt)*),
        Token($($op:tt)*)$(,)?
    )) => {
        $crate::grammar::ast::Expr::unary($crate::grammar::ast::Unary {
            operand: syntax_tree!($LhsTy($($lhs)*)),
            op: token!($($op)*),
            side: $crate::error::OpSide::Left,
        })
    };

    (Literal($($token:tt)*)) => {
        $crate::grammar::ast::Expr::literal(token!($($token)*))
    };

    (Grouping(
        $ExprTy:ident($($expr:tt)*)$(,)?
    )) => {
        $crate::grammar::ast::Expr::grouping($crate::grammar::ast::Grouping {
            open: token!(Punctuation::LParen),
            expr: syntax_tree!($ExprTy($($expr)*)),
            close: token!(Punctuation::RParen),
        })
    };

    (Type(
        Token($($name:tt)*)$(,)?
    )) => {
        $crate::grammar::ast::Expr::type_expr($crate::grammar::ast::TypeExpr {
            name: token!($($name)*),
            or_ty: None,
        })
    };

    (Type(
        Token($($name:tt)*),
        Token($($pipe:tt)*),
        Token($($or_ty:tt)*)$(,)?
    )) => {
        $crate::grammar::ast::Expr::type_expr($crate::grammar::ast::TypeExpr {
            name: token!($($name)*),
            or_ty: Some($crate::grammar::ast::OrType {
                pipe: token!($($pipe)*),
                ty: token!($($or_ty)*),
            }),
        })
    };

    (FnCall(
        // TODO
    )) => {
        $crate::grammar::ast::Expr::fn_call($crate::grammar::ast::FnCall {
            // TODO
        })
    };
}

macro_rules! lisp_test {
    ($expected:expr, $($ast:tt)*) => {
        assert_eq!(Lisp::new(&syntax_tree!($($ast)*)).to_string(), $expected)
    };
}

#[test]
fn test_ast_printer0() {
    lisp_test!(
        "(* (- 123) (group 45.67))",
        Binary(
            Unary(Token(Punctuation::SubNeg), Literal(SIntLiteral(123))),
            Token(Punctuation::Mul),
            Grouping(Literal(FracLiteral(45.67))),
        )
    );
}

#[test]
fn test_ast_printer_nested_exprs_rhs() {
    lisp_test!(
        "(+ 1 (+ 2 3))",
        Binary(
            Literal(SIntLiteral(1)),
            Token(Punctuation::Add),
            Binary(
                Literal(SIntLiteral(2)),
                Token(Punctuation::Add),
                Literal(SIntLiteral(3)),
            ),
        )
    );
}

#[test]
fn test_ast_printer_nested_exprs_lhs() {
    lisp_test!(
        "(+ (+ 1 2) 3)",
        Binary(
            Binary(
                Literal(SIntLiteral(1)),
                Token(Punctuation::Add),
                Literal(SIntLiteral(2)),
            ),
            Token(Punctuation::Add),
            Literal(SIntLiteral(3)),
        )
    );
}

#[test]
fn test_ast_printer_nested_exprs_both() {
    lisp_test!(
        "(+ (+ 1 2) (+ 3 4))",
        Binary(
            Binary(
                Literal(SIntLiteral(1)),
                Token(Punctuation::Add),
                Literal(SIntLiteral(2)),
            ),
            Token(Punctuation::Add),
            Binary(
                Literal(SIntLiteral(3)),
                Token(Punctuation::Add),
                Literal(SIntLiteral(4)),
            ),
        )
    );
}

macro_rules! binary_punc_tests {
    ($($test_name:ident($lhs:expr, $op:ident, $rhs:expr, $expected:expr $(,)?);)*) => {$(
        #[test]
        fn $test_name() {
            lisp_test!(
                $expected,
                Binary(
                    Literal(SIntLiteral($lhs)),
                    Token(Punctuation::$op),
                    Literal(SIntLiteral($rhs)),
                )
            );
        }
    )*};
}

binary_punc_tests! {
    test_print_bin_and_sints(1, And, 2, "(& 1 2)");
    test_print_bin_xor_sints(1, Xor, 2, "(^ 1 2)");
    test_print_bin_or_sints(1, Or, 2, "(| 1 2)");
    test_print_bin_nand_sints(1, Nand, 2, "(!& 1 2)");
    test_print_bin_xnor_sints(1, Xnor, 2, "(!^ 1 2)");
    test_print_bin_nor_sints(1, Nor, 2, "(!| 1 2)");
    test_print_bin_add_sints(1, Add, 2, "(+ 1 2)");
    test_print_bin_sub_neg_sints(1, SubNeg, 2, "(- 1 2)");
    test_print_bin_mul_sints(1, Mul, 2, "(* 1 2)");
    test_print_bin_div_sints(1, Div, 2, "(/ 1 2)");
    test_print_bin_rem_sints(1, Rem, 2, "(% 1 2)");
    test_print_bin_pow_sints(1, Pow, 2, "(** 1 2)");
    test_print_bin_shl_sints(1, Shl, 2, "(<< 1 2)");
    test_print_bin_shr_sints(1, Shr, 2, "(>> 1 2)");
    test_print_bin_rotl_sints(1, Rotl, 2, "([<<] 1 2)");
    test_print_bin_rotr_sints(1, Rotr, 2, "([>>] 1 2)");
    test_print_bin_convert_sints(1, Convert, 2, "(-:> 1 2)"); // TODO: this will never be the actual rhs of this operator
    test_print_bin_transmute_sints(1, Transmute, 2, "(=:> 1 2)"); // TODO: this will never be the actual rhs of this operator
    test_print_bin_eq_sints(1, Eq, 2, "(== 1 2)");
    test_print_bin_ne_sints(1, Ne, 2, "(!= 1 2)");
    test_print_bin_lt_sints(1, Lt, 2, "(< 1 2)");
    test_print_bin_gt_sints(1, Gt, 2, "(> 1 2)");
    test_print_bin_le_sints(1, Le, 2, "(<= 1 2)");
    test_print_bin_ge_sints(1, Ge, 2, "(>= 1 2)");
}

macro_rules! unary_pre_punc_tests {
    ($($test_name:ident($op:ident, $rhs:expr, $expected:expr $(,)?);)*) => {$(
        #[test]
        fn $test_name() {
            lisp_test!(
                $expected,
                Unary(
                    Token(Punctuation::$op),
                    Literal(SIntLiteral($rhs)),
                )
            );
        }
    )*};
}

unary_pre_punc_tests! {
    test_print_unary_pre_not_sint(Not, 1, "(! 1)");
    test_print_unary_pre_sub_neg_sint(SubNeg, 1, "(- 1)");
    test_print_unary_pre_exists_sint(Exists, 1, "(!! 1)");
}

macro_rules! unary_post_punc_tests {
    ($($test_name:ident($lhs:expr, $op:ident, $expected:expr $(,)?);)*) => {$(
        #[test]
        fn $test_name() {
            lisp_test!(
                $expected,
                Unary(
                    Literal(SIntLiteral($lhs)),
                    Token(Punctuation::$op),
                )
            );
        }
    )*};
}

unary_post_punc_tests! {
    test_print_unary_post_coalesce_sint(1, Coalesce, "(? 1)");
}

#[test]
fn test_print_type_expr_uint() {
    lisp_test!("uint", Type(Token(Keyword::Uint)));
}

#[test]
fn test_print_type_expr_uint_or_none() {
    lisp_test!(
        "(| uint none)",
        Type(
            Token(Keyword::Uint),
            Token(Punctuation::Or),
            Token(Keyword::None),
        )
    );
}

#[test]
fn test_print_type_expr_uint_or_fail() {
    lisp_test!(
        "(| uint fail)",
        Type(
            Token(Keyword::Uint),
            Token(Punctuation::Or),
            Token(Keyword::Fail),
        )
    );
}

macro_rules! literal_tests {
    ($($test_name:ident(Literal($($token:tt)*), $expected:expr $(,)?);)*) => {$(
        #[test]
        fn $test_name() {
            lisp_test!(
                $expected,
                Literal($($token)*)
            );
        }
    )*};
}

literal_tests! {
    test_print_literal_bool0(Literal(BoolLiteral(true)), "true");
    test_print_literal_bool1(Literal(BoolLiteral(false)), "false");
    test_print_literal_none(Literal(Keyword::None), "none");
    test_print_literal_sint(Literal(SIntLiteral(-53)), "-53");
    test_print_literal_uint(Literal(SIntLiteral(78)), "78");
    test_print_literal_frac(Literal(FracLiteral(54.87)), "54.87");
    test_print_literal_char(Literal(CharLiteral('x', false)), "'x'");
    test_print_literal_char_esc(Literal(CharLiteral('\x1b', true)), r"'\x1b'");
    test_print_literal_text(Literal(TextLiteral("apple")), r#""apple""#);
    test_print_literal_text_esc(Literal(TextLiteral("I \"bought\" an apple")), r#""I \"bought\" an apple""#);
}
