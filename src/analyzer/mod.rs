use crate::{
    analyzer::{
        eval::{Value, ValueMut},
        static_env::{EnvItem, OpOverload, OperandTypes, StaticEnvironment, VarType},
        types::{AbstractType, FunShape, FunType, PrimitiveType, StaticType, TypeCompatible},
    },
    error::{ContextError, ErrorType, OpSide},
    exec::env::Environment,
    grammar::ast::{self, VarKeyword},
    scanner::token::value::{Keyword, LexValue, Operator},
    types::*,
};
use std::{marker::PhantomData, range::Range};

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

#[derive(Debug)]
pub enum FnDef<'src, Var> {
    LangOpB(
        for<'val, 'srcf> fn(
            &'val mut Environment,
            Value<'val, 'srcf>,
            Value<'val, 'srcf>,
            ValueMut<'val, 'srcf>,
        ),
    ),
    LangOpU(for<'val, 'srcf> fn(&'val mut Environment, Value<'val, 'srcf>, ValueMut<'val, 'srcf>)),
    UserDef(BlockStmt<'src, Var>),
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
        env.lookup(self.ident).find_map(|var| match var {
            EnvItem::Fun((t, _)) => Some(t), // TBD: we have access to the name here, but should we use it?
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operands<'src, Var> {
    Binary(Expr<'src, Var>, Expr<'src, Var>),
    UnaryPre(Expr<'src, Var>),
    UnaryPost(Expr<'src, Var>),
}

impl<'src> Operands<'src, Variable<'src>> {
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

#[derive(Debug, Clone, PartialEq)]
pub struct Operation<'src, Var> {
    pub def: FunShape<'src>,
    pub operands: Operands<'src, Var>,
}

impl<'src> Expression<'src> for Operation<'src, Variable<'src>> {
    type Output<'ty>
        = &'ty StaticType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(&self.def.ret)
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

/// Runtime variable
#[derive(Debug, Clone, PartialEq)]
pub struct RunVariable<'src> {
    range: Range<usize>,
    #[cfg(debug_assertions)]
    _ty: StaticType<'src>,
    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

/// Analysis variable
///
/// Has no actual memory location, since its type may not have been determined yet
#[derive(Debug, Clone, PartialEq)]
pub struct Variable<'src> {
    pub id: &'src str,
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
        env.lookup(self.id).next().and_then(|v| v.get_type(env))
    }
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
pub struct Assignment<'src, Var> {
    /// Must evaluate to mutable
    pub lvalue: Expr<'src, Var>,
    pub rhs: Expr<'src, Var>,
}

impl<'src> Expression<'src> for Assignment<'src, Variable<'src>> {
    type Output<'ty>
        = PrimitiveType
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(PrimitiveType::None)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src, Var> {
    Assignment(Box<Assignment<'src, Var>>),
    Op(Box<Operation<'src, Var>>),
    Literal(Literal),
    Variable(Var),
}

impl<'src> Expression<'src> for Expr<'src, Variable<'src>> {
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
    type Ouptut = Expr<'src, Variable<'src>>;

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
    type Ouptut = Expr<'src, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let ident = self.ident.lex;
        env.lookup(ident)
            .find_map(|v| match *v {
                EnvItem::Var(ref x, is_global) => Some(Expr::Variable(Variable {
                    id: ident,
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
    type Ouptut = Operation<'src, Variable<'src>>;

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
                .map(|item| match item {
                    EnvItem::Op(overload) => overload,
                    _ => unreachable!("only operators should be stored with operator keys"),
                })
                .find(|overload| overload.operands == operand_types)
                .map(|overload| Operation {
                    def: overload.fun.clone(),
                    operands,
                })
                .ok_or_else(|| vec![todo!("error: no such operator defined")])
        })
    }
}

impl<'src> Analyze<'src> for ast::Binary<'src> {
    type Ouptut = Expr<'src, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let lhs = self.lhs.analyze(source, env)?;
        let rhs = self.rhs.analyze(source, env)?;
        let l_ty = lhs.get_type(env).map(|t| t.to_abstract());
        let r_ty = rhs.get_type(env).map(|t| t.to_abstract());
        if self.op.val == Operator::Assign {
            // TODO: cases where lhs is assignable in general, not specifically a variable
            if let Expr::Variable(var) = &lhs {
                match (l_ty, r_ty) {
                    (Some(l_ty), Some(r_ty)) => {
                        // require rhs to be the same type as lhs
                        r_ty.expect_type(&l_ty)
                            .map_err(|e| vec![ContextError::node_error(source, &self.rhs, e)])?;
                    }
                    (None, Some(r_ty)) => {
                        // update lhs type to match rhs
                        let Some(EnvItem::Var(var, _is_global)) = env.lookup_mut(var.id).next()
                        else {
                            unreachable!(
                                "Expr::Variable should only result from a StaticVar::Var present in `env`"
                            );
                        };
                        *var = Some(r_ty);
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
                        .map(|item| match item {
                            EnvItem::Op(overload) => overload,
                            _ => unreachable!("only operators should be stored with operator keys"),
                        })
                        .find(|overload| overload.operands == operand_types)
                        .ok_or_else(|| {
                            vec![todo!(
                                "error: no such operator defined\n operator: {op:?}\n \
                            operand types: {operand_types:?}\n \
                            operands: {operands:?}\n \
                            supported operands: {:#?}",
                                env.lookup(op)
                                    .map(|item| match item {
                                        EnvItem::Op(overload) => overload,
                                        _ => unreachable!(
                                            "only operators should be stored with operator keys"
                                        ),
                                    })
                                    .map(|t| t.fun.to_string())
                                    .collect::<Vec<_>>()
                            )]
                        })
                        .map(|overload| {
                            Expr::Op(Box::new(Operation {
                                def: overload.fun.clone(),
                                operands,
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
pub struct ExprStmt<'src, Var> {
    expr: Expr<'src, Var>,
}

impl<'src> Analyze<'src> for ast::ExprStmt<'src> {
    type Ouptut = ExprStmt<'src, Variable<'src>>;

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

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt<'src, Var> {
    expr: Expr<'src, Var>,
}

impl<'src> Analyze<'src> for ast::PrintStmt<'src> {
    type Ouptut = PrintStmt<'src, Variable<'src>>;

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
                .map_err(|e| vec![todo!()])
                .map(|()| PrintStmt { expr })
            } else {
                Err(todo!(
                    "error: expression has no type\n expression: {expr:?}"
                ))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrStmt<'src, Var> {
    More(IfStmt<'src, Var>),
    Last(BlockStmt<'src, Var>),
}

impl<'src> Expression<'src> for OrStmt<'src, Variable<'src>> {
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

impl<'src> Analyze<'src> for ast::OrBody<'src> {
    type Ouptut = OrStmt<'src, Variable<'src>>;

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
pub struct IfStmt<'src, Var> {
    /// Must output [`bool`]
    pub pred: Expr<'src, Var>,
    pub then: BlockStmt<'src, Var>,
    pub or: Option<Box<OrStmt<'src, Var>>>,
}

impl<'src> Expression<'src> for IfStmt<'src, Variable<'src>> {
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
    type Ouptut = IfStmt<'src, Variable<'src>>;

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
            .map_err(|e| vec![todo!()])
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

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt<'src, Var> {
    pub stmts: Vec<Statement<'src, Var>>,
}

impl<'src> Expression<'src> for BlockStmt<'src, Variable<'src>> {
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
    type Ouptut = BlockStmt<'src, Variable<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        env.push_scope();
        self.body
            .iter()
            .filter_map(|decl| decl.analyze(source, env).transpose())
            .collect::<Result<'src, Vec<_>>>()
            .map(|stmts| BlockStmt { stmts })
            .inspect(|_| {
                env.pop_scope();
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'src, Var> {
    Expr(ExprStmt<'src, Var>),
    Print(PrintStmt<'src, Var>),
    If(IfStmt<'src, Var>),
    Block(BlockStmt<'src, Var>),
}

impl<'src> Expression<'src> for Statement<'src, Variable<'src>> {
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
    type Ouptut = Statement<'src, Variable<'src>>;

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

impl<'src> Analyze<'src> for ast::DeclVar<'src> {
    type Ouptut = Option<ExprStmt<'src, Variable<'src>>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let expr = self
            .assignment
            .as_ref()
            .map(|x| x.expr.analyze(source, env))
            .transpose()?;
        match self.var_kw.val {
            // Constant
            VarKeyword::Pvt => {
                env.decl_pvt(
                    self.ident.lex,
                    expr.map(|_| todo!())
                        .ok_or_else(|| vec![todo!("error: missing assignment for constant")])?,
                );
                Ok(None)
            }

            // Variable
            VarKeyword::Uni | VarKeyword::Let => {
                env.decl_var(
                    self.ident.lex,
                    expr.as_ref()
                        .and_then(|x| x.get_type(env))
                        .map(|t| t.to_abstract()),
                    true,
                );
                Ok(expr.map(|expr| ExprStmt { expr }))
            }
        }
    }
}

impl<'src> Analyze<'src> for ast::Declaration<'src> {
    type Ouptut = Option<Statement<'src, Variable<'src>>>;

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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program<'src> {
    stmts: Vec<Statement<'src, RunVariable<'src>>>,
}

pub fn analyze<'src>(source: &'src str, prgm: &ast::Program<'src>) -> Result<'src, Program<'src>> {
    let mut env = StaticEnvironment::new();
    for (op, def) in builtin_ops::builtin_ops() {
        env.decl_op(op, def);
    }
    prgm.decls
        .iter()
        .filter_map(|decl| decl.analyze(source, &mut env).transpose())
        .collect::<Result<'src, Vec<_>>>()
        .map(|stmts| Program { stmts: todo!() }) // TODO: need to convert analysis `Variable`s to `RunVariable`s
}
