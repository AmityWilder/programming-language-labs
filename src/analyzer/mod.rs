use crate::{
    analyzer::{
        eval::{Value, ValueMut},
        static_env::{OpOverload, OperandTypes, StaticEnvironment, StaticVar, VarType},
        types::{AbstractType, FunType, PrimitiveType, StaticType, TypeCompatible},
    },
    error::{ContextError, ErrorType, OpSide},
    exec::env::Environment,
    grammar::ast::{self, AstEnum, AstNode, AstNodeF},
    scanner::token::{
        ExpansionData,
        value::{Keyword, LexValue, Operator},
    },
    types::*,
};
use std::range::Range;

pub mod builtin_ops;
pub mod eval;
pub mod static_env;
pub mod types;

pub type Error<'src> = Vec<ContextError<'src>>;
pub type Result<'src, T> = std::result::Result<T, Error<'src>>;

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
pub enum FnDef<'src> {
    LangOpB(
        for<'val, 'srcf> fn(
            &'val mut Environment,
            Value<'val, 'srcf>,
            Value<'val, 'srcf>,
            ValueMut<'val, 'srcf>,
        ),
    ),
    LangOpU(for<'val, 'srcf> fn(&'val mut Environment, Value<'val, 'srcf>, ValueMut<'val, 'srcf>)),
    Builtin(for<'val> fn(&'val mut Environment) -> ()),
    UserDef(BlockStmt<'src>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnRef<'src> {
    pub ident: &'src str,
}

impl<'src> Expression<'src> for FnRef<'src> {
    type Output<'ty>
        = &'ty FunType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        env.get(self.ident).map(|var| match var {
            StaticVar::Fun(t) => t,
            _ => unimplemented!("not the name of a function"), // TODO: can this be an error?
        })
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub enum Operands<'src> {
    Binary(Expr<'src>, Expr<'src>),
    UnaryPre(Expr<'src>),
    UnaryPost(Expr<'src>),
}

impl<'src> AstNode<'src> for Operands<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        match self {
            Operands::Binary(x, _) | Operands::UnaryPre(x) | Operands::UnaryPost(x) => {
                x.get_range_start(source)
            }
        }
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        match self {
            Operands::Binary(_, x) | Operands::UnaryPre(x) | Operands::UnaryPost(x) => {
                x.get_range_end(source)
            }
        }
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self {
            Operands::Binary(l, r) => (l, r).get_range(source),
            Operands::UnaryPre(x) | Operands::UnaryPost(x) => x.get_range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<crate::scanner::token::ExpansionData<'src>> {
        match self {
            Operands::Binary(l, r) => (l, r).expansion(source),
            Operands::UnaryPre(x) | Operands::UnaryPost(x) => x.expansion(source),
        }
    }
}

impl<'src> Operands<'src> {
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

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Operation<'src> {
    pub def: FunType<'src>,
    pub operands: Operands<'src>,
}

impl<'src> AstNode<'src> for Operation<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.def.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.operands.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.def, &self.operands).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<crate::scanner::token::ExpansionData<'src>> {
        (&self.def, &self.operands).expansion(source)
    }
}

impl<'src> Expression<'src> for Operation<'src> {
    type Output<'ty>
        = &'ty StaticType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(&self.def.shape.ret)
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

/// Passthrough references
impl<'src> AstNode<'src> for Literal {
    #[inline]
    fn get_range_start(&self, _: &'src str) -> Option<usize> {
        None
    }

    #[inline]
    fn get_range_end(&self, _: &'src str) -> Option<usize> {
        None
    }

    #[inline]
    fn get_range(&self, _: &'src str) -> Option<Range<usize>> {
        None
    }

    #[inline]
    fn expansion(&self, _: &'src str) -> Option<ExpansionData<'src>> {
        None
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct Variable<'src> {
    pub id: &'src str,
    #[cfg(debug_assertions)]
    pub _ty: Option<AbstractType<'src>>,
}

/// Passthrough references
impl<'src> AstNode<'src> for Variable<'src> {
    #[inline]
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.id.get_range_start(source)
    }

    #[inline]
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.id.get_range_end(source)
    }

    #[inline]
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.id.get_range(source)
    }

    #[inline]
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.id.expansion(source)
    }
}

impl<'src> Expression<'src> for Variable<'src> {
    type Output<'ty>
        = VarType<'src, 'ty>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        env.get(self.id).and_then(|v| v.get_type(env))
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

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum Expr<'src> {
    Op(Box<Operation<'src>>),
    Literal(Literal),
    Variable(Variable<'src>),
}

impl<'src> Expression<'src> for Expr<'src> {
    type Output<'ty>
        = ExprType<'src, 'ty>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            Self::Op(x) => x.get_type(env).map(ExprType::Static),
            Self::Literal(x) => x.get_type(env).map(ExprType::Literal),
            Self::Variable(x) => x.get_type(env).map(ExprType::Variable),
        }
    }
}

impl<'src> Analyze<'src> for ast::Expr<'src> {
    type Ouptut = Expr<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Binary(x) => x.analyze(source, env).map(Box::new).map(Expr::Op),
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
    type Ouptut = Expr<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let ident = self.ident.lex;
        env.get(ident)
            // TODO: shouldn't Variable have actual meaning?
            .map(|v| {
                Expr::Variable(Variable {
                    id: ident,
                    #[cfg(debug_assertions)]
                    _ty: match v {
                        StaticVar::Fun(f) => Some(f.to_abstract()),
                        StaticVar::Var(t) => t.as_ref().map(TypeCompatible::to_abstract),
                        StaticVar::Pvt(v) => v.get_type(env).map(TypeCompatible::to_abstract),
                    },
                })
            }) // remember: could be a constant
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
            env.get_operator(&self.op.val, &operand_types)
                .map(|(shape, _)| Operation {
                    def: shape.def.clone(),
                    operands,
                })
                .ok_or_else(|| vec![todo!("error: no such operator defined")])
        })
    }
}

impl<'src> Analyze<'src> for ast::Binary<'src> {
    type Ouptut = Operation<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        self.lhs
            .analyze(source, env)
            .and_then(|lhs| self.rhs.analyze(source, env).map(|rhs| (lhs, rhs)))
            .and_then(|(lhs, rhs)| {
                // TODO: turn this into a function of its own
                // TBD: this can produce two errors. we should return two errors.
                let Some(l_ty) = lhs
                    .get_type(env)
                    .map(|t| t.to_abstract()) else
                {
                    return Err(vec![ContextError::node_error(source, &self.lhs, ErrorType::TypeAmbiguity(Box::new(lhs)))]); // TODO: need ranges
                };
                let Some(r_ty) = rhs
                    .get_type(env)
                    .map(|t| t.to_abstract()) else
                {
                    return Err(vec![ContextError::node_error(source, &self.rhs, ErrorType::TypeAmbiguity(Box::new(rhs)))]); // TODO: need ranges
                };
                let operands = Operands::Binary(lhs, rhs);
                let operand_types = OperandTypes::Binary(l_ty, r_ty);
                let op = &self.op.val;
                env.get_operator(op, &operand_types)
                    .ok_or_else(|| vec![todo!("error: no such operator defined\n operator: {op:?}\n operand types: {operand_types:?}\n operands: {operands:?}")])
                    .map(|(shape, _)| Operation {
                        def: shape.def.clone(),
                        operands,
                    })
            })
    }
}

/// An [`Expr`] that returns [`none`]
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt<'src> {
    expr: Expr<'src>,
}

impl<'src> AstNode<'src> for ExprStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.expr.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.expr.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.expr.get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<crate::scanner::token::ExpansionData<'src>> {
        self.expr.expansion(source)
    }
}

impl<'src> Analyze<'src> for ast::ExprStmt<'src> {
    type Ouptut = ExprStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        self.expr.analyze(source, env).map(|expr| {
            ExprStmt { expr }

            // if let Some(ty) = expr.get_type(env) {
            //     if *ty == AbstractType::Concrete(StaticType::None) {
            //         Ok(ExprStmt { expr })
            //     } else {
            //         Err(todo!("error: expected none, found {ty:?}"))
            //     }
            // } else if let Expr::Op(x) = &mut expr
            //     && let Operation {
            //         def: Operator::Assign,
            //         operands: Operands::Binary(Expr::Variable(lhs), rhs),
            //     } = &mut **x
            // {
            //     if let Some(rhs_ty) = rhs.get_type(env).cloned() {
            //         if let Some(var) = env.get_mut(lhs.id) {
            //             match var {
            //                 Var::Run(t) => {
            //                     *t = Some(rhs_ty);
            //                     lhs._ty = t.clone();
            //                     println!("late initialization of {} as {t:?}", lhs.id);
            //                     Ok(ExprStmt { expr })
            //                 }
            //                 Var::Pvt(_) => Err(todo!(
            //                     "error: cannot assign to constant after initialization"
            //                 )),
            //             }
            //         } else {
            //             Err(todo!("error: no variable named {}", lhs.id))
            //         }
            //     } else {
            //         Err(todo!("error: expression has no type\n expression: {rhs:?}"))
            //     }
            // } else {
            //     Err(todo!(
            //         "error: expression has no type\n expression: {expr:?}"
            //     ))
            // }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt<'src> {
    expr: Expr<'src>,
}

/// Passthrough references
impl<'src> AstNode<'src> for PrintStmt<'src> {
    #[inline]
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.expr.get_range_start(source)
    }

    #[inline]
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.expr.get_range_end(source)
    }

    #[inline]
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.expr.get_range(source)
    }

    #[inline]
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.expr.expansion(source)
    }
}

impl<'src> Analyze<'src> for ast::PrintStmt<'src> {
    type Ouptut = PrintStmt<'src>;

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

#[derive(Debug, Clone, PartialEq, AstEnum)]
pub enum OrStmt<'src> {
    More(IfStmt<'src>),
    Last(BlockStmt<'src>),
}

impl<'src> Expression<'src> for OrStmt<'src> {
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
    type Ouptut = OrStmt<'src>;

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
pub struct IfStmt<'src> {
    /// Must output [`bool`]
    pub pred: Expr<'src>,
    pub then: BlockStmt<'src>,
    pub or: Option<Box<OrStmt<'src>>>,
}

/// Passthrough references
impl<'src> AstNode<'src> for IfStmt<'src> {
    #[inline]
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.pred.get_range_start(source)
    }

    #[inline]
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (&self.then, self.or.as_deref()).get_range_end(source)
    }

    #[inline]
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.pred, (&self.then, self.or.as_deref())).get_range(source)
    }

    #[inline]
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.pred, (&self.then, self.or.as_deref())).expansion(source)
    }
}

impl<'src> Expression<'src> for IfStmt<'src> {
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
    type Ouptut = IfStmt<'src>;

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
pub struct BlockStmt<'src> {
    pub stmts: Vec<Statement<'src>>,
}

/// Passthrough references
impl<'src> AstNode<'src> for BlockStmt<'src> {
    #[inline]
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.stmts.as_slice().get_range_start(source)
    }

    #[inline]
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.stmts.as_slice().get_range_end(source)
    }

    #[inline]
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.stmts.as_slice().get_range(source)
    }

    #[inline]
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.stmts.as_slice().expansion(source)
    }
}

impl<'src> Expression<'src> for BlockStmt<'src> {
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
    type Ouptut = BlockStmt<'src>;

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
            .inspect(|_| env.pop_scope())
    }
}

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum Statement<'src> {
    Expr(ExprStmt<'src>),
    Print(PrintStmt<'src>),
    If(IfStmt<'src>),
    Block(BlockStmt<'src>),
}

impl<'src> Expression<'src> for Statement<'src> {
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
    type Ouptut = Statement<'src>;

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
    type Ouptut = Option<ExprStmt<'src>>;

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
            LexValue::Keyword(Keyword::Pvt) => {
                env.decl_pvt(
                    self.ident.lex,
                    expr.map(|_| todo!())
                        .ok_or_else(|| vec![todo!("error: missing assignment for constant")])?,
                );
                Ok(None)
            }

            // Global
            LexValue::Keyword(Keyword::Uni) => {
                env.decl_global_var(
                    self.ident.lex,
                    expr.as_ref()
                        .and_then(|x| x.get_type(env))
                        .map(|t| t.to_abstract()),
                );
                Ok(expr.map(|expr| ExprStmt { expr }))
            }

            // Local
            LexValue::Keyword(Keyword::Let) => env
                .decl_local_var(
                    self.ident.lex,
                    expr.as_ref()
                        .and_then(|x| x.get_type(env))
                        .map(|t| t.to_abstract()),
                )
                .map(|_| expr.map(|expr| ExprStmt { expr }))
                .ok_or_else(|| todo!("error: can't use `let` at global scope")),

            _ => unimplemented!("should be caught by grammar"),
        }
    }
}

impl<'src> Analyze<'src> for ast::Declaration<'src> {
    type Ouptut = Option<Statement<'src>>;

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
    stmts: Vec<Statement<'src>>,
}

impl<'src> AstNode<'src> for Program<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.stmts.first().get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.stmts.last().get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.stmts.as_slice().get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<crate::scanner::token::ExpansionData<'src>> {
        self.stmts.as_slice().expansion(source)
    }
}

pub fn analyze<'src>(source: &'src str, prgm: &ast::Program<'src>) -> Result<'src, Program<'src>> {
    let mut env = StaticEnvironment::new();
    for (overload, def) in builtin_ops::builtin_ops() {
        env.decl_operator(overload, def);
    }
    prgm.decls
        .iter()
        .filter_map(|decl| decl.analyze(source, &mut env).transpose())
        .collect::<Result<'src, Vec<_>>>()
        .map(|stmts| Program { stmts })
}
