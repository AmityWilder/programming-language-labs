use crate::{
    error::{Diagnostic, DiagnosticKind},
    lex::{
        symbols::{CHAR_DELIM, ESCAPE, MACRO_PARAM_PREFIX, MACRO_PREFIX, TEXT_DELIM},
        token::{TokenKind, TokenTable, keyword::Keyword},
    },
    scanner::token::punc::Punctuation,
    src_code::span::{FileId, RelSpan, usize_to_uspan, uspan},
};

pub mod symbols;
pub mod token;

#[derive(Debug, Clone)]
pub struct Lexer<'src> {
    // A reference to the original source code. Since this is only a copy, it will get ripped apart and fed to the tokens.
    // The next token will always be at the start of this string.
    source: &'src str,
}

impl<'src> Lexer<'src> {
    const fn new(source: &'src str) -> Self {
        debug_assert!(
            uspan::try_from(source.len()).is_ok(),
            "should not tokenize oversize file"
        );
        Self { source }
    }

    fn whitespace(&mut self) -> usize {
        self.source
            .find(|ch: char| !ch.is_whitespace())
            .unwrap_or(self.source.len())
    }

    fn line_comment(&mut self) -> usize {
        self.source.find(['\n', '\r']).unwrap_or(self.source.len())
    }

    /// [`Err`]: unterminated
    fn block_comment(&mut self) -> Result<usize, ()> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        enum CommentChar {
            Star = -1,
            #[default]
            LiterallyAnythingElse = 0,
            Slash = 1,
        }
        self.source
            .find({
                use CommentChar::{LiterallyAnythingElse, Slash, Star};
                let mut depth: usize = 0;
                let mut prev = LiterallyAnythingElse;
                move |ch: char| {
                    let curr = match ch {
                        '*' => Star,
                        '/' => Slash,
                        _ => LiterallyAnythingElse,
                    };
                    match (prev, curr) {
                        (Slash, Star) => {
                            depth = depth.strict_add(1);
                            // no double-dipping
                            prev = LiterallyAnythingElse;
                        }

                        (Star, Slash) => {
                            if let Some(n @ 1..) = depth.checked_sub(1) {
                                depth = n;
                                // no double-dipping
                                prev = LiterallyAnythingElse;
                            } else {
                                return true;
                            }
                        }

                        _ => prev = curr,
                    }
                    false
                }
            })
            .map(|pos| {
                debug_assert!(
                    self.source.get(pos..).expect("find should return a valid position").starts_with('/'),
                    "`pos` should be the position of the closing `/`"
                );
                #[expect(clippy::arithmetic_side_effects, reason = "if `pos` is the position of a `/`, that `/` must be stored in the string at `pos`, \
                    meaning the string's len() is an integer at least equal to this.")]
                (pos + '/'.len_utf8())
            }).ok_or(())
    }

    fn macro_ident<const PREFIX: char>(&mut self, ch2: Option<char>) -> Result<usize, usize> {
        if ch2.is_some_and(|ch| ch.is_alphabetic() || ch == '_') {
            Ok(self.source[PREFIX.len_utf8()..]
                .find(|ch: char| !ch.is_alphanumeric() && ch != '_')
                .map_or(self.source.len(), |n| n + PREFIX.len_utf8()))
        } else {
            Err(PREFIX.len_utf8())
        }
    }

    fn strlike_lit<const DELIM: char>(&mut self) -> Option<usize> {
        let mut iter = self.source.char_indices();
        let _open = iter.next(); // skip
        debug_assert_eq!(_open, Some((0, DELIM)), "delimiter should be included");
        while let Some((i, ch)) = iter.next() {
            if ch == DELIM {
                return Some(i + DELIM.len_utf8());
            } else if ch == ESCAPE {
                _ = iter.next();
            }
        }
        None
    }

    fn num_lit(&mut self) -> usize {
        // the position of the first non-matching is equivalent to the length of the matching
        let mut iter = self.source.char_indices().filter(|(_, ch)| *ch != '_');
        let Some((len, next)) = iter.find(|&(_, ch)| !ch.is_alphanumeric()) else {
            return self.source.len();
        };
        if next == '.'
            && let Some((_, '0'..='9')) = iter.next()
        {
            let mut follows_e = false;
            let Some((len, next)) = iter.find(|&(_, ch)| {
                follows_e = matches!(ch, 'e' | 'E');
                !ch.is_alphanumeric()
            }) else {
                return self.source.len();
            };
            if follows_e
                && matches!(next, '+' | '-')
                && let Some((_, '0'..='9')) = iter.next()
            {
                iter.find(|&(_, ch)| !ch.is_alphanumeric())
                    .map_or(self.source.len(), |(len, _)| len)
            } else {
                len
            }
        } else {
            len
        }
    }

    fn ident(&mut self) -> usize {
        self.source
            .find(|ch: char| !ch.is_alphanumeric() && ch != '_')
            .unwrap_or(self.source.len())
    }

    fn punc(&mut self) -> Option<Punctuation> {
        Punctuation::from_prefix(self.source)
    }
}

impl Iterator for Lexer<'_> {
    type Item = (uspan, TokenKind, Option<DiagnosticKind>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut chars = self.source.chars();
        chars
            .next()
            .map(|ch1| match (ch1, chars.next()) {
                // whitespace
                (ch, _) if ch.is_whitespace() => (self.whitespace(), TokenKind::WSpace, None),

                // line comment
                ('/', Some('/')) => (self.line_comment(), TokenKind::Comment, None),

                // block commen
                ('/', Some('*')) => match self.block_comment() {
                    Ok(len) => (len, TokenKind::Comment, None),
                    Err(()) => (
                        self.source.len(),
                        TokenKind::Comment,
                        Some(DiagnosticKind::EndlessBlockComment),
                    ),
                },

                // macro name
                (MACRO_PREFIX, ch2) => match self.macro_ident::<MACRO_PREFIX>(ch2) {
                    Ok(len) => (len, TokenKind::Macro, None),
                    Err(len) => (
                        len,
                        TokenKind::Invalid,
                        Some(DiagnosticKind::EmptyMacroName),
                    ),
                },

                // macro parameter
                (MACRO_PARAM_PREFIX, ch2) => match self.macro_ident::<MACRO_PARAM_PREFIX>(ch2) {
                    Ok(len) => (len, TokenKind::MParam, None),
                    Err(len) => (
                        len,
                        TokenKind::Invalid,
                        Some(DiagnosticKind::EmptyMacroParam),
                    ),
                },

                // character literal
                (CHAR_DELIM, _) => {
                    let len = self.strlike_lit::<CHAR_DELIM>();
                    (
                        len.unwrap_or(self.source.len()),
                        TokenKind::CharLit,
                        len.is_none().then_some(DiagnosticKind::EndlessCharLiteral),
                    )
                }

                // text literal
                (TEXT_DELIM, _) => {
                    let len = self.strlike_lit::<TEXT_DELIM>();
                    (
                        len.unwrap_or(self.source.len()),
                        TokenKind::TextLit,
                        len.is_none()
                            .then_some(DiagnosticKind::EndlessStringLiteral),
                    )
                }

                // number literal
                ('0'..='9', _) => (self.num_lit(), TokenKind::NumberLit, None),

                // identifier
                (ch, _) if ch.is_alphabetic() || ch == '_' => {
                    let len = self.ident();
                    #[expect(
                        clippy::string_slice,
                        reason = "ident() should return a valid position in source"
                    )]
                    let s = &self.source[..len];
                    if let Some(kw) = Keyword::try_from_str(s) {
                        (len, TokenKind::Keyword(kw), None)
                    } else if matches!(s, "true" | "fals") {
                        (len, TokenKind::BoolLit(s == "true"), None)
                    } else {
                        (self.ident(), TokenKind::Ident, None)
                    }
                }

                // punctuation or give up
                (ch, _) => {
                    if let Some(punc) = self.punc() {
                        (punc.as_str().len(), TokenKind::Punc(punc), None)
                    } else {
                        (
                            ch.len_utf8(),
                            TokenKind::Invalid,
                            Some(DiagnosticKind::UnknownToken),
                        )
                    }
                }
            })
            .inspect(
                #[expect(
                    clippy::string_slice,
                    reason = "should produce a valid position in source"
                )]
                |&(len, kind, ref err)| {
                    assert_ne!(
                        len,
                        0,
                        "token should not be empty\
                        \n upcoming: {:?}\
                        \n     kind: {kind:?}\
                        \n      err: {err:?}",
                        self.source.lines().next().unwrap_or("")
                    );
                    self.source = &self.source[len..];
                },
            )
            .map(|(len, kind, e)| (usize_to_uspan(len), kind, e))
    }
}

pub fn lex(file: FileId, code: &str, diagnostics: &mut Vec<Diagnostic>) -> TokenTable {
    let mut tokens = Vec::new();
    let mut start: uspan = 0;
    for (len, kind, e) in Lexer::new(code) {
        let end = start.strict_add(len);
        if let Some(kind) = e {
            let span = RelSpan::from(start..end).within_file(file);
            diagnostics.push(Diagnostic::new(span, kind));
        }
        tokens.push((len, kind));
        start = end;
    }
    TokenTable { tokens }
}
