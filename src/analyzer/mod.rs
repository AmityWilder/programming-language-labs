use crate::{
    analyzer::{
        eval::{Value, ValueMut},
        static_env::{EnvItem, OpOverload, OperandTypes, ScopeItem, StaticEnvironment, VarType},
        types::{
            AbstractType, FunShape, FunType, PrimitiveType, StaticType, TypeCompatible, TypeSize,
        },
    },
    error::{ContextError, ErrorType, OpSide},
    exec::{env::Environment, error::Failure},
    grammar::ast::{self, VarKeyword},
    scanner::token::value::{Keyword, LexValue, Operator},
    types::*,
};
#[cfg(not(debug_assertions))]
use std::marker::PhantomData;
use std::{collections::HashMap, range::Range};

pub mod builtin_ops;
pub mod eval;
pub mod static_env;
pub mod types;

pub type Result<'src, T> = std::result::Result<T, Vec<ContextError<'src>>>;

pub trait Analyze<'src> {
    /// May use [`None`] if the statement creates no event (e.g. declaring a variable without assigning it)
    type Ouptut;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut>;
}

pub trait Expression<'src> {
    type Output<'ty>: 'ty + TypeCompatible<'src>
    where
        'src: 'ty,
        Self: 'ty;

    /// The type output by the expression
    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>>;
}

pub trait CementVars<'src> {
    /// The version of `Self` with [`Operation<'src>, Variable<'src>`] replaced with [`RunOperation< RunVariable<'src>>, RunVariable<'src>`]
    type Concrete;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete;
}

#[derive(Debug)]
pub enum FnDef<O, V> {
    LangOpB(BinaryOpFn),
    LangOpU(UnaryOpFn),
    UserDef(BlockStmt<O, V>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnRef<'src> {
    pub ident: &'src str,
}

impl<'src> Expression<'src> for FnRef<'src> {
    type Output<'ty>
        = &'ty FunShape<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        env.lookup(self.ident).find_map(|x| match &x.val {
            EnvItem::Fun((t, _)) => Some(t), // TBD: we have access to the name here, but should we use it?
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operands<O, V> {
    Binary(Expr<O, V>, Expr<O, V>),
    UnaryPre(Expr<O, V>),
    UnaryPost(Expr<O, V>),
}

impl<'src> Operands<Operation<'src>, Variable<'src>> {
    pub fn get_types(&self, env: &StaticEnvironment<'src>) -> Option<OperandTypes<'src>> {
        Some(match self {
            Self::Binary(lhs, rhs) => OperandTypes::Binary(
                lhs.get_type(env).map(|t| t.to_abstract())?,
                rhs.get_type(env).map(|t| t.to_abstract())?,
            ),
            Self::UnaryPre(rhs) => {
                OperandTypes::UnaryPre(rhs.get_type(env).map(|t| t.to_abstract())?)
            }
            Self::UnaryPost(lhs) => {
                OperandTypes::UnaryPost(lhs.get_type(env).map(|t| t.to_abstract())?)
            }
        })
    }
}

impl<'src> CementVars<'src> for Operands<Operation<'src>, Variable<'src>> {
    type Concrete = Operands<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Binary(l, r) => Operands::Binary(l.cement_vars(ranges), r.cement_vars(ranges)),
            Self::UnaryPre(x) => Operands::UnaryPre(x.cement_vars(ranges)),
            Self::UnaryPost(x) => Operands::UnaryPost(x.cement_vars(ranges)),
        }
    }
}

pub type BinaryOpFn = for<'val, 'srcf> fn(
    &'val Environment,
    ValueMut<'val, 'srcf>,
    Value<'val, 'srcf>,
    Value<'val, 'srcf>,
) -> std::result::Result<(), Failure>;

pub type UnaryOpFn = for<'val, 'srcf> fn(
    &'val Environment,
    ValueMut<'val, 'srcf>,
    Value<'val, 'srcf>,
) -> std::result::Result<(), Failure>;

#[derive(Debug, Clone, PartialEq)]
pub enum Operation<'src> {
    Binary(
        (FunShape<'src>, BinaryOpFn),
        Expr<Self, Variable<'src>>,
        Expr<Self, Variable<'src>>,
    ),
    UnaryPre((FunShape<'src>, UnaryOpFn), Expr<Self, Variable<'src>>),
    UnaryPost((FunShape<'src>, UnaryOpFn), Expr<Self, Variable<'src>>),
}

#[derive(Debug, Clone)]
pub enum RunOperation<'src> {
    Binary(
        #[cfg(debug_assertions)] StaticType<'src>,
        #[cfg(not(debug_assertions))] (),
        BinaryOpFn,
        usize,
        Expr<Self, RunVariable<'src>>,
        Expr<Self, RunVariable<'src>>,
    ),
    UnaryPre(
        #[cfg(debug_assertions)] StaticType<'src>,
        #[cfg(not(debug_assertions))] (),
        UnaryOpFn,
        usize,
        Expr<Self, RunVariable<'src>>,
    ),
    UnaryPost(
        #[cfg(debug_assertions)] StaticType<'src>,
        #[cfg(not(debug_assertions))] (),
        UnaryOpFn,
        usize,
        Expr<Self, RunVariable<'src>>,
    ),
}

impl<'src> Expression<'src> for Operation<'src> {
    type Output<'ty>
        = &'ty StaticType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(
            &match self {
                Self::Binary((f, _), _, _)
                | Self::UnaryPre((f, _), _)
                | Self::UnaryPost((f, _), _) => f,
            }
            .ret,
        )
    }
}

impl<'src> CementVars<'src> for Operation<'src> {
    type Concrete = RunOperation<'src>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Binary((f, def), l, r) => {
                let size = f
                    .ret
                    .size()
                    .unwrap_or_else(|| panic!("size should be known by this point\n type: {f}"))
                    .unwrap_or_else(|e| panic!("error determining type size: {e}\n type: {f}"));
                RunOperation::Binary(
                    #[cfg(debug_assertions)]
                    f.ret,
                    #[cfg(not(debug_assertions))]
                    (),
                    def,
                    size,
                    l.cement_vars(ranges),
                    r.cement_vars(ranges),
                )
            }
            Self::UnaryPre((f, def), x) => {
                let size = f
                    .ret
                    .size()
                    .unwrap_or_else(|| panic!("size should be known by this point\n type: {f}"))
                    .unwrap_or_else(|e| panic!("error determining type size: {e}\n type: {f}"));
                RunOperation::UnaryPre(
                    #[cfg(debug_assertions)]
                    f.ret,
                    #[cfg(not(debug_assertions))]
                    (),
                    def,
                    size,
                    x.cement_vars(ranges),
                )
            }
            Self::UnaryPost((f, def), x) => {
                let size = f
                    .ret
                    .size()
                    .unwrap_or_else(|| panic!("size should be known by this point\n type: {f}"))
                    .unwrap_or_else(|e| panic!("error determining type size: {e}\n type: {f}"));
                RunOperation::UnaryPost(
                    #[cfg(debug_assertions)]
                    f.ret,
                    #[cfg(not(debug_assertions))]
                    (),
                    def,
                    size,
                    x.cement_vars(ranges),
                )
            }
        }
    }
}

// TODO: what about literals for custom types and arrays?
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    None,
    Nevr,
    Fail(fail),
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    Flpt(flpt),
    Char(char),
    Text(text),
}

impl<'src> Expression<'src> for Literal {
    type Output<'ty>
        = PrimitiveType
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(match self {
            Self::None => PrimitiveType::None,
            Self::Nevr => PrimitiveType::Nevr,
            Self::Fail(_) => PrimitiveType::Fail,
            Self::Bool(_) => PrimitiveType::Bool,
            Self::Byte(_) => PrimitiveType::Byte,
            Self::Uint(_) => PrimitiveType::Uint,
            Self::Sint(_) => PrimitiveType::Sint,
            Self::Flpt(_) => PrimitiveType::Flpt,
            Self::Char(_) => PrimitiveType::Char,
            Self::Text(_) => PrimitiveType::Text,
        })
    }
}

/// Operation<'src>, Variable<'src> variable
///
/// Has no actual memory location, since its type may not have been determined yet
#[derive(Debug, Clone, PartialEq)]
pub struct Variable<'src> {
    pub unique_id: usize,
    pub ident: &'src str,
    pub is_global: bool,
    #[cfg(debug_assertions)]
    _ty: Option<AbstractType<'src>>,
}

impl<'src> Expression<'src> for Variable<'src> {
    type Output<'ty>
        = VarType<'src, 'ty>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        env.lookup(self.ident)
            .next()
            .and_then(|x| x.val.get_type(env))
    }
}

impl<'src> CementVars<'src> for Variable<'src> {
    type Concrete = RunVariable<'src>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        RunVariable {
            range: *ranges
                .get(self.unique_id)
                .expect("unique_id missing from ranges"),
            #[cfg(debug_assertions)]
            ty_: match self
                ._ty
                .expect("all types should be determined by this point")
            {
                AbstractType::Concrete(ty) => ty,
                t => panic!("all variables should be concrete by this point\n type: {t}"),
            },
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }
}

/// Runtime variable
#[derive(Debug, Clone, PartialEq)]
pub struct RunVariable<'src> {
    range: Range<usize>,
    #[cfg(debug_assertions)]
    ty_: StaticType<'src>,
    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprType<'src, 'ty> {
    Static(&'ty StaticType<'src>),
    Literal(PrimitiveType),
    Variable(VarType<'src, 'ty>),
}

impl<'src> TypeCompatible<'src> for ExprType<'src, '_> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        match self {
            Self::Static(t) => t.is_compatible(other),
            Self::Literal(t) => t.is_compatible(other),
            Self::Variable(t) => t.is_compatible(other),
        }
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        match self {
            Self::Static(t) => t.to_abstract(),
            Self::Literal(t) => t.to_abstract(),
            Self::Variable(t) => t.to_abstract(),
        }
    }
}
impl std::fmt::Display for ExprType<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(t) => t.fmt(f),
            Self::Literal(t) => t.fmt(f),
            Self::Variable(t) => t.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment<O, V> {
    /// Must evaluate to mutable
    pub lvalue: Expr<O, V>,
    pub rhs: Expr<O, V>,
}

impl<'src> Expression<'src> for Assignment<Operation<'src>, Variable<'src>> {
    type Output<'ty>
        = PrimitiveType
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(PrimitiveType::None)
    }
}

impl<'src> CementVars<'src> for Assignment<Operation<'src>, Variable<'src>> {
    type Concrete = Assignment<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        Assignment {
            lvalue: self.lvalue.cement_vars(ranges),
            rhs: self.rhs.cement_vars(ranges),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<O, V> {
    Assignment(Box<Assignment<O, V>>),
    Op(Box<O>),
    Literal(Literal),
    Variable(V),
}

impl<'src> Expression<'src> for Expr<Operation<'src>, Variable<'src>> {
    type Output<'ty>
        = ExprType<'src, 'ty>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            Self::Assignment(x) => x.get_type(env).map(ExprType::Literal),
            Self::Op(x) => x.get_type(env).map(ExprType::Static),
            Self::Literal(x) => x.get_type(env).map(ExprType::Literal),
            Self::Variable(x) => x.get_type(env).map(ExprType::Variable),
        }
    }
}

impl<'src> Analyze<'src> for ast::Expr<'src> {
    type Ouptut = Expr<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Binary(x) => x.analyze(source, env),
            Self::Unary(x) => x.analyze(source, env).map(Box::new).map(Expr::Op),
            Self::Literal(x) => x.analyze(source, env).map(Expr::Literal),
            Self::Variable(x) => x.analyze(source, env),
            Self::Grouping(x) => x.expr.analyze(source, env),
            Self::Type(_) => todo!(),
        }
    }
}

impl<'src> CementVars<'src> for Expr<Operation<'src>, Variable<'src>> {
    type Concrete = Expr<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Assignment(x) => Expr::Assignment(Box::new(x.cement_vars(ranges))),
            Self::Op(x) => Expr::Op(Box::new(x.cement_vars(ranges))),
            Self::Literal(x) => Expr::Literal(x),
            Self::Variable(x) => Expr::Variable(x.cement_vars(ranges)),
        }
    }
}

impl<'src> Analyze<'src> for ast::Literal<'src> {
    type Ouptut = Literal;

    fn analyze(
        &self,
        source: &'src str,
        _env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self.value.val {
            LexValue::UIntLiteral(x) => Ok(Literal::Uint(x)),
            LexValue::SIntLiteral(x) => Ok(Literal::Sint(x)),
            LexValue::FlptLiteral(x) => Ok(Literal::Flpt(x)),
            LexValue::CharLiteral(x) => Ok(Literal::Char(x.ch)),
            LexValue::TextLiteral(x) => Ok(Literal::Text(
                x.process()
                    .map_err(|e| vec![ContextError::token_error(source, Some(&self.value), e)])?
                    .to_string(),
            )),
            LexValue::BoolLiteral(x) => Ok(Literal::Bool(x)),
            LexValue::Keyword(Keyword::None) => Ok(Literal::None),
            _ => unimplemented!("should be handled by grammar"),
        }
    }
}

impl<'src> Analyze<'src> for ast::Variable<'src> {
    type Ouptut = Expr<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let ident = self.ident.lex;
        env.lookup(ident)
            .find_map(|item| match item.val {
                EnvItem::Var(ref x, is_global) => Some(Expr::Variable(Variable {
                    unique_id: item.id,
                    ident,
                    is_global,
                    #[cfg(debug_assertions)]
                    _ty: x.as_ref().map(TypeCompatible::to_abstract),
                })),
                _ => None,
            })
            .ok_or_else(|| {
                vec![ContextError::token_error(
                    source,
                    Some(&self.ident),
                    ErrorType::UnknownVar,
                )]
            })
    }
}

impl<'src> Analyze<'src> for ast::Unary<'src> {
    type Ouptut = Operation<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        self.operand.analyze(source, env).and_then(|operand| {
            let Some(ty) = operand.get_type(env).map(|t| t.to_abstract()) else {
                return Err(vec![ContextError::node_error(
                    source,
                    &self.operand,
                    ErrorType::TypeAmbiguity(Box::new(operand)),
                )]);
            };
            let (operands, operand_types) = match self.side {
                OpSide::Left => (Operands::UnaryPost(operand), OperandTypes::UnaryPost(ty)),
                OpSide::Right => (Operands::UnaryPre(operand), OperandTypes::UnaryPre(ty)),
            };
            env.lookup(&self.op.val)
                .map(|item| match &item.val {
                    EnvItem::Op(overload) => overload,
                    _ => unreachable!("only operators should be stored with operator keys"),
                })
                .find(|overload| overload.operands == operand_types)
                .map(|overload| {
                    let def = match overload.def {
                        FnDef::LangOpU(def) => def,
                        FnDef::LangOpB(_) | FnDef::UserDef(_) => unimplemented!(),
                    };
                    match operands {
                        Operands::Binary(_, _) => unreachable!(),
                        Operands::UnaryPre(r) => {
                            Operation::UnaryPre((overload.fun.clone(), def), r)
                        }
                        Operands::UnaryPost(l) => {
                            Operation::UnaryPre((overload.fun.clone(), def), l)
                        }
                    }
                })
                .ok_or_else(|| vec![todo!("error: no such operator defined")])
        })
    }
}

impl<'src> Analyze<'src> for ast::Binary<'src> {
    type Ouptut = Expr<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let mut lhs = self.lhs.analyze(source, env)?;
        let rhs = self.rhs.analyze(source, env)?;
        let l_ty = lhs.get_type(env).map(|t| t.to_abstract());
        let r_ty = rhs.get_type(env).map(|t| t.to_abstract());
        if self.op.val == Operator::Assign {
            // TODO: cases where lhs is assignable in general, not specifically a variable
            if let Expr::Variable(var) = &mut lhs {
                match (l_ty, r_ty) {
                    (Some(l_ty), Some(r_ty)) => {
                        // require rhs to be the same type as lhs
                        r_ty.expect_type(&l_ty)
                            .map_err(|e| vec![ContextError::node_error(source, &self.rhs, e)])?;
                    }
                    (None, Some(r_ty)) => {
                        // update lhs type to match rhs
                        let Some(&mut ScopeItem {
                            id,
                            val: EnvItem::Var(ref mut var_ty, _),
                            ..
                        }) = env.lookup_mut(var.ident).next()
                        else {
                            unreachable!(
                                "Expr::Variable should only result from a StaticVar::Var present in `env`"
                            );
                        };
                        let r_ty_size = r_ty.size();
                        *var_ty = Some(r_ty);
                        #[cfg(debug_assertions)]
                        {
                            var._ty = var_ty.clone();
                        }
                        if let Some(size) = r_ty_size {
                            match size {
                                Ok(size) => {
                                    env.update_size(id, size).expect("guarded by match arm");
                                }
                                Err(e) => return Err(vec![todo!("error: {e}")]),
                            }
                        }
                    }
                    (Some(_l_ty), None) => todo!("re-analyze rhs knowing its output type"), // TBD: how will this be done?
                    (None, None) => {
                        // type ambiguity error
                        // TBD: which side should be considered ambiguous here?
                        return Err(vec![ContextError::node_error(
                            source,
                            &self.lhs,
                            ErrorType::TypeAmbiguity(Box::new(lhs)),
                        )]);
                    }
                }
                Ok(Expr::Assignment(Box::new(Assignment { lvalue: lhs, rhs })))
            } else {
                todo!("error: cannot assign twice to immutable")
            }
        } else {
            match (l_ty, r_ty) {
                (Some(l_ty), Some(r_ty)) => {
                    let operands = Operands::Binary(lhs, rhs);
                    let operand_types = OperandTypes::Binary(l_ty, r_ty);
                    let op = &self.op.val;
                    env.lookup(op)
                        .map(|item| match &item.val {
                            EnvItem::Op(overload) => overload,
                            _ => unreachable!("only operators should be stored with operator keys"),
                        })
                        .find(|overload| overload.operands == operand_types)
                        .ok_or_else(|| {
                            todo!(
                                "error: no such operator defined\n operator: {op:?}\n \
                                operand types: {operand_types:?}\n \
                                operands: {operands:?}\n \
                                supported operands: {:#?}",
                                env.lookup(op)
                                    .map(|item| match &item.val {
                                        EnvItem::Op(overload) => overload,
                                        _ => unreachable!(
                                            "only operators should be stored with operator keys"
                                        ),
                                    })
                                    .map(|t| t.fun.to_string())
                                    .collect::<Vec<_>>()
                            );
                            Vec::new()
                        })
                        .map(|overload| {
                            let def = match &overload.def {
                                FnDef::LangOpB(def) => *def,
                                FnDef::LangOpU(_) | FnDef::UserDef(_) => {
                                    unimplemented!()
                                }
                            };
                            Expr::Op(Box::new(match operands {
                                Operands::Binary(l, r) => {
                                    Operation::Binary((overload.fun.clone(), def), l, r)
                                }
                                _ => unreachable!(),
                            }))
                        })
                }

                (l_ty, r_ty) => Err(std::iter::chain(
                    l_ty.is_none().then(|| {
                        ContextError::node_error(
                            source,
                            &self.lhs,
                            ErrorType::TypeAmbiguity(Box::new(lhs)),
                        )
                    }),
                    r_ty.is_none().then(|| {
                        ContextError::node_error(
                            source,
                            &self.rhs,
                            ErrorType::TypeAmbiguity(Box::new(rhs)),
                        )
                    }),
                )
                .collect::<Vec<ContextError<'src>>>()),
            }
        }
    }
}

/// An [`Expr`] that returns [`none`]
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt<O, V> {
    expr: Expr<O, V>,
}

impl<'src> Analyze<'src> for ast::ExprStmt<'src> {
    type Ouptut = ExprStmt<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        self.expr.analyze(source, env).and_then(|expr| {
            if let Some(ty) = expr.get_type(env) {
                // TBD: can this be a "must_use" lint instead?
                const REQUIRE_NONE: bool = false;
                if REQUIRE_NONE {
                    ty.expect_type(&AbstractType::Concrete(StaticType::Primitive(
                        PrimitiveType::None,
                    )))
                    .map_err(|e| vec![ContextError::node_error(source, &self.expr, e)])?;
                }
                Ok(ExprStmt { expr })
            } else {
                Err(vec![ContextError::node_error(
                    source,
                    &self.expr,
                    ErrorType::TypeAmbiguity(Box::new(expr)),
                )])
            }
        })
    }
}

impl<'src> CementVars<'src> for ExprStmt<Operation<'src>, Variable<'src>> {
    type Concrete = ExprStmt<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        ExprStmt {
            expr: self.expr.cement_vars(ranges),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt<O, V> {
    expr: Expr<O, V>,
}

impl<'src> Analyze<'src> for ast::PrintStmt<'src> {
    type Ouptut = PrintStmt<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        self.group.expr.analyze(source, env).and_then(|expr| {
            if let Some(ty) = expr.get_type(env) {
                ty.expect_type(&AbstractType::Concrete(StaticType::Primitive(
                    PrimitiveType::Text,
                )))
                .map_err(|_| vec![todo!()])
                .map(|()| PrintStmt { expr })
            } else {
                Err(todo!(
                    "error: expression has no type\n expression: {expr:?}"
                ))
            }
        })
    }
}

impl<'src> CementVars<'src> for PrintStmt<Operation<'src>, Variable<'src>> {
    type Concrete = PrintStmt<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        PrintStmt {
            expr: self.expr.cement_vars(ranges),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrStmt<O, V> {
    More(IfStmt<O, V>),
    Last(BlockStmt<O, V>),
}

impl<'src> Expression<'src> for OrStmt<Operation<'src>, Variable<'src>> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            Self::More(stmt) => stmt.get_type(env),
            Self::Last(stmt) => stmt.get_type(env),
        }
    }
}

impl<'src> CementVars<'src> for OrStmt<Operation<'src>, Variable<'src>> {
    type Concrete = OrStmt<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::More(stmt) => OrStmt::More(stmt.cement_vars(ranges)),
            Self::Last(stmt) => OrStmt::Last(stmt.cement_vars(ranges)),
        }
    }
}

impl<'src> Analyze<'src> for ast::OrBody<'src> {
    type Ouptut = OrStmt<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::More(stmt) => stmt.analyze(source, env).map(OrStmt::More),
            Self::Last(stmt) => stmt.analyze(source, env).map(OrStmt::Last),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt<O, V> {
    /// Must output [`bool`]
    pub pred: Expr<O, V>,
    pub then: BlockStmt<O, V>,
    pub or: Option<Box<OrStmt<O, V>>>,
}

impl<'src> Expression<'src> for IfStmt<Operation<'src>, Variable<'src>> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        let ty = self.then.get_type(env);
        #[cfg(debug_assertions)]
        if let Some(then_ty) = ty
            && let Some(or) = &self.or
        {
            let or_ty = or.get_type(env);
            // TODO: this needs to be an error somewhere
            debug_assert_eq!(or_ty, Some(then_ty), "branches should return the same type");
        }
        ty
    }
}

impl<'src> Analyze<'src> for ast::IfStmt<'src> {
    type Ouptut = IfStmt<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let pred = self.pred.analyze(source, env)?;
        if let Some(ty) = pred.get_type(env) {
            ty.expect_type(&AbstractType::Concrete(StaticType::Primitive(
                PrimitiveType::Bool,
            )))
            .map_err(|_| vec![todo!()])
            .and_then(|()| {
                Ok(IfStmt {
                    pred,
                    then: self.then.analyze(source, env)?,
                    or: self
                        .or
                        .as_ref()
                        .map(|x| x.body.analyze(source, env))
                        .transpose()?
                        .map(Box::new),
                })
            })
        } else {
            Err(todo!("error: predicate has no type\n predicate: {pred:?}"))
        }
    }
}

impl<'src> CementVars<'src> for IfStmt<Operation<'src>, Variable<'src>> {
    type Concrete = IfStmt<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        IfStmt {
            pred: self.pred.cement_vars(ranges),
            then: self.then.cement_vars(ranges),
            or: self.or.map(|or| Box::new(or.cement_vars(ranges))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt<O, V> {
    pub stmts: Vec<Statement<O, V>>,
}

impl<'src> Expression<'src> for BlockStmt<Operation<'src>, Variable<'src>> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        self.stmts.last().map_or(
            Some(&AbstractType::Concrete(StaticType::Primitive(
                PrimitiveType::None,
            ))),
            |stmt| stmt.get_type(env),
        )
        // TODO: find all cases where the block can be returned from
    }
}

impl<'src> Analyze<'src> for ast::BlockStmt<'src> {
    type Ouptut = BlockStmt<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        env.push_scope().unwrap_or_else(|| {
            todo!("warning: cannot push a scope here, need to be in a function");
        });
        self.body
            .iter()
            .filter_map(|decl| decl.analyze(source, env).transpose())
            .collect::<Result<'src, Vec<_>>>()
            .map(|stmts| BlockStmt { stmts })
            .inspect(|_| {
                // TODO: drop each variable in scope
                if env.pop_scope().is_none() {
                    todo!("scope pushes and pops should be balanced - tried to pop global scope");
                }
            })
    }
}

impl<'src> CementVars<'src> for BlockStmt<Operation<'src>, Variable<'src>> {
    type Concrete = BlockStmt<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        BlockStmt {
            stmts: self
                .stmts
                .into_iter()
                .map(|stmt| stmt.cement_vars(ranges))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<O, V> {
    Expr(ExprStmt<O, V>),
    Print(PrintStmt<O, V>),
    If(IfStmt<O, V>),
    Block(BlockStmt<O, V>),
}

impl<'src> Expression<'src> for Statement<Operation<'src>, Variable<'src>> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty,
        Self: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            Self::Expr(_) | Self::Print(_) => Some(&AbstractType::Concrete(StaticType::Primitive(
                PrimitiveType::None,
            ))),
            Self::If(stmt) => stmt.get_type(env),
            Self::Block(stmt) => stmt.get_type(env),
        }
    }
}

impl<'src> Analyze<'src> for ast::Statement<'src> {
    type Ouptut = Statement<Operation<'src>, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Expr(stmt) => stmt.analyze(source, env).map(Statement::Expr),
            Self::Print(stmt) => stmt.analyze(source, env).map(Statement::Print),
            Self::If(stmt) => stmt.analyze(source, env).map(Statement::If),
            Self::Block(stmt) => stmt.analyze(source, env).map(Statement::Block),
        }
    }
}

impl<'src> CementVars<'src> for Statement<Operation<'src>, Variable<'src>> {
    type Concrete = Statement<RunOperation<'src>, RunVariable<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Expr(stmt) => Statement::Expr(stmt.cement_vars(ranges)),
            Self::Print(stmt) => Statement::Print(stmt.cement_vars(ranges)),
            Self::If(stmt) => Statement::If(stmt.cement_vars(ranges)),
            Self::Block(stmt) => Statement::Block(stmt.cement_vars(ranges)),
        }
    }
}

impl<'src> Analyze<'src> for ast::DeclVar<'src> {
    type Ouptut = Option<ExprStmt<Operation<'src>, Variable<'src>>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let ident = self.ident.lex;
        let expr = self
            .assignment
            .as_ref()
            .map(|x| x.expr.analyze(source, env))
            .transpose()?;
        match self.var_kw.val {
            // Constant
            VarKeyword::Pvt => env
                .decl_pvt(
                    ident,
                    expr.map(|_| todo!("constant folding"))
                        .ok_or_else(|| vec![todo!("error: missing assignment for constant")])?,
                )
                .map_err(|e| vec![todo!("error: {e}")])
                .map(|()| None),

            // Variable
            VarKeyword::Uni | VarKeyword::Let => env
                .decl_var(
                    ident,
                    expr.as_ref()
                        .and_then(|x| x.get_type(env))
                        .map(|t| t.to_abstract()),
                    true,
                )
                .map_err(|e| vec![todo!("error: {e}")])
                .map(|(unique_id, _ty, is_global)| {
                    expr.map(|rhs| ExprStmt {
                        expr: Expr::Assignment(Box::new(Assignment {
                            lvalue: Expr::Variable(Variable {
                                unique_id,
                                ident,
                                is_global,
                                #[cfg(debug_assertions)]
                                _ty: _ty.clone(),
                            }),
                            rhs,
                        })),
                    })
                }),
        }
    }
}

impl<'src> Analyze<'src> for ast::Declaration<'src> {
    type Ouptut = Option<Statement<Operation<'src>, Variable<'src>>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Var(decl) => decl.analyze(source, env).map(|x| x.map(Statement::Expr)),
            Self::Stmt(stmt) => stmt.analyze(source, env).map(Some),
        }
    }
}

/// A post-analysis program ready for execution
#[derive(Debug, Clone, Default)]
pub struct Program<'src> {
    stmts: Vec<Statement<RunOperation<'src>, RunVariable<'src>>>,
}

pub fn analyze<'src>(source: &'src str, prgm: &ast::Program<'src>) -> Result<'src, Program<'src>> {
    let mut env = StaticEnvironment::new();
    for (op, def) in builtin_ops::builtin_ops() {
        env.decl_op(op, def);
    }
    env.push_layer(); // HACK: remove this when functions are implemented
    prgm.decls
        .iter()
        .filter_map(|decl| decl.analyze(source, &mut env).transpose())
        .collect::<Result<'src, Vec<_>>>()
        .and_then(|stmts| {
            let ranges = env.cement_ranges().map_err(|e| {
                for item in e {
                    println!("error: size left indeterminant: {item:?}"); // TODO
                }
                Vec::new()
            })?;
            Ok(Program {
                stmts: stmts
                    .into_iter()
                    .map(|stmt| stmt.cement_vars(&ranges))
                    .collect(),
            })
        }) // TODO: need to convert analysis `Variable`s to `RunVariable`s
}
