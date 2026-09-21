#![allow(dead_code)]

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

#[allow(clippy::struct_excessive_bools, reason = "flags")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    pub flags: u8,
    pub color: Option<Color>,
    pub background: Option<Color>,
}

impl Style {
    const BOLD_FLAG: u8 = 1;
    const ITALIC_FLAG: u8 = 2;
    const UNDERLINE_FLAG: u8 = 4;
    const STRIKETHROUGH_FLAG: u8 = 8;

    pub const fn new() -> Self {
        Self {
            flags: 0,
            color: None,
            background: None,
        }
    }

    pub const fn bold(mut self) -> Self {
        self.flags |= Self::BOLD_FLAG;
        self
    }

    pub const fn italic(mut self) -> Self {
        self.flags |= Self::ITALIC_FLAG;
        self
    }

    pub const fn underline(mut self) -> Self {
        self.flags |= Self::UNDERLINE_FLAG;
        self
    }

    pub const fn strikethrough(mut self) -> Self {
        self.flags |= Self::STRIKETHROUGH_FLAG;
        self
    }

    pub const fn foreground(mut self, value: Color) -> Self {
        self.color = Some(value);
        self
    }

    pub const fn background(mut self, value: Color) -> Self {
        self.background = Some(value);
        self
    }

    pub const fn begin(self) -> BeginStyle {
        BeginStyle(self)
    }

    pub const fn end(self) -> EndStyle {
        EndStyle(self)
    }

    pub const fn style<T>(self, what: T) -> Styled<T> {
        Styled {
            style: self,
            inner: what,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BeginStyle(Style);

impl std::fmt::Display for BeginStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SEP: &str = ";";
        let Self(Style {
            flags,
            color,
            background,
        }) = *self;
        if flags != 0 || color.is_some() || background.is_some() {
            let mut has_prev = false;
            f.write_str("\x1b[")?;
            for (flag, code) in [
                (flags & Style::BOLD_FLAG != 0, "1"),
                (flags & Style::ITALIC_FLAG != 0, "3"),
                (flags & Style::UNDERLINE_FLAG != 0, "4"),
                (flags & Style::STRIKETHROUGH_FLAG != 0, "9"),
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
                    color => {
                        // SAFETY: the greatest discriminant of any non-Id, non-Rgb variant in Color is 67; 67+30=97, which is less than 255
                        let code = unsafe { color.discriminant().unchecked_add(30) };
                        write!(f, "{code}")?;
                    }
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
                    background => {
                        // SAFETY: the greatest discriminant of any non-Id, non-Rgb variant in Color is 67; 67+40=107, which is less than 255
                        let code = unsafe { background.discriminant().unchecked_add(40) };
                        write!(f, "{code}")?;
                    }
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
            flags,
            color,
            background,
        }) = *self;
        if flags != 0 || color.is_some() || background.is_some() {
            let mut has_prev = false;
            f.write_str("\x1b[")?;
            for (flag, code) in [
                (flags & Style::BOLD_FLAG != 0, "22"),
                (flags & Style::ITALIC_FLAG != 0, "23"),
                (flags & Style::UNDERLINE_FLAG != 0, "24"),
                (flags & Style::STRIKETHROUGH_FLAG != 0, "29"),
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
