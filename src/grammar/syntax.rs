use crate::{
    grammar::style::{Style, Styled},
    scanner::{TokenResult, TokenType, TokenValue},
};

/// Forces me to make both correctly
macro_rules! syntaxes {
    (
        $(#[$smeta:meta])*
        $svis:vis struct $Struct:ident;

        $(#[$emeta:meta])*
        $evis:vis enum $Enum:ident {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident {
                    $(#[$fmeta:meta])*
                    $field:ident
                }
            ),* $(,)?
        }
    ) => {
        $(#[$smeta])*
        $svis struct $Struct {$(
            $(#[$fmeta])*
            pub $field: Style,
        )*}

        $(#[$emeta])*
        $evis enum $Enum {$(
            $(#[$vmeta])*
            $Variant,
        )*}

        impl std::ops::Index<$Enum> for $Struct {
            type Output = Style;

            fn index(&self, index: $Enum) -> &Self::Output {
                match index {
                    $($Enum::$Variant => &self.$field,)*
                }
            }
        }
    };
}

syntaxes! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SyntaxStyle;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub enum Syntax {
        #[default]
        Normal { normal },
        Comment { comment },
        NumberLiteral { number_literal },
        CharLiteral { char_literal },
        StringLiteral { string_literal },
        InterpStrLiteral { interp_str_literal },
        EscapeSeq { escape_seq },
        InterpExpr { interp_expr },
        Variable { variable },
        Constant { constant },
        Callable { callable },
        Keyword { keyword },
        CtrlKeyword { ctrl_keyword },
        Bracket { bracket },
        Invalid { invalid },
    }
}

impl SyntaxStyle {
    pub fn stylize<T>(&self, syntax: Syntax, what: T) -> Styled<T> {
        Styled {
            style: self[syntax],
            inner: what,
        }
    }
}

pub fn syntax_of<'a: 'b, 'b, T>(
    item: &'b TokenResult<'a, T>,
) -> (&'a str, Syntax, Option<&'b TokenValue<'a, T>>) {
    match item {
        Ok((token, value)) => (
            token.src,
            match token.ty {
                TokenType::Comment => Syntax::Comment,
                TokenType::NumberLiteral => Syntax::NumberLiteral,
                TokenType::CharLiteral => Syntax::CharLiteral,
                TokenType::StringLiteral => Syntax::StringLiteral,
                TokenType::InterpolatedString => Syntax::InterpStrLiteral,
                TokenType::Identifier => Syntax::Variable, // TODO: distinguish from constants
                TokenType::Callable => Syntax::Callable,
                TokenType::Keyword => Syntax::Keyword,
                TokenType::CtrlKeyword => Syntax::CtrlKeyword,
                TokenType::Punctuation
                    if matches!(token.src, "[" | "]" | "(" | ")" | "{" | "}") =>
                {
                    Syntax::Bracket
                }

                TokenType::Whitespace | TokenType::Punctuation => Syntax::Normal,
            },
            value.as_ref(),
        ),
        Err(e) => (&e.source[e.range], Syntax::Invalid, None),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BracketPair<'a> {
    pub depth: &'a [Style],
}

impl std::ops::Index<usize> for BracketPair<'_> {
    type Output = Style;

    fn index(&self, index: usize) -> &Self::Output {
        &self.depth[index % self.depth.len()]
    }
}

impl BracketPair<'_> {
    pub fn stylize<T>(&self, depth: usize, what: T) -> Styled<T> {
        Styled {
            style: self[depth],
            inner: what,
        }
    }
}
