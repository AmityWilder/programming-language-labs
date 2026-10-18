use crate::{
    analyzer::{
        Assignment, BlockStmt, Expr, ExprStmt, IfStmt, Literal, OrStmt, PrintStmt, Program,
        RunOperation, RunVariable, Statement, types::StaticType,
    },
    exec::{env::Environment, error::Failure},
    types::*,
};
#[cfg(not(debug_assertions))]
use std::marker::PhantomData;

// TODO: so many allocations :c

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

#[derive(Clone, PartialEq, PartialOrd, Default)]
pub struct OwnedValue<'src> {
    pub data: Box<[u8]>,

    /// For ensuring type checker is sound
    #[cfg(debug_assertions)]
    tag_: StaticType<'src>,

    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

impl std::fmt::Debug for OwnedValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'src> OwnedValue<'src> {
    fn new<T>(data: T, #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self
    where
        T: Into<Box<[u8]>>,
    {
        Self {
            data: data.into(),
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    fn as_ref(&self) -> Value<'_, 'src> {
        Value::new(
            &self.data,
            #[cfg(debug_assertions)]
            self.tag_.clone(),
        )
    }

    fn as_mut(&mut self) -> ValueMut<'_, 'src> {
        ValueMut::new(
            &mut self.data,
            #[cfg(debug_assertions)]
            self.tag_.clone(),
        )
    }
}

#[derive(Clone, PartialEq, PartialOrd, Default)]
pub struct Value<'val, 'src> {
    pub data: &'val [u8],

    /// For ensuring type checker is sound
    #[cfg(debug_assertions)]
    tag_: StaticType<'src>,

    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

impl std::fmt::Debug for Value<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(debug_assertions)]
        struct SpecValue<'a, 'b, 'c>(&'a Value<'b, 'c>);

        #[cfg(debug_assertions)]
        impl std::fmt::Debug for SpecValue<'_, '_, '_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // SAFETY: static analysis should ensure this
                unsafe {
                    let value: &dyn std::fmt::Debug = match &self.0.tag_ {
                        StaticType::Primitive(t) => {
                            use crate::analyzer::types::PrimitiveType;
                            match t {
                                PrimitiveType::None => self.0.clone().cast_into::<none>(),
                                PrimitiveType::Nevr => self.0.clone().cast_into::<nevr>(),
                                PrimitiveType::Fail => self.0.clone().cast_into::<fail>(),
                                PrimitiveType::Bool => self.0.clone().cast_into::<bool>(),
                                PrimitiveType::Byte => self.0.clone().cast_into::<byte>(),
                                PrimitiveType::Uint => self.0.clone().cast_into::<uint>(),
                                PrimitiveType::Sint => self.0.clone().cast_into::<sint>(),
                                PrimitiveType::Flpt => self.0.clone().cast_into::<flpt>(),
                                PrimitiveType::Char => self.0.clone().cast_into::<char>(),
                                PrimitiveType::Text => self.0.clone().cast_into::<text>(),
                            }
                        }
                        StaticType::View(_)
                        | StaticType::Arr(_)
                        | StaticType::Ptr(_)
                        | StaticType::OrTy(_)
                        | StaticType::Rec(_)
                        | StaticType::Sup(_)
                        | StaticType::Sub(_)
                        | StaticType::Cat(_)
                        | StaticType::Fun(_) => &"[unknown]",
                    };
                    value.fmt(f)
                }
            }
        }

        let mut f = f.debug_struct("Value");
        #[cfg(debug_assertions)]
        let f = f.field("tag_", &self.tag_).field("data", &SpecValue(self));
        f.field("data(raw)", &self.data).finish()
    }
}

impl<'val, 'src> Value<'val, 'src> {
    const fn new(data: &'val [u8], #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    fn into_owned(self) -> OwnedValue<'src> {
        OwnedValue::new(
            self.data,
            #[cfg(debug_assertions)]
            self.tag_,
        )
    }

    pub(super) const fn cast_from<T: PrimitiveType>(value: &'val T) -> Self {
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

#[derive(PartialEq, PartialOrd, Default)]
pub struct ValueMut<'val, 'src> {
    pub data: &'val mut [u8],

    /// For ensuring type checker is sound
    #[cfg(debug_assertions)]
    tag_: StaticType<'src>,

    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

impl std::fmt::Debug for ValueMut<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'val, 'src> ValueMut<'val, 'src> {
    const fn new(data: &'val mut [u8], #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    fn into_owned(self) -> OwnedValue<'src> {
        OwnedValue::new(
            self.data,
            #[cfg(debug_assertions)]
            self.tag_,
        )
    }

    fn as_ref<'subval>(&'subval self) -> Value<'subval, 'src> {
        Value {
            data: &*self.data,
            tag_: self.tag_.clone(),
        }
    }

    pub(super) const fn cast_from<T: PrimitiveType>(value: &'val mut T) -> Self {
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

    fn write_from(self, value: Value<'_, '_>) {
        debug_assert_eq!(
            value.tag_, self.tag_,
            "SOUNDNESS ERROR: attempting to write a different type (left) into a buffer (right)"
        );
        debug_assert_eq!(value.data.len(), self.data.len(), "runtime size mismatch");
        self.data.copy_from_slice(value.data);
    }
}

/// Yeilds a value, but does not mutate state
pub trait Evaluate<'src> {
    fn eval(&self, env: &Environment) -> Result<OwnedValue<'src>, Failure>;
}

/// Mutates state, does not yeild a value
pub trait Execute<'src> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure>;
}

impl<'src> Execute<'src> for Assignment<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        let lvalue = match &self.lvalue {
            Expr::Variable(var) => var,
            Expr::Literal(_) => {
                panic!("cannot assign to literal - should be caught by static analysis")
            }
            _ => todo!("mutable expressions"),
        };
        let rvalue = self.rhs.eval(env)?;
        env.stack_mut()
            .get_mut(lvalue.range)
            .expect("should be handled by static analysis")
            .copy_from_slice(&rvalue.data);
        Ok(())
    }
}

impl<'src> Evaluate<'src> for RunOperation<'src> {
    fn eval(&self, env: &Environment) -> Result<OwnedValue<'src>, Failure> {
        match *self {
            RunOperation::Binary(ref ty_, ref def, out_size, ref lhs, ref rhs) => {
                let lhs = lhs.eval(env)?;
                let rhs = rhs.eval(env)?;
                let mut out = OwnedValue::new(
                    vec![0xee; out_size],
                    #[cfg(debug_assertions)]
                    ty_.clone(),
                );
                def(env, out.as_mut(), lhs.as_ref(), rhs.as_ref()).map(|()| out)
            }
            RunOperation::UnaryPre(ref ty_, ref def, out_size, ref rhs) => {
                let rhs = rhs.eval(env)?;
                let mut out = OwnedValue::new(
                    vec![0xee; out_size],
                    #[cfg(debug_assertions)]
                    ty_.clone(),
                );
                def(env, out.as_mut(), rhs.as_ref()).map(|()| out)
            }
            RunOperation::UnaryPost(ref ty_, ref def, out_size, ref lhs) => {
                let lhs = lhs.eval(env)?;
                let mut out = OwnedValue::new(
                    vec![0xee; out_size],
                    #[cfg(debug_assertions)]
                    ty_.clone(),
                );
                def(env, out.as_mut(), lhs.as_ref()).map(|()| out)
            }
        }
    }
}

impl<'src> Evaluate<'src> for Literal {
    fn eval<'val>(&'val self, _env: &'val Environment) -> Result<OwnedValue<'src>, Failure> {
        match self {
            Literal::Nevr => unimplemented!("nevr cannot be accessed"), // TBD: should this be a Failure instead?
            Literal::None => Ok(Value::cast_from(&()).into_owned()),
            Literal::Fail(val) => Ok(Value::cast_from(val).into_owned()),
            Literal::Bool(val) => Ok(Value::cast_from(val).into_owned()),
            Literal::Byte(val) => Ok(Value::cast_from(val).into_owned()),
            Literal::Uint(val) => Ok(Value::cast_from(val).into_owned()),
            Literal::Sint(val) => Ok(Value::cast_from(val).into_owned()),
            Literal::Flpt(val) => Ok(Value::cast_from(val).into_owned()),
            Literal::Char(val) => Ok(Value::cast_from(val).into_owned()),
            Literal::Text(val) => Ok(Value::cast_from(val).into_owned()),
        }
    }
}

impl<'src> Evaluate<'src> for RunVariable<'src> {
    fn eval(&self, env: &Environment) -> Result<OwnedValue<'src>, Failure> {
        Ok(OwnedValue::new(
            env.stack()
                .get(self.range)
                .expect("should be guaranteed by static analysis"),
            #[cfg(debug_assertions)]
            self.ty_.clone(),
        ))
    }
}

impl<'src> Evaluate<'src> for Expr<RunOperation<'src>, RunVariable<'src>> {
    fn eval(&self, env: &Environment) -> Result<OwnedValue<'src>, Failure> {
        match self {
            Self::Assignment(_) => unreachable!("should be handled by outer statement"),
            Self::Op(x) => x.eval(env),
            Self::Literal(x) => x.eval(env),
            Self::Variable(x) => x.eval(env),
        }
    }
}

impl<'src> Execute<'src> for ExprStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        match &self.expr {
            Expr::Assignment(assignment) => assignment.exec(env),

            expr => expr.eval(env).map(|val| {
                #[cfg(debug_assertions)]
                if val.tag_ != StaticType::Primitive(crate::analyzer::types::PrimitiveType::None) {
                    const WARN_MUST_USE: bool = true;
                    if WARN_MUST_USE {
                        println!(
                            "warning: expression evaluating to {} is having its output ignored\n value: {val:?}\n expr: {expr:?}",
                            val.tag_
                        );
                    }
                }
            }),
        }
    }
}

impl<'src> Execute<'src> for PrintStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        let res = self.expr.eval(env)?;
        // SAFETY: static analyzer must ensure soundness
        let msg = unsafe { res.as_ref().cast_into::<text>() };
        print!("{msg}");
        Ok(())
    }
}

impl<'src> Execute<'src> for IfStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        let mut if_stmt = self;
        loop {
            let res = if_stmt.pred.eval(env)?;
            // SAFETY: static analyzer must ensure soundness
            break if *unsafe { res.as_ref().cast_into::<bool>() } {
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

impl<'src> Execute<'src> for BlockStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.exec(env)?;
        }
        Ok(())
    }
}

impl<'src> Execute<'src> for Statement<RunOperation<'src>, RunVariable<'src>> {
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
