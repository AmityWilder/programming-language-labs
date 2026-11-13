use crate::{
    SYNTAX_STYLE_ANSI,
    highlight::{
        style::{Color, Style, StyleWrapper},
        write_highlight,
    },
    lex::token::TokenKind,
    src_code::{
        code::{LineContentSpans, SourceCode},
        map::SourceMap,
        span::{RelSpan, Span, usize_to_uspan},
    },
};
use std::{collections::VecDeque, fmt::Write, range::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MessageStyle {
    pub style: Style,
    pub underline: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum MessageKind {
    #[default]
    Info,
    #[expect(dead_code, reason = "future use")]
    Warning,
    Error,
}

impl MessageKind {
    const INFO_STYLE: MessageStyle = MessageStyle {
        style: Style::new().foreground(Color::BrightBlue).bold(true),
        underline: '-',
    };

    const WARNING_STYLE: MessageStyle = MessageStyle {
        style: Style::new().foreground(Color::Yellow).bold(true),
        underline: '~',
    };

    const ERROR_STYLE: MessageStyle = MessageStyle {
        style: Style::new().foreground(Color::BrightRed).bold(true),
        underline: '^',
    };

    pub const fn style(self) -> MessageStyle {
        match self {
            Self::Info => Self::INFO_STYLE,
            Self::Warning => Self::WARNING_STYLE,
            Self::Error => Self::ERROR_STYLE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum CodeDelta<T> {
    #[default]
    None,
    Remove,
    Insert(T),
    Replace(T),
}

impl<T> CodeDelta<T> {
    const INSERT_STYLE: MessageStyle = MessageStyle {
        style: Style::new().foreground(Color::BrightGreen).bold(true),
        underline: '+',
    };

    const REMOVE_STYLE: MessageStyle = MessageStyle {
        style: Style::new().foreground(Color::BrightRed).bold(true),
        underline: '~',
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeRef<T> {
    pub kind: MessageKind,
    pub span: Span,
    pub msg: T,
}

impl<T> CodeRef<T> {
    pub const fn new(kind: MessageKind, span: Span, msg: T) -> Self {
        Self { kind, span, msg }
    }
}

pub trait CodeReference {
    fn kind(&self) -> &MessageKind;
    fn span(&self) -> &Span;
    fn msg(&self) -> &dyn std::fmt::Display;
}

impl<T: ?Sized + CodeReference> CodeReference for &T {
    fn kind(&self) -> &MessageKind {
        (*self).kind()
    }

    fn span(&self) -> &Span {
        (*self).span()
    }

    fn msg(&self) -> &dyn std::fmt::Display {
        (*self).msg()
    }
}

impl<T: ?Sized + CodeReference> CodeReference for Box<T> {
    fn kind(&self) -> &MessageKind {
        (**self).kind()
    }

    fn span(&self) -> &Span {
        (**self).span()
    }

    fn msg(&self) -> &dyn std::fmt::Display {
        (**self).msg()
    }
}

impl<T: std::fmt::Display> CodeReference for CodeRef<T> {
    fn kind(&self) -> &MessageKind {
        &self.kind
    }

    fn span(&self) -> &Span {
        &self.span
    }

    fn msg(&self) -> &dyn std::fmt::Display {
        &self.msg
    }
}

pub fn render_code_refs<I>(
    source: &SourceMap,
    refs: I,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result
where
    I: IntoIterator<Item: CodeReference>,
{
    CodeRefs::new(source, refs).render(f)
}

struct CodeRefsQueue<T> {
    dferred: VecDeque<T>,
    upcoming: VecDeque<T>,
}

struct CodeRefs<'src, T> {
    source: &'src SourceMap,
    q: CodeRefsQueue<T>,
}

impl<'src, T: CodeReference> CodeRefs<'src, T> {
    fn new<I>(source: &'src SourceMap, refs: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut refs = Vec::from_iter(refs);

        refs.sort_unstable_by(|a, b| {
            a.span()
                .partial_cmp(b.span()) // returns None if there is overlap
                .unwrap_or_else(|| {
                    // overlap case: we want to maximize the number of spans packed into one group,
                    // since that should (I think) minimize the overall number of groups without exceeding sorting-algorithm complexity.
                    //
                    // look, this is literally the set cover problem, ok?
                    // I don't want the compiler chugging along for 20 minutes just to minimize the number of separate underline groups.

                    // where there is overlap, order by earliest end, then earliest start.
                    // by preferring the earliest end first, there's a greater chance of fitting more non-overlapping spans after.
                    //
                    // since spans that end earlier are ordered earlier, any spans that could fill the gap would have already come first.
                    // so, if two spans have the same end, prefer to order the one that starts earlier ahead of the one that starts later.
                    // that way, the larger one can fill the gap.
                    //
                    // if everything else is equal, order the most important message first.
                    // note: `a.kind` and `b.kind` are swapped, since they should be in *descending* order of importance
                    let x = (a.span().end, a.span().start, b.kind());
                    let y = (b.span().end, b.span().start, a.kind());
                    x.cmp(&y)
                })
        });

        Self {
            source,
            q: CodeRefsQueue {
                dferred: VecDeque::from(refs),
                upcoming: VecDeque::new(),
            },
        }
    }

    fn render(&mut self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        while !self.q.dferred.is_empty() {
            std::mem::swap(&mut self.q.upcoming, &mut self.q.dferred);
            while let Some(mut group) = CodeRefsGroup::new(self) {
                group.render(f)?;
            }
        }
        Ok(())
    }
}

// TODO: change to use tokens so we can get highlighting
struct CodeRefsGroup<'src, 'outer, T> {
    q: &'outer mut CodeRefsQueue<T>,
    prev_span: Option<Span>,
    curr_file: &'src SourceCode,
    lines: std::iter::Enumerate<LineContentSpans<'src>>,
}

impl<'src, 'outer, T: CodeReference> CodeRefsGroup<'src, 'outer, T> {
    fn new(outer: &'outer mut CodeRefs<'src, T>) -> Option<Self> {
        if let Some(first) = outer.q.upcoming.front() {
            let curr_file = outer.source.source_of(*first.span());
            Some(Self {
                q: &mut outer.q,
                prev_span: None,
                lines: curr_file.line_content_spans().enumerate(),
                curr_file,
            })
        } else {
            None
        }
    }

    fn render(&mut self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const LINE_PRELUDE_STYLE: Style = Style::new().foreground(Color::BrightCyan).bold(true);
        let max_linenum = self.curr_file.line_lens().len();
        let max_linenum_width = max_linenum.to_string().len();
        write!(
            f,
            "{}{:>max_linenum_width$} |  ",
            LINE_PRELUDE_STYLE.begin(),
            ""
        )?;
        while let Some(item) = self.q.upcoming.pop_front() {
            if let Some(prev) = self.prev_span {
                if prev.file != item.span().file {
                    break;
                } else if item.span().overlaps(prev) {
                    self.q.dferred.push_back(item);
                    continue;
                }
            }

            let prev_end = self.prev_span.map_or(0, |prev| {
                debug_assert_eq!(
                    prev.file,
                    item.span().file,
                    "different files should be handled in different loops"
                );
                prev.end
            });

            debug_assert!(
                prev_end <= item.span().start,
                "non-overlapping previous element should end before the next one starts\n prev: {:?}\n curr: {:?}",
                self.prev_span
                    .expect("item.span.start can't be less than 0, so prev must have been Some"),
                item.span(),
            );

            let mut sep = RelSpan(Range {
                start: prev_end,
                end: item.span().start,
            });

            let mut skipped_lines = self.curr_file[sep].matches('\n');
            if self.prev_span.is_none() || skipped_lines.next().is_some() {
                if skipped_lines.next().is_some() {
                    // multiple: skip lines with "..."
                    for _ in self.curr_file[sep].matches('\n') {
                        _ = self.lines.next();
                    }
                    if self.prev_span.is_some() {
                        writeln!(f, "\n{}...", LINE_PRELUDE_STYLE.begin())?;
                    }
                } else {
                    // one: print the next line
                }
                if let Some(last_linebreak) = self.curr_file[sep].rfind('\n') {
                    // truncate to start of line
                    sep.0.start = usize_to_uspan(
                        #[expect(
                            clippy::arithmetic_side_effects,
                            reason = "by finding the position of a character, it is guaranteed that position + the character's lenth will fit in usize"
                        )]
                        (last_linebreak + '\n'.len_utf8()),
                    );
                }
                let (i, line_span) = self
                    .lines
                    .next()
                    .unwrap_or_else(|| todo!("error at end of document"));
                write!(
                    f,
                    "\n{}{:>max_linenum_width$} |{}  ",
                    LINE_PRELUDE_STYLE.begin(),
                    i.strict_add(1),
                    LINE_PRELUDE_STYLE.end(),
                )?;
                let line_tokens = self
                    .curr_file
                    .tokens()
                    .lines(self.curr_file)
                    .overlapping(line_span)
                    .map(|(span, kind)| (&self.curr_file[span], kind));
                write_highlight(line_tokens, f, &SYNTAX_STYLE_ANSI)?;
                write!(
                    f,
                    "\n{}{:>max_linenum_width$} |{}  ",
                    LINE_PRELUDE_STYLE.begin(),
                    "",
                    LINE_PRELUDE_STYLE.end(),
                )?;
            } else {
                // none: still on the same line
            }

            for _ in self.curr_file[sep].chars() {
                f.write_char(' ')?;
            }

            let msg_style = item.kind().style();
            std::fmt::Display::fmt(&msg_style.style.begin(), f)?;
            let underline = msg_style.underline;
            for _ in self.curr_file[item.span().rel].chars() {
                // TODO: multiline
                f.write_char(underline)?;
            }
            write!(f, " {}", item.msg())?; // TODO: multiple in one line
            std::fmt::Display::fmt(&msg_style.style.end(), f)?;

            self.prev_span = Some(*item.span());
        }
        writeln!(
            f,
            "\n{}{:>max_linenum_width$} |\x1b[0m",
            LINE_PRELUDE_STYLE.begin(),
            ""
        )
    }
}
