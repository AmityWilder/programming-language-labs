#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    clippy::todo,
    reason = "need to be aware of when the structure changes so its formatting can update"
)]

use super::{EnvId, Value, tree};
use crate::types::{byte, fail, flpt, none, sint, text, uint};
use std::fmt::{self, Write};

mod helpers {
    use super::*;

    /// [`std::fmt::PadAdapter`] is private :c
    pub struct Indented<'buf> {
        buf: &'buf mut (dyn 'buf + fmt::Write),
        on_newline: bool,
    }

    impl<'buf> Indented<'buf> {
        pub const INDENT: &str = "  ";

        fn wrap<'slot, 'fmt>(
            f: &'fmt mut fmt::Formatter<'_>,
            slot: &'slot mut Option<Self>,
        ) -> fmt::Formatter<'slot>
        where
            'fmt: 'buf + 'slot,
        {
            fmt::FormattingOptions::new()
                .alternate(f.alternate()) // <- the only option that's important in this situation
                .create_formatter(slot.insert(Self {
                    buf: f,
                    on_newline: true,
                }))
        }
    }

    impl Write for Indented<'_> {
        // while this function looks exactly identical to the one in the standard library,
        // I actually wrote it BEFORE checking the implementation and I just happened to "convergent evolution" the same definition.
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for line in s.split_inclusive('\n') {
                if self.on_newline {
                    self.buf.write_str(Self::INDENT)?;
                }
                self.on_newline = line.ends_with('\n');
                self.buf.write_str(line)?;
            }
            Ok(())
        }

        // this one I wrote after finding the implementation, and it's basically copy-pasted
        fn write_char(&mut self, c: char) -> fmt::Result {
            if self.on_newline {
                self.buf.write_str(Self::INDENT)?;
            }
            self.on_newline = c == '\n';
            self.buf.write_char(c)
        }
    }

    pub struct MirBlock<'fmt, 'buf> {
        f: &'fmt mut fmt::Formatter<'buf>,
        res: fmt::Result,
        has_prev: bool,
    }

    impl<'fmt, 'buf> MirBlock<'fmt, 'buf> {
        pub fn new(f: &'fmt mut fmt::Formatter<'buf>) -> Self {
            Self {
                res: f.write_str("{"),
                f,
                has_prev: false,
            }
        }

        pub fn finish(&mut self) -> fmt::Result {
            self.res?;
            self.f.write_str("}")
        }

        pub fn entry_with<F>(&mut self, fun: F) -> &mut Self
        where
            F: FnOnce(&mut fmt::Formatter<'_>) -> fmt::Result,
        {
            self.res = self.res.and_then(|()| {
                if self.f.alternate() {
                    if !self.has_prev {
                        self.f.write_str("\n")?;
                    }
                    let mut slot = None;
                    let mut writer = Indented::wrap(self.f, &mut slot);
                    fun(&mut writer)?;
                    self.f.write_str("\n")
                } else {
                    if self.has_prev {
                        self.f.write_str(" ")?;
                    }
                    fun(&mut *self.f)
                }
            });
            self.has_prev = true;
            self
        }

        pub fn entry<D>(&mut self, item: &D) -> &mut Self
        where
            D: ?Sized + MirDisplay,
        {
            self.entry_with(move |f| item.fmt(f))
        }

        pub fn entries<D, I>(&mut self, items: I) -> &mut Self
        where
            D: MirDisplay,
            I: IntoIterator<Item = D>,
        {
            for item in items {
                self.entry(&item);
            }
            self
        }
    }
}
use helpers::*;

pub trait MirDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T: ?Sized + MirDisplay> MirDisplay for &T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (*self).fmt(f)
    }
}

#[derive(Clone, Copy)]
pub struct DisplayMir<'mir> {
    pub mir: &'mir tree::Program,
}

impl fmt::Display for DisplayMir<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mir.fmt(f)
    }
}

impl MirDisplay for tree::Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { stmts } = self;
        for stmt in stmts {
            stmt.fmt(f)?;
        }
        Ok(())
    }
}

impl MirDisplay for tree::Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expr(x) => {
                x.fmt(f)?;
                f.write_char(';')
            }
            Self::Print(x) => x.fmt(f),
            Self::Loop(x) => x.fmt(f),
            Self::Dfer(x) => x.fmt(f),
            Self::Branch(x) => x.fmt(f),
            Self::Give(x) => x.fmt(f),
            Self::Fail(x) => x.fmt(f),
            Self::Skip(x) => x.fmt(f),
            Self::Done(x) => x.fmt(f),
        }
    }
}

impl MirDisplay for tree::Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(x) => x.fmt(f),
            Self::Access(x) => x.fmt(f),
            Self::Operation(x) => x.fmt(f),
        }
    }
}

impl MirDisplay for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => ().fmt(f),
            Self::Fail(x) => x.fmt(f),
            Self::Bool(x) => x.fmt(f),
            Self::Byte(x) => x.fmt(f),
            Self::Uint(x) => x.fmt(f),
            Self::Sint(x) => x.fmt(f),
            Self::Flpt(x) => x.fmt(f),
            Self::Char(x) => x.fmt(f),
            Self::Text(x) => x.fmt(f),
            Self::Var(x) => x.fmt(f),
            Self::View { start, count } => {
                write!(f, "[")?;
                start.fmt(f)?;
                write!(f, "; {count}]")
            }
        }
    }
}

impl MirDisplay for none {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("none")
    }
}

impl MirDisplay for fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fail{{{self}}}")
    }
}

impl MirDisplay for bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if *self { "true" } else { "fals" })
    }
}

impl MirDisplay for byte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}b")
    }
}

impl MirDisplay for uint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}u")
    }
}

impl MirDisplay for sint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}s")
    }
}

impl MirDisplay for flpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}f")
    }
}

impl MirDisplay for char {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl MirDisplay for text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl MirDisplay for EnvId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(id) = self;
        write!(f, "var{id:#X}")
    }
}

impl MirDisplay for tree::Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { what, args } = self;
        match what {
            tree::Callable::Builtin(op) => {
                enum Nary {
                    Nullary,
                    UnaryPre,
                    UnaryPost,
                    Binary,
                    Subscript,
                }
                let nary = match op {
                    tree::BuiltinOp::Inpt => Nary::Nullary,

                    tree::BuiltinOp::Exists | tree::BuiltinOp::Not | tree::BuiltinOp::Neg => {
                        Nary::UnaryPre
                    }

                    tree::BuiltinOp::Assign
                    | tree::BuiltinOp::And
                    | tree::BuiltinOp::Or
                    | tree::BuiltinOp::Xor
                    | tree::BuiltinOp::Nand
                    | tree::BuiltinOp::Nor
                    | tree::BuiltinOp::Xnor
                    | tree::BuiltinOp::Eq
                    | tree::BuiltinOp::Ne
                    | tree::BuiltinOp::Lt
                    | tree::BuiltinOp::Gt
                    | tree::BuiltinOp::Le
                    | tree::BuiltinOp::Ge
                    | tree::BuiltinOp::Add
                    | tree::BuiltinOp::Sub
                    | tree::BuiltinOp::Mul
                    | tree::BuiltinOp::Div
                    | tree::BuiltinOp::Rem
                    | tree::BuiltinOp::Pow
                    | tree::BuiltinOp::Shl
                    | tree::BuiltinOp::Shr
                    | tree::BuiltinOp::Rotl
                    | tree::BuiltinOp::Rotr => Nary::Binary,

                    tree::BuiltinOp::Index | tree::BuiltinOp::Slice => Nary::Subscript,
                };
                let sym = match op {
                    tree::BuiltinOp::Assign => "=",
                    tree::BuiltinOp::Inpt => "inpt",
                    tree::BuiltinOp::Exists => "!!",
                    tree::BuiltinOp::Index | tree::BuiltinOp::Slice => "",
                    tree::BuiltinOp::Not => "!",
                    tree::BuiltinOp::And => "&",
                    tree::BuiltinOp::Or => "|",
                    tree::BuiltinOp::Xor => "^",
                    tree::BuiltinOp::Nand => "!&",
                    tree::BuiltinOp::Nor => "!|",
                    tree::BuiltinOp::Xnor => "!^",
                    tree::BuiltinOp::Eq => "==",
                    tree::BuiltinOp::Ne => "!=",
                    tree::BuiltinOp::Lt => "<",
                    tree::BuiltinOp::Gt => ">",
                    tree::BuiltinOp::Le => "<=",
                    tree::BuiltinOp::Ge => ">=",
                    tree::BuiltinOp::Add => "+",
                    tree::BuiltinOp::Neg | tree::BuiltinOp::Sub => "-",
                    tree::BuiltinOp::Mul => "*",
                    tree::BuiltinOp::Div => "/",
                    tree::BuiltinOp::Rem => "%",
                    tree::BuiltinOp::Pow => "**",
                    tree::BuiltinOp::Shl => "<<",
                    tree::BuiltinOp::Shr => ">>",
                    tree::BuiltinOp::Rotl => "[<<]",
                    tree::BuiltinOp::Rotr => "[>>]",
                };
                match nary {
                    Nary::Nullary => {
                        let [] = args else { return Err(fmt::Error) };
                        f.write_str(sym)
                    }
                    Nary::UnaryPre => {
                        let [x] = args else { return Err(fmt::Error) };
                        f.write_str(sym)?;
                        x.fmt(f)
                    }
                    Nary::UnaryPost => {
                        let [x] = args else { return Err(fmt::Error) };
                        x.fmt(f)?;
                        f.write_str(sym)
                    }
                    Nary::Binary => {
                        let [a, b] = args else { return Err(fmt::Error) };
                        a.fmt(f)?;
                        f.write_str(sym)?;
                        b.fmt(f)
                    }
                    Nary::Subscript => {
                        let [x, i] = args else { return Err(fmt::Error) };
                        x.fmt(f)?;
                        f.write_char('[')?;
                        i.fmt(f)?;
                        f.write_char(']')
                    }
                }
            }

            // TODO
            tree::Callable::UserDefined(x) => x.fmt(f),
        }
    }
}

impl MirDisplay for tree::Print {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { msg } = self;
        f.write_str("prnt(")?;
        msg.fmt(f)?;
        f.write_str(");")
    }
}

impl MirDisplay for tree::Loop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { block } = self;
        f.write_str("loop")?;
        block.fmt(f)
    }
}

impl MirDisplay for tree::Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { stmts } = self;
        MirBlock::new(f).entries(stmts).finish()
    }
}

impl MirDisplay for tree::Dfer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            do_first,
            ret_param,
            on_pop,
        } = self;
        f.write_str("dfer")?;
        if let Some(ret_param) = ret_param {
            f.write_char('(')?;
            ret_param.fmt(f)?;
            f.write_char(')')?;
        }
        on_pop.fmt(f)?;
        let tree::Block { stmts } = do_first;
        for stmt in stmts {
            stmt.fmt(f)?;
        }
        Ok(())
    }
}

impl MirDisplay for tree::Branch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { cases, default } = self;
        let mut has_prev = false;
        for tree::Case { pred, then } in cases {
            if has_prev {
                f.write_str(" or ")?;
            } else {
                has_prev = true;
            }
            f.write_str("if ")?;
            pred.fmt(f)?;
            then.fmt(f)?;
        }
        if let Some(default) = default {
            f.write_str(" or ")?;
            default.fmt(f)?;
        }
        Ok(())
    }
}

impl MirDisplay for tree::Give {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { value } = self;
        f.write_str("give ")?;
        value.fmt(f)?;
        f.write_char(';')
    }
}

impl MirDisplay for tree::Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { value } = self;
        f.write_str("fail ")?;
        value.fmt(f)?;
        f.write_char(';')
    }
}

impl MirDisplay for tree::Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { label } = self;
        f.write_str("skip ")?;
        if let Some(label) = label {
            label.fmt(f)?;
        }
        f.write_char(';')
    }
}

impl MirDisplay for tree::Done {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { label, value } = self;
        f.write_str("done ")?;
        if let Some(label) = label {
            label.fmt(f)?;
        }
        value.fmt(f)?;
        f.write_char(';')
    }
}

impl MirDisplay for ! {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}
