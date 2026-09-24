//! ANSI text styling

/// A type that can wrap an item in a style
pub trait StyleWrapper {
    /// The type used for opening the style
    type Begin: std::fmt::Display;
    /// The type used for finishing the style
    type End: std::fmt::Display;

    /// Gives the style opener
    fn begin(&self) -> Self::Begin;
    /// Gives the style closer
    fn end(&self) -> Self::End;

    /// Constructs a [`Styled`] for this type
    fn style<T>(&self, what: T) -> Styled<'_, T, Self> {
        Styled {
            style: self,
            inner: what,
        }
    }
}

/// Encloses `T` with the styling of `U`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Styled<'a, T, U>
where
    U: ?Sized + StyleWrapper,
{
    /// The style to wrap [`Self::inner`] with
    style: &'a U,
    /// The content being styled
    inner: T,
}

impl<T: std::fmt::Display, U: StyleWrapper> std::fmt::Display for Styled<'_, T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.style.begin(),
            self.inner,
            self.style.end()
        )
    }
}

/// An ANSI color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
#[allow(dead_code, reason = "flexibility")]
pub enum Color {
    /// 3-bit black
    Black = 0,
    /// 3-bit red
    Red,
    /// 3-bit green
    Green,
    /// 3-bit yellow
    Yellow,
    /// 3-bit blue
    Blue,
    /// 3-bit magenta
    Magenta,
    /// 3-bit cyan
    Cyan,
    /// 3-bit white
    White,
    /// Reset the color
    #[default]
    Default = 9,
    /// 3-bit black - bright
    BrightBlack = 60,
    /// 3-bit red - bright
    BrightRed,
    /// 3-bit green - bright
    BrightGreen,
    /// 3-bit yellow - bright
    BrightYellow,
    /// 3-bit blue - bright
    BrightBlue,
    /// 3-bit magenta - bright
    BrightMagenta,
    /// 3-bit cyan - bright
    BrightCyan,
    /// 3-bit white - bright
    BrightWhite,
    /// 8-bit color
    Id(u8),
    /// 24-bit truecolor
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

/// An ANSI style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    /// Bitflags defining bold, italic, underline, and strikethrough
    pub flags: u8,
    /// The foreground color
    pub color: Option<Color>,
    /// The background color
    pub background: Option<Color>,
}

impl Style {
    /// Bitflag for [`Style::flags`] representing the bold style
    const BOLD_FLAG: u8 = 1;
    /// Bitflag for [`Style::flags`] representing the italic style
    const ITALIC_FLAG: u8 = 2;
    /// Bitflag for [`Style::flags`] representing the underline style
    const UNDERLINE_FLAG: u8 = 4;
    /// Bitflag for [`Style::flags`] representing the strikethrough style
    const STRIKETHROUGH_FLAG: u8 = 8;

    /// Construct a new, default [`Style`]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            flags: 0,
            color: None,
            background: None,
        }
    }

    /// Make this style bold
    #[must_use]
    #[allow(dead_code, reason = "flexibility")]
    pub const fn bold(mut self) -> Self {
        self.flags |= Self::BOLD_FLAG;
        self
    }

    /// Make this style italic
    #[must_use]
    #[allow(dead_code, reason = "flexibility")]
    pub const fn italic(mut self) -> Self {
        self.flags |= Self::ITALIC_FLAG;
        self
    }

    /// Make this style underline
    #[must_use]
    #[allow(dead_code, reason = "flexibility")]
    pub const fn underline(mut self) -> Self {
        self.flags |= Self::UNDERLINE_FLAG;
        self
    }

    /// Make this style strikethrough
    #[must_use]
    #[allow(dead_code, reason = "flexibility")]
    pub const fn strikethrough(mut self) -> Self {
        self.flags |= Self::STRIKETHROUGH_FLAG;
        self
    }

    /// Set the foreground color for this style
    #[must_use]
    #[allow(dead_code, reason = "flexibility")]
    pub const fn foreground(mut self, value: Color) -> Self {
        self.color = Some(value);
        self
    }

    /// Set the background color for this style
    #[must_use]
    #[allow(dead_code, reason = "flexibility")]
    pub const fn background(mut self, value: Color) -> Self {
        self.background = Some(value);
        self
    }
}

impl StyleWrapper for Style {
    type Begin = BeginStyle;
    type End = EndStyle;

    fn begin(&self) -> Self::Begin {
        BeginStyle(*self)
    }

    fn end(&self) -> Self::End {
        EndStyle(*self)
    }
}

/// Creates the ANSI sequence for starting a styled segment
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
                        // SAFETY: Because we have handled the Id and Rgb variants, this branch cannot be either of those.
                        // The greatest discriminant of any non-Id, non-Rgb variant in Color is 67 (`BrightWhite`).
                        // Observe: 67+30=97. We know 97 <= 255, therefore it fits inside u8 without overflowing.
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
                        // SAFETY: Because we have handled the Id and Rgb variants, this branch cannot be either of those.
                        // The greatest discriminant of any non-Id, non-Rgb variant in Color is 67 (`BrightWhite`).
                        // Observe: 67+40=107. We know 107 <= 255, therefore it fits inside u8 without overflowing.
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

/// Creates the ANSI sequence for ending (undoing) a styled segment
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
