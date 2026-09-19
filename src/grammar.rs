//! Syntax (not semantic) highlighting

use std::range::Range;

use crate::scanner::{
    CharLiteral, ContextError, Error, InterpolatedExpr, InterpolatedString, RemappedEscapes,
    RemappedReplacements, Replacements, StringLiteral, Token, TokenResult, TokenType, TokenValue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    #[default]
    Default = 9,
    BrightBlack = 60,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Id(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    /// Copied from [`std::mem::discriminant`](https://doc.rust-lang.org/std/mem/fn.discriminant.html#accessing-the-numeric-value-of-the-discriminant) example
    fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Option<Color>,
    pub background: Option<Color>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: None,
            background: None,
        }
    }

    pub const fn bold(mut self, value: bool) -> Self {
        self.bold = value;
        self
    }

    pub const fn italic(mut self, value: bool) -> Self {
        self.italic = value;
        self
    }

    pub const fn underline(mut self, value: bool) -> Self {
        self.underline = value;
        self
    }

    pub const fn strikethrough(mut self, value: bool) -> Self {
        self.strikethrough = value;
        self
    }

    pub const fn foreground(mut self, value: Option<Color>) -> Self {
        self.color = value;
        self
    }

    pub const fn background(mut self, value: Option<Color>) -> Self {
        self.background = value;
        self
    }

    pub const fn begin(self) -> BeginStyle {
        BeginStyle(self)
    }

    pub const fn end(self) -> EndStyle {
        EndStyle(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BeginStyle(Style);

impl std::fmt::Display for BeginStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SEP: &str = ";";
        let Self(Style {
            bold,
            italic,
            underline,
            strikethrough,
            color,
            background,
        }) = *self;
        if bold || italic || underline || strikethrough || color.is_some() || background.is_some() {
            let mut has_prev = false;
            f.write_str("\x1b[")?;
            for (flag, code) in [
                (bold, "1"),
                (italic, "3"),
                (underline, "4"),
                (strikethrough, "9"),
            ] {
                if flag {
                    if has_prev {
                        f.write_str(SEP)?;
                    }
                    f.write_str(code)?;
                    has_prev = true;
                }
            }
            if let Some(color) = color {
                if has_prev {
                    f.write_str(SEP)?;
                }
                match color {
                    Color::Id(code) => write!(f, "38;5;{code}")?,
                    Color::Rgb(r, g, b) => write!(f, "38;2;{r};{g};{b}")?,
                    color => write!(f, "{}", color.discriminant() + 30)?,
                }
                has_prev = true;
            }
            if let Some(background) = background {
                if has_prev {
                    f.write_str(SEP)?;
                }
                match background {
                    Color::Id(code) => write!(f, "48;5;{code}")?,
                    Color::Rgb(r, g, b) => write!(f, "48;2;{r};{g};{b}")?,
                    background => write!(f, "{}", background.discriminant() + 40)?,
                }
            }
            f.write_str("m")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EndStyle(Style);

impl std::fmt::Display for EndStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SEP: &str = ";";
        let Self(Style {
            bold,
            italic,
            underline,
            strikethrough,
            color,
            background,
        }) = *self;
        if bold || italic || underline || strikethrough || color.is_some() || background.is_some() {
            let mut has_prev = false;
            f.write_str("\x1b[")?;
            for (flag, code) in [
                (bold, "22"),
                (italic, "23"),
                (underline, "24"),
                (strikethrough, "29"),
                (color.is_some(), "39"),
                (background.is_some(), "49"),
            ] {
                if flag {
                    if has_prev {
                        f.write_str(SEP)?;
                    }
                    f.write_str(code)?;
                    has_prev = true;
                }
            }
            f.write_str("m")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Styled<T> {
    pub style: Style,
    pub inner: T,
}

impl<T: std::fmt::Display> std::fmt::Display for Styled<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            BeginStyle(self.style),
            self.inner,
            EndStyle(self.style)
        )
    }
}

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

fn syntax_of<'a, 'b, T>(
    item: &'b TokenResult<'a, T>,
) -> (&'a str, Syntax, Option<&'b TokenValue<'a, T>>) {
    match item {
        Ok((token, value)) => (
            token.src,
            match token.ty {
                TokenType::Whitespace | TokenType::Punctuation => Syntax::Normal,
                TokenType::Comment => Syntax::Comment,
                TokenType::NumberLiteral => Syntax::NumberLiteral,
                TokenType::CharLiteral => Syntax::CharLiteral,
                TokenType::StringLiteral => Syntax::StringLiteral,
                TokenType::InterpolatedString => Syntax::InterpStrLiteral,
                TokenType::Identifier => Syntax::Variable, // TODO: distinguish from constants
                TokenType::Callable => Syntax::Callable,
                TokenType::Keyword => Syntax::Keyword,
                TokenType::CtrlKeyword => Syntax::CtrlKeyword,
            },
            value.as_ref(),
        ),
        Err(e) => (&e.source[e.range], Syntax::Invalid, None),
    }
}

#[derive(Debug, Clone)]
pub enum LiteralIter<'a, I> {
    Char(std::iter::Once<Range<usize>>),
    String(RemappedEscapes<std::iter::Copied<std::slice::Iter<'a, Range<usize>>>>),
    Interp(RemappedReplacements<'a, I>),
}

impl<'a, I> Iterator for LiteralIter<'a, I> {
    type Item = (Range<usize>, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Char(iter) => iter.next().map(|x| (x, Syntax::EscapeSeq)),
            Self::String(iter) => iter.next().map(|x| (x, Syntax::EscapeSeq)),
            Self::Interp(iter) => iter.next().map(|(range, x)| match x {
                Some(iter) => (range, Syntax::InterpExpr),
                None => (range, Syntax::EscapeSeq),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HighlightItem<'a, I: Iterator<Item = (Range<usize>, Syntax)>> {
    Simple(std::iter::Once<(&'a str, Syntax)>),
    Nested {
        outer: (&'a str, Syntax),
        prev_end: usize,
        iter: std::iter::Peekable<I>,
    },
}

impl<'a, I> Iterator for HighlightItem<'a, I>
where
    I: Iterator<Item = (Range<usize>, Syntax)>,
{
    type Item = (&'a str, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Self::Simple(ref mut item) => item.next(),

            Self::Nested {
                outer: (olex, osyn),
                ref mut prev_end,
                ref mut iter,
            } => iter
                .next_if(|(range, _)| range.start == *prev_end)
                .or_else(|| {
                    iter.peek()
                        .map(|(range, _)| Range::from(*prev_end..range.start))
                        .or_else(|| {
                            (*prev_end < olex.len()).then_some(Range::from(*prev_end..olex.len()))
                        })
                        .map(|range| (range, osyn))
                })
                .inspect(|(range, _)| *prev_end = range.end)
                .map(|(range, syn)| (&olex[range], syn)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Highlight<I> {
    tokens: I,
}

impl<I> Highlight<I> {
    const fn new(tokens: I) -> Self {
        Self { tokens }
    }
}

impl<'a, I, T> Iterator for Highlight<I>
where
    T: 'a,
    I: Iterator<Item = &'a TokenResult<'a, T>>,
{
    type Item = HighlightItem<'a, LiteralIter<'a, T>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.next().map(|item| {
            let (lex, syn, val) = syntax_of::<T>(item);
            match val {
                Some(TokenValue::CharLiteral(CharLiteral {
                    is_escaped: true, ..
                })) => HighlightItem::Nested {
                    outer: (lex, syn),
                    prev_end: 0,
                    iter: LiteralIter::Char(std::iter::once(Range::from(
                        '\''.len_utf8()..lex.len() - '\''.len_utf8(),
                    )))
                    .peekable(),
                },

                Some(TokenValue::StringLiteral(lit @ StringLiteral { escapes, .. }))
                    if !escapes.is_empty() =>
                {
                    HighlightItem::Nested {
                        outer: (lex, syn),
                        prev_end: 0,
                        iter: LiteralIter::String(lit.remapped_escapes("\"")).peekable(),
                    }
                }

                Some(TokenValue::InterpolatedString(
                    lit @ InterpolatedString {
                        text: StringLiteral { escapes, .. },
                        expressions,
                    },
                )) if !escapes.is_empty() || !expressions.is_empty() => HighlightItem::Nested {
                    outer: (lex, syn),
                    prev_end: 0,
                    iter: LiteralIter::Interp(lit.remapped_replacements("`")).peekable(),
                },

                _ => HighlightItem::Simple(std::iter::once((lex, syn))),
            }
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tokens.size_hint()
    }
}

pub fn highlight<'a, I, T>(tokens: I) -> impl Iterator<Item = (&'a str, Syntax)>
where
    T: 'a,
    I: IntoIterator<Item = &'a TokenResult<'a, T>>,
{
    Highlight::new(tokens.into_iter()).flatten()
}
