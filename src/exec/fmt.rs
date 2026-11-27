#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    clippy::todo,
    reason = "need to be aware of when the structure changes so its formatting can update"
)]
#![deny(unused_must_use, reason = "don't miss formatter errors")]

use super::{EnvId, Value, tree};
use crate::{
    exec::LblId,
    lex::{
        symbols::{BYTE_SUFFIX, SIGNED_SUFFIX, UNSIGNED_SUFFIX},
        token::{keyword::Keyword, punc::Punctuation},
    },
    types::{byte, fail, flpt, none, sint, text, uint},
};
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
        let mut has_prev = false;
        for stmt in stmts {
            if has_prev {
                f.write_char('\n')?;
            } else {
                has_prev = true;
            }
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
        f.write_str(Keyword::None.as_str())
    }
}

impl MirDisplay for fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{{{self}}}", Keyword::Fail)
    }
}

impl MirDisplay for bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if *self { Keyword::True } else { Keyword::Fals }.as_str())
    }
}

impl MirDisplay for byte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}{BYTE_SUFFIX}")
    }
}

impl MirDisplay for uint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}{UNSIGNED_SUFFIX}")
    }
}

impl MirDisplay for sint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}{SIGNED_SUFFIX}")
    }
}

impl MirDisplay for flpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
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
        write!(f, "var@{id:X}")
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
                    Call,
                }
                let nary = match op {
                    tree::BuiltinOp::Inpt => Nary::Nullary,

                    tree::BuiltinOp::Exists
                    | tree::BuiltinOp::Not
                    | tree::BuiltinOp::Neg
                    | tree::BuiltinOp::Szof => Nary::UnaryPre,

                    tree::BuiltinOp::Incr | tree::BuiltinOp::Decr => Nary::UnaryPost,

                    tree::BuiltinOp::Assign
                    | tree::BuiltinOp::AndAssign
                    | tree::BuiltinOp::OrAssign
                    | tree::BuiltinOp::XorAssign
                    | tree::BuiltinOp::NandAssign
                    | tree::BuiltinOp::NorAssign
                    | tree::BuiltinOp::XnorAssign
                    | tree::BuiltinOp::AddAssign
                    | tree::BuiltinOp::SubAssign
                    | tree::BuiltinOp::MulAssign
                    | tree::BuiltinOp::PowAssign
                    | tree::BuiltinOp::DivAssign
                    | tree::BuiltinOp::RemAssign
                    | tree::BuiltinOp::ShlAssign
                    | tree::BuiltinOp::ShrAssign
                    | tree::BuiltinOp::RotlAssign
                    | tree::BuiltinOp::RotrAssign
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

                    tree::BuiltinOp::Prnt => Nary::Call,
                };
                let sym = match op {
                    tree::BuiltinOp::Assign => Punctuation::Assign.as_str(),
                    tree::BuiltinOp::AndAssign => Punctuation::AndAssign.as_str(),
                    tree::BuiltinOp::OrAssign => Punctuation::OrAssign.as_str(),
                    tree::BuiltinOp::XorAssign => Punctuation::XorAssign.as_str(),
                    tree::BuiltinOp::NandAssign => Punctuation::NandAssign.as_str(),
                    tree::BuiltinOp::NorAssign => Punctuation::NorAssign.as_str(),
                    tree::BuiltinOp::XnorAssign => Punctuation::XnorAssign.as_str(),
                    tree::BuiltinOp::AddAssign => Punctuation::AddAssign.as_str(),
                    tree::BuiltinOp::SubAssign => Punctuation::SubAssign.as_str(),
                    tree::BuiltinOp::MulAssign => Punctuation::MulAssign.as_str(),
                    tree::BuiltinOp::PowAssign => Punctuation::PowAssign.as_str(),
                    tree::BuiltinOp::DivAssign => Punctuation::DivAssign.as_str(),
                    tree::BuiltinOp::RemAssign => Punctuation::RemAssign.as_str(),
                    tree::BuiltinOp::ShlAssign => Punctuation::ShlAssign.as_str(),
                    tree::BuiltinOp::ShrAssign => Punctuation::ShrAssign.as_str(),
                    tree::BuiltinOp::RotlAssign => Punctuation::RotlAssign.as_str(),
                    tree::BuiltinOp::RotrAssign => Punctuation::RotrAssign.as_str(),
                    tree::BuiltinOp::Inpt => Keyword::Inpt.as_str(),
                    tree::BuiltinOp::Prnt => Keyword::Prnt.as_str(),
                    tree::BuiltinOp::Exists => Punctuation::Exists.as_str(),
                    tree::BuiltinOp::Szof => Keyword::Szof.as_str(),
                    tree::BuiltinOp::Index | tree::BuiltinOp::Slice => "",
                    tree::BuiltinOp::Incr => Punctuation::Incr.as_str(),
                    tree::BuiltinOp::Decr => Punctuation::Decr.as_str(),
                    tree::BuiltinOp::Not => Punctuation::Not.as_str(),
                    tree::BuiltinOp::And => Punctuation::And.as_str(),
                    tree::BuiltinOp::Or => Punctuation::Or.as_str(),
                    tree::BuiltinOp::Xor => Punctuation::Xor.as_str(),
                    tree::BuiltinOp::Nand => Punctuation::Nand.as_str(),
                    tree::BuiltinOp::Nor => Punctuation::Nor.as_str(),
                    tree::BuiltinOp::Xnor => Punctuation::Xnor.as_str(),
                    tree::BuiltinOp::Eq => Punctuation::Eq.as_str(),
                    tree::BuiltinOp::Ne => Punctuation::Ne.as_str(),
                    tree::BuiltinOp::Lt => Punctuation::Lt.as_str(),
                    tree::BuiltinOp::Gt => Punctuation::Gt.as_str(),
                    tree::BuiltinOp::Le => Punctuation::Le.as_str(),
                    tree::BuiltinOp::Ge => Punctuation::Ge.as_str(),
                    tree::BuiltinOp::Add => Punctuation::Add.as_str(),
                    tree::BuiltinOp::Neg | tree::BuiltinOp::Sub => Punctuation::SubNeg.as_str(),
                    tree::BuiltinOp::Mul => Punctuation::Mul.as_str(),
                    tree::BuiltinOp::Div => Punctuation::Div.as_str(),
                    tree::BuiltinOp::Rem => Punctuation::Rem.as_str(),
                    tree::BuiltinOp::Pow => Punctuation::Pow.as_str(),
                    tree::BuiltinOp::Shl => Punctuation::Shl.as_str(),
                    tree::BuiltinOp::Shr => Punctuation::Shr.as_str(),
                    tree::BuiltinOp::Rotl => Punctuation::Rotl.as_str(),
                    tree::BuiltinOp::Rotr => Punctuation::Rotr.as_str(),
                };
                match nary {
                    Nary::Nullary => {
                        let [] = args else { return Err(fmt::Error) };
                        f.write_str(sym)
                    }
                    Nary::UnaryPre => {
                        let [x] = args else { return Err(fmt::Error) };
                        f.write_str(sym)?;
                        f.write_char('(')?;
                        x.fmt(f)?;
                        f.write_char(')')
                    }
                    Nary::UnaryPost => {
                        let [x] = args else { return Err(fmt::Error) };
                        f.write_char('(')?;
                        x.fmt(f)?;
                        f.write_char(')')?;
                        f.write_str(sym)
                    }
                    Nary::Binary => {
                        let [a, b] = args else { return Err(fmt::Error) };
                        f.write_char('(')?;
                        a.fmt(f)?;
                        f.write_char(')')?;
                        f.write_str(sym)?;
                        f.write_char('(')?;
                        b.fmt(f)?;
                        f.write_char(')')
                    }
                    Nary::Subscript => {
                        let [x, i] = args else { return Err(fmt::Error) };
                        x.fmt(f)?;
                        f.write_char('[')?;
                        i.fmt(f)?;
                        f.write_char(']')
                    }
                    Nary::Call => {
                        f.write_str(sym)?;
                        f.write_char('(')?;
                        let mut has_prev = false;
                        for arg in args {
                            if has_prev {
                                f.write_str(", ")?;
                            } else {
                                has_prev = true;
                            }
                            arg.fmt(f)?;
                        }
                        f.write_char(')')
                    }
                }
            }

            // TODO
            tree::Callable::UserDefined(x) => x.fmt(f),
        }
    }
}

impl MirDisplay for tree::Loop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { label, block } = self;
        if let Some(label) = label {
            f.write_char('.')?;
            label.fmt(f)?;
            f.write_str(": ")?;
        }
        write!(f, "{} ", Keyword::Loop)?;
        block.fmt(f)
    }
}

impl MirDisplay for LblId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(id) = self;
        write!(f, "lbl@{id:X}")
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
        f.write_str(Keyword::Dfer.as_str())?;
        if let Some(ret_param) = ret_param {
            f.write_char('(')?;
            ret_param.fmt(f)?;
            f.write_char(')')?;
        }
        f.write_char(' ')?;
        on_pop.fmt(f)?;
        let tree::Block { stmts } = do_first;
        for stmt in stmts {
            f.write_char('\n')?;
            stmt.fmt(f)?;
        }
        Ok(())
    }
}

impl MirDisplay for tree::Branch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cases(x) => x.fmt(f),
            Self::Table(x) => x.fmt(f),
        }
    }
}

impl MirDisplay for tree::Cases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { cases } = self;
        let mut has_prev = false;
        for case in cases {
            if has_prev {
                write!(f, " {} ", Keyword::Or)?;
            } else {
                has_prev = true;
            }
            case.fmt(f)?;
        }
        Ok(())
    }
}

impl MirDisplay for tree::Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { pred, then } = self;
        write!(f, "{} ", Keyword::If)?;
        pred.fmt(f)?;
        f.write_char(' ')?;
        then.fmt(f)
    }
}

impl MirDisplay for tree::JumpTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            expr,
            transformer,
            thens,
        } = self;
        write!(f, "{} ", Keyword::Pick)?;
        match transformer {
            tree::JumpLookup::Bools => {
                f.write_str("(")?;
                expr.fmt(f)?;
                f.write_str(") -:> uint")?;
            }

            tf @ (tree::JumpLookup::AdjacentBytes(_)
            | tree::JumpLookup::AdjacentUints(_)
            | tree::JumpLookup::AdjacentSints(_)
            | tree::JumpLookup::AdjacentChars(_)) => {
                f.write_str("(")?;
                expr.fmt(f)?;
                f.write_str(" - ")?;
                match tf {
                    tree::JumpLookup::AdjacentBytes(min) => min.fmt(f)?,
                    tree::JumpLookup::AdjacentUints(min) => min.fmt(f)?,
                    tree::JumpLookup::AdjacentSints(min) => min.fmt(f)?,
                    tree::JumpLookup::AdjacentChars(min) => min.fmt(f)?,
                    _ => unreachable!(),
                }
                f.write_str(")")?;
                if !matches!(tf, tree::JumpLookup::AdjacentUints(_)) {
                    f.write_str(" -:> uint")?;
                }
            }

            tree::JumpLookup::SparseBytes(_)
            | tree::JumpLookup::SparseUints(_)
            | tree::JumpLookup::SparseSints(_)
            | tree::JumpLookup::SparseChars(_) => return Err(fmt::Error), // TODO

            tree::JumpLookup::Strings(map) => {
                f.write_str("table@[")?;
                let mut has_prev = false;
                for k in map.keys() {
                    if has_prev {
                        f.write_str(", ")?;
                    } else {
                        has_prev = true;
                    }
                    write!(f, "{k:?}")?;
                }
                f.write_str("].find(")?;
                expr.fmt(f)?;
                f.write_str(")")?;
            }
        }
        f.write_str(" ")?;
        let mut f = MirBlock::new(f);
        for (i, x) in thens.iter().enumerate() {
            f.entry_with(|f| {
                write!(f, "{i} => ")?;
                x.fmt(f)
            });
        }
        f.finish()
    }
}

impl MirDisplay for tree::Give {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { value } = self;
        write!(f, "{} ", Keyword::Give)?;
        value.fmt(f)?;
        f.write_char(';')
    }
}

impl MirDisplay for tree::Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { value } = self;
        write!(f, "{} ", Keyword::Fail)?;
        value.fmt(f)?;
        f.write_char(';')
    }
}

impl MirDisplay for tree::Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { label } = self;
        f.write_str(Keyword::Skip.as_str())?;
        if let Some(label) = label {
            f.write_char(' ')?;
            label.fmt(f)?;
        }
        f.write_char(';')
    }
}

impl MirDisplay for tree::Done {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { label, value } = self;
        write!(f, "{} ", Keyword::Done)?;
        if let Some(label) = label {
            label.fmt(f)?;
            f.write_str(" ")?;
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
