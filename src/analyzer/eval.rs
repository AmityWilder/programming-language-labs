use crate::{
    Todo,
    analyzer::{
        Assignment, BlockStmt, Expr, ExprStmt, IfStmt, Literal, Operands, Operation, OrStmt,
        PrintStmt, Program, RunVariable, Statement, types::StaticType,
    },
    exec::{env::Environment, error::Failure},
    types::*,
};
#[cfg(not(debug_assertions))]
use std::marker::PhantomData;

pub trait PrimitiveType {
    const STATIC_TY: StaticType<'static>;
}
macro_rules! primitive_types {
    ($($Type:ty = $StaticType:ident;)*) => {$(
        impl PrimitiveType for $Type {
            const STATIC_TY: StaticType<'static> = StaticType::Primitive(super::types::PrimitiveType::$StaticType);
        }
    )*};
}
primitive_types! {
    none = None;
    nevr = Nevr;
    fail = Fail;
    byte = Byte;
    bool = Bool;
    uint = Uint;
    sint = Sint;
    flpt = Flpt;
    char = Char;
    text = Text;
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Value<'val, 'src> {
    pub data: &'val [u8],

    /// For ensuring type checker is sound
    #[cfg(debug_assertions)]
    tag_: StaticType<'src>,

    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

impl<'val, 'src> Value<'val, 'src> {
    fn new(data: &'val [u8], #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    pub(super) fn cast_from<T: PrimitiveType>(value: &'val T) -> Self {
        let len = std::mem::size_of_val(value);
        let data = (&raw const *value).cast();
        // SAFETY: `data` comes from a reference to valid memory of len `len`
        let data = unsafe { std::slice::from_raw_parts(data, len) };
        Self::new(
            data,
            #[cfg(debug_assertions)]
            T::STATIC_TY,
        )
    }

    /// # Safety
    /// `self` must hold a pointer to a legitimate bytestring representation of
    /// a valid instance of `T`, or a type that is safe to cast to `T`.
    pub(super) unsafe fn cast_into<T: PrimitiveType>(self) -> &'val T {
        debug_assert_eq!(
            T::STATIC_TY,
            self.tag_,
            "SOUNDNESS ERROR: attempting to extract a different type (left) from what was inserted (right)"
        );
        // length in bytes, not elements
        let len = self.data.len();
        debug_assert_eq!(
            len,
            std::mem::size_of::<T>(),
            "type sizes must match in order to cast"
        );
        let ptr = self.data.as_ptr().cast::<T>();
        debug_assert!(ptr.is_aligned(), "pointer should be aligned");
        // SAFETY: caller must uphold safety contract
        unsafe { &*ptr }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Default)]
pub struct ValueMut<'val, 'src> {
    pub data: &'val mut [u8],

    /// For ensuring type checker is sound
    #[cfg(debug_assertions)]
    tag_: StaticType<'src>,

    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

impl<'val, 'src> ValueMut<'val, 'src> {
    fn new(data: &'val mut [u8], #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    pub(super) fn cast_from<T: PrimitiveType>(value: &'val mut T) -> Self {
        let len = std::mem::size_of_val(value);
        let data = (&raw mut *value).cast();
        // SAFETY: `data` comes from a reference to valid memory of len `len`
        let data = unsafe { std::slice::from_raw_parts_mut(data, len) };
        Self::new(
            data,
            #[cfg(debug_assertions)]
            T::STATIC_TY,
        )
    }

    /// # Safety
    /// `self` must hold a pointer to a legitimate bytestring representation of
    /// a valid instance of `T`, or a type that is safe to cast to `T`.
    pub(super) unsafe fn cast_into<T: PrimitiveType>(self) -> &'val mut T {
        debug_assert_eq!(
            T::STATIC_TY,
            self.tag_,
            "SOUNDNESS ERROR: attempting to extract a different type (left) from what was inserted (right)"
        );
        // length in bytes, not elements
        let len = self.data.len();
        debug_assert_eq!(
            len,
            std::mem::size_of::<T>(),
            "type sizes must match in order to cast"
        );
        let ptr = self.data.as_mut_ptr().cast::<T>();
        debug_assert!(ptr.is_aligned(), "pointer should be aligned");
        // SAFETY: caller must uphold safety contract
        unsafe { &mut *ptr }
    }
}

pub trait Evaluate<'src> {
    fn eval<'val>(&'val self, env: &'val mut Environment) -> Result<Value<'val, 'src>, Failure>;
}

pub trait Execute<'src> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure>;
}

impl<'src> Evaluate<'src> for Assignment<'src, RunVariable<'src>> {
    fn eval<'val>(&'val self, env: &'val mut Environment) -> Result<Value<'val, 'src>, Failure> {
        let rhs = self.rhs.eval(env);
        let lvalue = &self.lvalue;
        // TODO
        Ok(Value::cast_from(&()))
    }
}

impl<'src> Evaluate<'src> for Operation<'src, RunVariable<'src>> {
    fn eval<'val>(&'val self, env: &'val mut Environment) -> Result<Value<'val, 'src>, Failure> {
        match &self.operands {
            Operands::Binary(lhs, rhs) => {
                let lhs = lhs.eval(env)?;
                let rhs = rhs.eval(env)?;
                todo!()
            }
            Operands::UnaryPre(rhs) => {
                let rhs = rhs.eval(env)?;
                todo!()
            }
            Operands::UnaryPost(lhs) => {
                let lhs = lhs.eval(env)?;
                todo!()
            }
        }
    }
}

impl<'src> Evaluate<'src> for Literal {
    fn eval<'val>(&'val self, _env: &'val mut Environment) -> Result<Value<'val, 'src>, Failure> {
        match self {
            Literal::Nevr => unimplemented!("nevr cannot be accessed"), // TBD: should this be a Failure instead?
            Literal::None => Ok(Value::cast_from(const { &() })),
            Literal::Fail(val) => Ok(Value::cast_from(val)),
            Literal::Bool(val) => Ok(Value::cast_from(val)),
            Literal::Byte(val) => Ok(Value::cast_from(val)),
            Literal::Uint(val) => Ok(Value::cast_from(val)),
            Literal::Sint(val) => Ok(Value::cast_from(val)),
            Literal::Flpt(val) => Ok(Value::cast_from(val)),
            Literal::Char(val) => Ok(Value::cast_from(val)),
            Literal::Text(val) => Ok(Value::cast_from(val)),
        }
    }
}

impl<'src> Evaluate<'src> for RunVariable<'src> {
    fn eval<'val>(&'val self, env: &'val mut Environment) -> Result<Value<'val, 'src>, Failure> {
        Ok(Value::new(
            env.stack()
                .get(self.range)
                .expect("should be guaranteed by static analysis"),
            #[cfg(debug_assertions)]
            self._ty.clone(),
        ))
    }
}

impl<'src> Evaluate<'src> for Expr<'src, RunVariable<'src>> {
    fn eval<'val>(&'val self, env: &'val mut Environment) -> Result<Value<'val, 'src>, Failure> {
        match self {
            Self::Assignment(x) => x.eval(env),
            Self::Op(x) => x.eval(env),
            Self::Literal(x) => x.eval(env),
            Self::Variable(x) => x.eval(env),
        }
    }
}

impl<'src> Execute<'src> for ExprStmt<'src, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        self.expr.eval(env).map(|_| ())
    }
}

impl<'src> Execute<'src> for PrintStmt<'src, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        // SAFETY: static analyzer must ensure soundness
        let msg = unsafe { self.expr.eval(env)?.cast_into::<text>() };
        print!("{msg}");
        Ok(())
    }
}

impl<'src> Execute<'src> for IfStmt<'src, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        let mut if_stmt = self;
        loop {
            // SAFETY: static analyzer must ensure soundness
            break if *unsafe { if_stmt.pred.eval(env)?.cast_into::<bool>() } {
                &if_stmt.then
            } else if let Some(or) = if_stmt.or.as_deref() {
                match or {
                    OrStmt::More(next) => {
                        if_stmt = next;
                        continue;
                    }
                    OrStmt::Last(block_stmt) => block_stmt,
                }
            } else {
                break Ok(());
            }
            .exec(env);
        }
    }
}

impl<'src> Execute<'src> for BlockStmt<'src, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.exec(env)?;
        }
        Ok(())
    }
}

impl<'src> Execute<'src> for Statement<'src, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        match self {
            Self::Expr(stmt) => stmt.exec(env),
            Self::Print(stmt) => stmt.exec(env),
            Self::If(stmt) => stmt.exec(env),
            Self::Block(stmt) => stmt.exec(env),
        }
    }
}

impl<'src> Execute<'src> for Program<'src> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.exec(env)?;
        }
        Ok(())
    }
}

pub fn execute(prgm: &Program<'_>) -> Result<(), Failure> {
    let mut env = Environment::with_capacity(1024, 1024); // TODO: how much memory do we give it?
    prgm.exec(&mut env)
}
