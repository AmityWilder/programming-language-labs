use crate::{
    lex::token::{IdentKind, PuncKind, TokenKind, TokenTable},
    new_error::{Error, ErrorKind},
    scanner::{
        symbols::{CHAR_DELIM, ESCAPE, MACRO_PARAM_PREFIX, MACRO_PREFIX, TEXT_DELIM},
        token::punc::Punctuation,
    },
    src_code::span::{FileId, RelSpan, Span, uspan},
};

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
            .find(|ch: char| !ch.is_whitespace() || ch == '\n')
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
                {
                    pos + '/'.len_utf8()
                }
            }).ok_or(())
    }

    fn macro_ident<const PREFIX: char>(&mut self, ch2: Option<char>) -> Result<usize, usize> {
        if ch2.is_some_and(|ch| ch.is_alphabetic() || ch == '_') {
            Ok(self.source[PREFIX.len_utf8()..]
                .find(|ch: char| !ch.is_alphanumeric() && ch != '_')
                .map(|n| n + PREFIX.len_utf8())
                .unwrap_or(self.source.len()))
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
            .find(|ch: char| !ch.is_alphanumeric())
            .unwrap_or(self.source.len())
    }

    fn punc(&mut self) -> Option<(usize, TokenKind)> {
        Punctuation::from_prefix(self.source).map(|punc| {
            (
                punc.as_str().len(),
                TokenKind::Punc(punc, PuncKind::default()),
            )
        })
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = (uspan, TokenKind, Option<ErrorKind>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut chars = self.source.chars();
        chars
            .next()
            .map(|ch1| match (ch1, chars.next()) {
                // newline
                ('\n', _) => ('\n'.len_utf8(), TokenKind::Newline, None),

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
                        Some(ErrorKind::EndlessBlockComment),
                    ),
                },

                // macro name
                (MACRO_PREFIX, ch2) => match self.macro_ident::<MACRO_PREFIX>(ch2) {
                    Ok(len) => (len, TokenKind::Macro, None),
                    Err(len) => (len, TokenKind::Invalid, Some(ErrorKind::EmptyMacroName)),
                },

                // macro parameter
                (MACRO_PARAM_PREFIX, ch2) => match self.macro_ident::<MACRO_PARAM_PREFIX>(ch2) {
                    Ok(len) => (len, TokenKind::MParam, None),
                    Err(len) => (len, TokenKind::Invalid, Some(ErrorKind::EmptyMacroParam)),
                },

                // character literal
                (CHAR_DELIM, _) => {
                    let len = self.strlike_lit::<CHAR_DELIM>();
                    (
                        len.unwrap_or(self.source.len()),
                        TokenKind::CharLit,
                        len.is_none().then_some(ErrorKind::EndlessCharLiteral),
                    )
                }

                // text literal
                (TEXT_DELIM, _) => {
                    let len = self.strlike_lit::<TEXT_DELIM>();
                    (
                        len.unwrap_or(self.source.len()),
                        TokenKind::TextLit,
                        len.is_none().then_some(ErrorKind::EndlessStringLiteral),
                    )
                }

                // number literal
                ('0'..='9', _) => (self.num_lit(), TokenKind::NumberLit, None),

                // identifier
                (ch, _) if ch.is_alphabetic() || ch == '_' => {
                    (self.ident(), TokenKind::Ident(IdentKind::default()), None)
                }

                // punctuation or give up
                (ch, _) => {
                    if let Some((len, kind)) = self.punc() {
                        (len, kind, None)
                    } else {
                        (
                            ch.len_utf8(),
                            TokenKind::Invalid,
                            Some(ErrorKind::UnknownToken),
                        )
                    }
                }
            })
            .inspect(|(len, _, _)| self.source = &self.source[*len..])
            .map(|(len, kind, e)| {
                debug_assert!(
                    uspan::try_from(len).is_ok(),
                    "should not tokenize oversize file"
                );
                (len as uspan, kind, e)
            })
    }
}

pub fn lex(file: FileId, code: &str) -> (TokenTable, Vec<Error>) {
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let mut start: uspan = 0;
    for (len, kind, e) in Lexer::new(code) {
        let end = start.strict_add(len);
        if let Some(kind) = e {
            let span = Span {
                file,
                rel: RelSpan::from(start..end),
            };
            errors.push(Error::new(span, kind));
        }
        tokens.push((len, kind));
        start = end;
    }
    (TokenTable { tokens }, errors)
}
