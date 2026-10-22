use crate::{
    analyzer::{
        static_env::{EnvItem, OpOverload, OperandTypes, StaticEnvironment, StaticItem, VarType},
        types::{
            AbstractType, Fallible, FunShape, FunType, OrType, PrimitiveType, StaticType,
            TypeCompatible, TypeSize,
        },
    },
    error::{ContextError, ErrorType, ImmutableAssignment, OpSide},
    exec::{env::Environment, error::Failure},
    grammar::ast::{self, AstNodeFinal, VarKeyword},
    interp::{
        env::{InterpEnvironment, InterpVar},
        value::{InterpBinaryOpFn, InterpOp, InterpUnaryOpFn, InterpVal},
    },
    scanner::token::value::{Keyword, LexValue, Operator},
    types::*,
};
#[cfg(not(debug_assertions))]
use std::marker::PhantomData;
use std::range::Range;
use types::TypeBound;

pub mod builtin_ops;
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
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut>;
}

pub trait GetType<'src> {
    type Output<'ty>: 'ty + TypeCompatible<'src>
    where
        'src: 'ty,
        Self: 'ty;

    /// The type output by the expression
    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>>;
}

pub trait CementVars {
    /// The version of `Self` where [`AnalysisOp`]/[`AnalysisVar`] is replaced with [`RuntimeOp`]/[`RuntimeVar`]
    type Concrete;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete;
}

pub trait CoagulateVars<'src> {
    /// The version of `Self` where [`AnalysisOp`]/[`AnalysisVar`] is replaced with [`InterpOp`]/[`InterpVar`]
    type Interp;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp;
}

#[derive(Debug)]
pub enum FnDef<O, V> {
    LangOpB(BinaryOpFn, InterpBinaryOpFn),
    LangOpU(UnaryOpFn, InterpUnaryOpFn),
    UserDef(BlockStmt<O, V>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnRef<'src> {
    pub ident: &'src str,
}

impl<'src> GetType<'src> for FnRef<'src> {
    type Output<'ty>
        = &'ty FunShape<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        env.lookup(self.ident).find_map(|(_, x)| match &x.val {
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

impl<'src> Operands<AnalysisOp<'src>, AnalysisVar<'src>> {
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

impl<'src> CementVars for Operands<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = Operands<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Binary(l, r) => Operands::Binary(l.cement_vars(ranges), r.cement_vars(ranges)),
            Self::UnaryPre(x) => Operands::UnaryPre(x.cement_vars(ranges)),
            Self::UnaryPost(x) => Operands::UnaryPost(x.cement_vars(ranges)),
        }
    }
}

impl<'src> CoagulateVars<'src> for Operands<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = Operands<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        match self {
            Self::Binary(l, r) => Operands::Binary(
                l.coagulate_vars(s_env, i_env),
                r.coagulate_vars(s_env, i_env),
            ),
            Self::UnaryPre(x) => Operands::UnaryPre(x.coagulate_vars(s_env, i_env)),
            Self::UnaryPost(x) => Operands::UnaryPost(x.coagulate_vars(s_env, i_env)),
        }
    }
}

pub type BinaryOpFn = for<'val, 'src> fn(
    crate::exec::value::ValueMut<'val, 'src>,
    crate::exec::value::Value<'val, 'src>,
    crate::exec::value::Value<'val, 'src>,
) -> std::result::Result<(), Failure>;

pub type UnaryOpFn = for<'val, 'src> fn(
    crate::exec::value::ValueMut<'val, 'src>,
    crate::exec::value::Value<'val, 'src>,
) -> std::result::Result<(), Failure>;

#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisOp<'src> {
    Binary(
        (FunShape<'src>, BinaryOpFn, InterpBinaryOpFn),
        Expr<Self, AnalysisVar<'src>>,
        Expr<Self, AnalysisVar<'src>>,
    ),
    UnaryPre(
        (FunShape<'src>, UnaryOpFn, InterpUnaryOpFn),
        Expr<Self, AnalysisVar<'src>>,
    ),
    UnaryPost(
        (FunShape<'src>, UnaryOpFn, InterpUnaryOpFn),
        Expr<Self, AnalysisVar<'src>>,
    ),
}

#[derive(Debug, Clone)]
pub enum RuntimeOp<'src> {
    Binary(
        #[cfg(debug_assertions)] StaticType<'src>,
        #[cfg(not(debug_assertions))] (),
        BinaryOpFn,
        // output size
        usize,
        Expr<Self, RuntimeVar<'src>>,
        Expr<Self, RuntimeVar<'src>>,
    ),
    UnaryPre(
        #[cfg(debug_assertions)] StaticType<'src>,
        #[cfg(not(debug_assertions))] (),
        UnaryOpFn,
        // output size
        usize,
        Expr<Self, RuntimeVar<'src>>,
    ),
    UnaryPost(
        #[cfg(debug_assertions)] StaticType<'src>,
        #[cfg(not(debug_assertions))] (),
        UnaryOpFn,
        // output size
        usize,
        Expr<Self, RuntimeVar<'src>>,
    ),
}

impl<'src> GetType<'src> for AnalysisOp<'src> {
    type Output<'ty>
        = &'ty StaticType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(
            &match self {
                Self::Binary((f, _, _), _, _)
                | Self::UnaryPre((f, _, _), _)
                | Self::UnaryPost((f, _, _), _) => f,
            }
            .ret,
        )
    }
}

impl<'src> CementVars for AnalysisOp<'src> {
    type Concrete = RuntimeOp<'src>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Binary((f, def, _), l, r) => {
                let size = f
                    .ret
                    .size()
                    .unwrap_or_else(|| panic!("size should be known by this point\n type: {f}"))
                    .unwrap_or_else(|e| panic!("error determining type size: {e}\n type: {f}"));
                RuntimeOp::Binary(
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
            Self::UnaryPre((f, def, _), x) => {
                let size = f
                    .ret
                    .size()
                    .unwrap_or_else(|| panic!("size should be known by this point\n type: {f}"))
                    .unwrap_or_else(|e| panic!("error determining type size: {e}\n type: {f}"));
                RuntimeOp::UnaryPre(
                    #[cfg(debug_assertions)]
                    f.ret,
                    #[cfg(not(debug_assertions))]
                    (),
                    def,
                    size,
                    x.cement_vars(ranges),
                )
            }
            Self::UnaryPost((f, def, _), x) => {
                let size = f
                    .ret
                    .size()
                    .unwrap_or_else(|| panic!("size should be known by this point\n type: {f}"))
                    .unwrap_or_else(|e| panic!("error determining type size: {e}\n type: {f}"));
                RuntimeOp::UnaryPost(
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

impl<'src> CoagulateVars<'src> for AnalysisOp<'src> {
    type Interp = InterpOp<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        match self {
            Self::Binary((shape, _run_fun, interp_fun), lhs, rhs) => InterpOp::Binary(
                shape,
                interp_fun,
                lhs.coagulate_vars(s_env, i_env),
                rhs.coagulate_vars(s_env, i_env),
            ),
            Self::UnaryPre((shape, _run_fun, interp_fun), rhs) => {
                InterpOp::UnaryPre(shape, interp_fun, rhs.coagulate_vars(s_env, i_env))
            }
            Self::UnaryPost((shape, _run_fun, interp_fun), lhs) => {
                InterpOp::UnaryPost(shape, interp_fun, lhs.coagulate_vars(s_env, i_env))
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

impl<'src> GetType<'src> for Literal {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Directive {
    Inpt,
}

impl<'src> GetType<'src> for Directive {
    type Output<'ty>
        = AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(match self {
            Self::Inpt => cfg_select! {
                // needs fleshing out
                false => AbstractType::Opaque(Box::new(TypeBound::Any(vec![
                    TypeBound::Exactly(AbstractType::Concrete(StaticType::Primitive(
                        PrimitiveType::Text,
                    ))),
                    TypeBound::Exactly(AbstractType::Concrete(StaticType::Primitive(
                        PrimitiveType::Char,
                    ))),
                ]))),

                _ => AbstractType::Concrete(StaticType::Primitive(PrimitiveType::Text)),
            },
        })
    }
}

impl<'src> Analyze<'src> for ast::Directive<'src> {
    type Ouptut = Directive;

    fn analyze(
        &self,
        _source: &'src str,
        _env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>, // TODO: how can we use this to specialize?
    ) -> Result<'src, Self::Ouptut> {
        match self.kw.val {
            ast::DirectiveKw::Inpt => Ok(Directive::Inpt),
        }
    }
}

/// Operation<'src>, Variable<'src> variable
///
/// Has no actual memory location, since its type may not have been determined yet
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisVar<'src> {
    pub unique_id: usize,
    pub ident: &'src str,
    pub is_global: bool,
    #[cfg(debug_assertions)]
    _ty: Option<AbstractType<'src>>,
}

impl<'src> GetType<'src> for AnalysisVar<'src> {
    type Output<'ty>
        = VarType<'src, 'ty>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        env.lookup(self.ident)
            .next()
            .and_then(|(_, x)| x.val.get_type(env))
    }
}

impl<'src> CementVars for AnalysisVar<'src> {
    type Concrete = RuntimeVar<'src>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        RuntimeVar {
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

impl<'src> CoagulateVars<'src> for AnalysisVar<'src> {
    type Interp = InterpVar<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        let item = s_env
            .get_decl(self.unique_id)
            .expect("unique_id missing from ranges");
        match &item.val {
            EnvItem::Var(ty, _) => {
                match ty
                    .as_ref()
                    .expect("all types should be determined by this point")
                {
                    AbstractType::Concrete(ty) => {
                        i_env
                            .vars
                            .insert(self.unique_id, InterpVal::Uninit(ty.clone()));
                        InterpVar {
                            ident: self.ident,
                            id: self.unique_id,
                        }
                    }
                    t => panic!("all variables should be concrete by this point\n type: {t}"),
                }
            }
            _ => panic!("AnalysisVar should represent EnvItem::Var"),
        }
    }
}

/// Runtime variable
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeVar<'src> {
    pub range: Range<usize>,
    #[cfg(debug_assertions)]
    pub ty_: StaticType<'src>,
    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprType<'src, 'ty> {
    Static(&'ty StaticType<'src>),
    Literal(PrimitiveType),
    Directive(AbstractType<'src>),
    Variable(VarType<'src, 'ty>),
}

impl<'src> TypeCompatible<'src> for ExprType<'src, '_> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        match self {
            Self::Static(t) => t.is_compatible(other),
            Self::Literal(t) => t.is_compatible(other),
            Self::Directive(t) => t.is_compatible(other),
            Self::Variable(t) => t.is_compatible(other),
        }
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        match self {
            Self::Static(t) => t.to_abstract(),
            Self::Literal(t) => t.to_abstract(),
            Self::Directive(t) => t.to_abstract(),
            Self::Variable(t) => t.to_abstract(),
        }
    }
}
impl std::fmt::Display for ExprType<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(t) => t.fmt(f),
            Self::Literal(t) => t.fmt(f),
            Self::Directive(t) => t.fmt(f),
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

impl<'src> GetType<'src> for Assignment<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Output<'ty>
        = PrimitiveType
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(PrimitiveType::None)
    }
}

impl<'src> CementVars for Assignment<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = Assignment<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        Assignment {
            lvalue: self.lvalue.cement_vars(ranges),
            rhs: self.rhs.cement_vars(ranges),
        }
    }
}

impl<'src> CoagulateVars<'src> for Assignment<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = Assignment<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        Assignment {
            lvalue: self.lvalue.coagulate_vars(s_env, i_env),
            rhs: self.rhs.coagulate_vars(s_env, i_env),
        }
    }
}

impl<'src> ast::TypeExpr<'src> {
    fn analyze(
        &self,
        _source: &'src str,
        _env: &StaticEnvironment<'src>,
    ) -> Result<'src, AbstractType<'src>> {
        let ok_ty = if let Some(name) = self.name.val {
            StaticType::Primitive(match name {
                ast::TypenameKeyword::None => PrimitiveType::None,
                ast::TypenameKeyword::Nevr => PrimitiveType::Nevr,
                ast::TypenameKeyword::Fail => PrimitiveType::Fail,
                ast::TypenameKeyword::Bool => PrimitiveType::Bool,
                ast::TypenameKeyword::Uint => PrimitiveType::Uint,
                ast::TypenameKeyword::Sint => PrimitiveType::Sint,
                ast::TypenameKeyword::Flpt => PrimitiveType::Flpt,
                ast::TypenameKeyword::Char => PrimitiveType::Char,
                ast::TypenameKeyword::Text => PrimitiveType::Text,
            })
        } else {
            todo!("custom types")
        };

        Ok(AbstractType::Concrete(
            if let Some(or_ty) = self.or_ty.as_ref() {
                StaticType::OrTy(Box::new(Fallible {
                    then: ok_ty,
                    or: match or_ty.ty.val {
                        ast::OrTypeKeyword::None => OrType::None,
                        ast::OrTypeKeyword::Nevr => OrType::Nevr,
                        ast::OrTypeKeyword::Fail => OrType::Fail,
                    },
                }))
            } else {
                ok_ty
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<O, V> {
    Assignment(Box<Assignment<O, V>>),
    Op(Box<O>),
    Literal(Literal),
    Directive(Directive),
    Variable(V),
}

impl<'src> GetType<'src> for Expr<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Output<'ty>
        = ExprType<'src, 'ty>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            Self::Assignment(x) => x.get_type(env).map(ExprType::Literal),
            Self::Op(x) => x.get_type(env).map(ExprType::Static),
            Self::Literal(x) => x.get_type(env).map(ExprType::Literal),
            Self::Directive(x) => x.get_type(env).map(ExprType::Directive),
            Self::Variable(x) => x.get_type(env).map(ExprType::Variable),
        }
    }
}

impl<'src> Analyze<'src> for ast::Expr<'src> {
    type Ouptut = Expr<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Binary(x) => x.analyze(source, env, ret),
            Self::Unary(x) => x.analyze(source, env, ret).map(Box::new).map(Expr::Op),
            Self::Literal(x) => x.analyze(source, env, ret).map(Expr::Literal),
            Self::Directive(x) => x.analyze(source, env, ret).map(Expr::Directive),
            Self::Variable(x) => x.analyze(source, env, ret),
            Self::Grouping(x) => x.expr.analyze(source, env, ret),
            Self::Type(_) => unreachable!("type expressions should be handled by operator"),
        }
    }
}

impl<'src> CementVars for Expr<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = Expr<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Assignment(x) => Expr::Assignment(Box::new(x.cement_vars(ranges))),
            Self::Op(x) => Expr::Op(Box::new(x.cement_vars(ranges))),
            Self::Literal(x) => Expr::Literal(x),
            Self::Directive(x) => Expr::Directive(x),
            Self::Variable(x) => Expr::Variable(x.cement_vars(ranges)),
        }
    }
}

impl<'src> CoagulateVars<'src> for Expr<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = Expr<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        match self {
            Self::Assignment(x) => Expr::Assignment(Box::new(x.coagulate_vars(s_env, i_env))),
            Self::Op(x) => Expr::Op(Box::new(x.coagulate_vars(s_env, i_env))),
            Self::Literal(x) => Expr::Literal(x),
            Self::Directive(x) => Expr::Directive(x),
            Self::Variable(x) => Expr::Variable(x.coagulate_vars(s_env, i_env)),
        }
    }
}

impl<'src> Analyze<'src> for ast::Literal<'src> {
    type Ouptut = Literal;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        let val = match self.value.val {
            LexValue::UIntLiteral(x) => Literal::Uint(x),
            LexValue::SIntLiteral(x) => Literal::Sint(x),
            LexValue::FlptLiteral(x) => Literal::Flpt(x),
            LexValue::CharLiteral(x) => Literal::Char(x.ch),
            LexValue::TextLiteral(x) => Literal::Text(
                x.process()
                    .map_err(|e| vec![ContextError::token_error(source, Some(&self.value), e)])?
                    .to_string(),
            ),
            LexValue::BoolLiteral(x) => Literal::Bool(x),
            LexValue::Keyword(Keyword::None) => Literal::None,
            _ => unimplemented!("should be handled by grammar"),
        };
        if let Some(ret) = ret {
            val.get_type(env)
                .expect("literals do not support implicit typing")
                .expect_type(ret, source, self)
                .map_err(|e| vec![e])
                .map(|()| val)
        } else {
            Ok(val)
        }
    }
}

/// Variable access, not assignment
impl<'src> Analyze<'src> for ast::Variable<'src> {
    type Ouptut = Expr<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TODO
    ) -> Result<'src, Self::Ouptut> {
        let ident = self.ident.lex;
        let (unique_id, ty, is_global) = env
            .lookup_mut(ident)
            .find_map(|(unique_id, item)| match &mut item.val {
                EnvItem::Var(ty, is_global) => Some((unique_id, ty, *is_global)),
                _ => None,
            })
            .ok_or_else(|| {
                vec![ContextError::token_error(
                    source,
                    Some(&self.ident),
                    ErrorType::UnknownVar,
                )]
            })?;
        // infer variable type
        if let Some(ret) = ret {
            if let Some(ty) = ty
                && !matches!(ty, AbstractType::Opaque(_))
            {
                if let Err(e) = ty.expect_type(ret, source, self) {
                    return Err(vec![e]);
                }
            } else {
                // TODO: apply opaque restraints
                #[cfg(debug_assertions)]
                println!("<inferrence> {ident}: {ret}");
                *ty = Some(ret.clone());
            }
        }
        Ok(Expr::Variable(AnalysisVar {
            unique_id,
            ident,
            is_global,
            #[cfg(debug_assertions)]
            _ty: ty.as_ref().map(TypeCompatible::to_abstract),
        }))
    }
}

impl<'src> Analyze<'src> for ast::Unary<'src> {
    type Ouptut = AnalysisOp<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        self.operand.analyze(source, env, None).and_then(|operand| {
            let Some(ty) = operand.get_type(env).map(|t| t.to_abstract()) else {
                return Err(vec![ContextError::node_error(
                    source,
                    &self.operand,
                    ErrorType::TypeAmbiguity,
                )]);
            };
            let (operands, operand_types) = match self.side {
                OpSide::Postfix => (Operands::UnaryPost(operand), OperandTypes::UnaryPost(ty)),
                OpSide::Prefix => (Operands::UnaryPre(operand), OperandTypes::UnaryPre(ty)),
            };
            let overload = env
                .lookup(&self.op.val)
                .map(|(_, item)| match &item.val {
                    EnvItem::Op(overload) => overload,
                    _ => unreachable!("only operators should be stored with operator keys"),
                })
                .filter(|overload| ret.is_none_or(|ret| overload.output.is_compatible(ret)))
                .find(|overload| overload.operands == operand_types);

            if let Some(overload) = overload {
                let (def, interp_def) = match overload.def {
                    FnDef::LangOpU(def, interp_def) => (def, interp_def),
                    FnDef::LangOpB(_, _) | FnDef::UserDef(_) => unimplemented!(),
                };
                Ok(match operands {
                    Operands::Binary(_, _) => unreachable!(),
                    Operands::UnaryPre(r) => {
                        AnalysisOp::UnaryPre((overload.fun.clone(), def, interp_def), r)
                    }
                    Operands::UnaryPost(l) => {
                        AnalysisOp::UnaryPost((overload.fun.clone(), def, interp_def), l)
                    }
                })
            }
            // HACK: `Exists` should be generic
            else if let Operands::UnaryPre(operand) = &operands
                && matches!(self.op.val, Operator::Exists)
            {
                fn exists_def<'val, 'src>(
                    _out: crate::exec::value::ValueMut<'val, 'src>,
                    _lhs: crate::exec::value::Value<'val, 'src>,
                ) -> std::result::Result<(), Failure> {
                    todo!()
                }

                #[expect(
                    clippy::unnecessary_wraps,
                    reason = "needs to fit the function pointer shape"
                )]
                fn interp_exists_def<'src>(
                    lhs: &InterpVal<'src>,
                ) -> std::result::Result<InterpVal<'src>, Failure> {
                    Ok(InterpVal::Primitive(
                        crate::interp::value::PrimitiveVal::Bool(crate::interp::value::BoolObject(
                            lhs.ensure_ok().is_ok(),
                        )),
                    ))
                }

                let Some(ty) = operand.get_type(env) else {
                    return Err(vec![ContextError::node_error(
                        source,
                        &self.operand,
                        ErrorType::TypeAmbiguity,
                    )]);
                };

                let ty = match ty {
                    ExprType::Static(ty)
                    | ExprType::Variable(VarType::Abstract(AbstractType::Concrete(ty))) => {
                        ty.clone()
                    }
                    ExprType::Literal(ty) => StaticType::Primitive(ty),
                    _ => todo!("generic `exists`"),
                };
                Ok(AnalysisOp::UnaryPost(
                    (
                        FunShape {
                            params: vec![ty],
                            ret: StaticType::Primitive(PrimitiveType::Bool),
                        },
                        exists_def,
                        interp_exists_def,
                    ),
                    match operands {
                        Operands::UnaryPre(operand) => operand,
                        _ => unreachable!("guarded by branch condition"),
                    },
                ))
            } else {
                let r_ty = match operand_types {
                    OperandTypes::UnaryPost(ty) | OperandTypes::UnaryPre(ty) => ty,
                    OperandTypes::Binary(_, _) => unreachable!(),
                };
                Err(vec![ContextError::node_error(
                    source,
                    self,
                    ErrorType::Unsupported {
                        op: self.op.val.to_token_eq(),
                        r_ty,
                        r_range: self.operand.range(source),
                    },
                )])
            }
        })
    }
}

impl<'src> Analyze<'src> for ast::Binary<'src> {
    type Ouptut = Expr<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        if self.op.val == Operator::Assign {
            let mut lhs = self.lhs.analyze(source, env, None)?;
            let l_ty = lhs.get_type(env).map(|t| t.to_abstract());
            let rhs = self.rhs.analyze(source, env, l_ty.as_ref())?;
            let r_ty = rhs.get_type(env).map(|t| t.to_abstract());
            if let Some(ret) = ret
                && let Err(e) = ret.expect_type(
                    &AbstractType::Concrete(StaticType::Primitive(PrimitiveType::None)),
                    source,
                    self,
                )
            {
                return Err(vec![e]);
            }
            // TODO: cases where lhs is assignable in general, not specifically a variable
            if let Expr::Variable(var) = &mut lhs {
                match (l_ty, r_ty) {
                    (Some(l_ty), Some(r_ty)) => {
                        // require rhs to be the same type as lhs
                        r_ty.expect_type(&l_ty, source, &self.rhs)
                            .map_err(|e| vec![e])?;
                    }

                    (None, Some(r_ty)) => {
                        // update lhs type to match rhs
                        let Some((
                            id,
                            &mut StaticItem {
                                val: EnvItem::Var(ref mut var_ty, _),
                                ..
                            },
                        )) = env.lookup_mut(var.ident).next()
                        else {
                            unreachable!("Expr::Variable should only result from a StaticVar::Var");
                        };
                        let r_ty_size = r_ty.size();
                        *var_ty = Some(r_ty);
                        #[cfg(debug_assertions)]
                        {
                            var._ty = var_ty.clone();
                        }
                    }

                    (_, None) => {
                        // failed type deduction twice; type ambiguity error
                        return Err(vec![
                            ContextError::node_error(source, &self.lhs, ErrorType::TypeAmbiguity),
                            ContextError::node_error(source, &self.rhs, ErrorType::TypeAmbiguity),
                        ]);
                    }
                }

                Ok(Expr::Assignment(Box::new(Assignment { lvalue: lhs, rhs })))
            } else {
                Err(vec![ContextError::node_error(
                    source,
                    &self.lhs,
                    ErrorType::AssignImmutable(match &lhs {
                        Expr::Variable(_) => unreachable!(),
                        Expr::Op(_) => ImmutableAssignment::Expression,
                        Expr::Assignment(_) => ImmutableAssignment::Assignment,
                        Expr::Literal(_) => ImmutableAssignment::Literal,
                        Expr::Directive(directive) => match directive {
                            Directive::Inpt => ImmutableAssignment::Keyword(Keyword::Inpt),
                        },
                    }),
                )])
            }
        } else {
            let lhs = self.lhs.analyze(source, env, None)?;
            let rhs = self.rhs.analyze(source, env, None)?;
            let l_ty = lhs.get_type(env).map(|t| t.to_abstract());
            let r_ty = rhs.get_type(env).map(|t| t.to_abstract());
            match (l_ty, r_ty) {
                (Some(l_ty), Some(r_ty)) => {
                    let operands = Operands::Binary(lhs, rhs);
                    let operand_types = OperandTypes::Binary(l_ty, r_ty);
                    let op = &self.op.val;
                    env.lookup(op)
                        .map(|(_, item)| match &item.val {
                            EnvItem::Op(overload) => overload,
                            _ => unreachable!("only operators should be stored with operator keys"),
                        })
                        .filter(|overload| ret.is_none_or(|ret| overload.output.is_compatible(ret)))
                        .find(|overload| overload.operands == operand_types)
                        .ok_or_else(|| {
                            // TODO: give the actual error this much info
                            #[cfg(debug_assertions)]
                            println!(
                                "error: no such operator defined\n operator: {op:?}\n \
                                operand types: {operand_types}\n \
                                operands: {operands:?}\n \
                                supported operands: {:#?}",
                                env.lookup(op)
                                    .map(|(_, item)| match &item.val {
                                        EnvItem::Op(overload) => overload,
                                        _ => unreachable!(
                                            "only operators should be stored with operator keys"
                                        ),
                                    })
                                    .map(|t| t.fun.to_string())
                                    .collect::<Vec<_>>()
                            );
                            let OperandTypes::Binary(l_ty, r_ty) = operand_types else {
                                unreachable!();
                            };
                            vec![ContextError::node_error(
                                source,
                                &self.op,
                                ErrorType::Incompatible {
                                    op: op.to_token_eq(),
                                    l_range: self.lhs.range(source),
                                    r_range: self.rhs.range(source),
                                    l_ty,
                                    r_ty,
                                },
                            )]
                        })
                        .map(|overload| {
                            let (def, interp_def) = match &overload.def {
                                FnDef::LangOpB(def, interp_def) => (def, interp_def),
                                FnDef::LangOpU(_, _) | FnDef::UserDef(_) => {
                                    unimplemented!()
                                }
                            };
                            Expr::Op(Box::new(match operands {
                                Operands::Binary(l, r) => AnalysisOp::Binary(
                                    (overload.fun.clone(), *def, *interp_def),
                                    l,
                                    r,
                                ),
                                _ => unreachable!(),
                            }))
                        })
                }

                (l_ty, r_ty) => Err(std::iter::chain(
                    l_ty.is_none().then(|| {
                        ContextError::node_error(source, &self.lhs, ErrorType::TypeAmbiguity)
                    }),
                    r_ty.is_none().then(|| {
                        ContextError::node_error(source, &self.rhs, ErrorType::TypeAmbiguity)
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
    pub expr: Expr<O, V>,
}

impl<'src> Analyze<'src> for ast::ExprStmt<'src> {
    type Ouptut = ExprStmt<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        self.expr.analyze(source, env, ret).and_then(|expr| {
            if let Some(ty) = expr.get_type(env) {
                // TBD: can this be a "must_use" lint instead?
                const REQUIRE_NONE: bool = false;
                if REQUIRE_NONE {
                    ty.expect_type(
                        &AbstractType::Concrete(StaticType::Primitive(PrimitiveType::None)),
                        source,
                        &self.expr,
                    )
                    .map_err(|e| vec![e])?;
                }
                Ok(ExprStmt { expr })
            } else {
                Err(vec![ContextError::node_error(
                    source,
                    &self.expr,
                    ErrorType::TypeAmbiguity,
                )])
            }
        })
    }
}

impl<'src> CementVars for ExprStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = ExprStmt<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        ExprStmt {
            expr: self.expr.cement_vars(ranges),
        }
    }
}

impl<'src> CoagulateVars<'src> for ExprStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = ExprStmt<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        ExprStmt {
            expr: self.expr.coagulate_vars(s_env, i_env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt<O, V> {
    pub expr: Expr<O, V>,
}

impl<'src> Analyze<'src> for ast::PrintStmt<'src> {
    type Ouptut = PrintStmt<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        const EXPR_TY: &AbstractType =
            &AbstractType::Concrete(StaticType::Primitive(PrimitiveType::Text));
        self.group
            .expr
            .analyze(source, env, Some(EXPR_TY))
            .and_then(|expr| {
                if let Some(ty) = expr.get_type(env) {
                    ty.expect_type(EXPR_TY, source, &self.group.expr)
                        .map_err(|e| vec![e])
                        .map(|()| PrintStmt { expr })
                } else {
                    Err(vec![ContextError::node_error(
                        source,
                        &self.group.expr,
                        ErrorType::TypeAmbiguity,
                    )])
                }
            })
    }
}

impl<'src> CementVars for PrintStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = PrintStmt<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        PrintStmt {
            expr: self.expr.cement_vars(ranges),
        }
    }
}

impl<'src> CoagulateVars<'src> for PrintStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = PrintStmt<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        PrintStmt {
            expr: self.expr.coagulate_vars(s_env, i_env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrStmt<O, V> {
    More(IfStmt<O, V>),
    Last(BlockStmt<O, V>),
}

impl<'src> GetType<'src> for OrStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
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

impl<'src> CementVars for OrStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = OrStmt<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::More(stmt) => OrStmt::More(stmt.cement_vars(ranges)),
            Self::Last(stmt) => OrStmt::Last(stmt.cement_vars(ranges)),
        }
    }
}

impl<'src> CoagulateVars<'src> for OrStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = OrStmt<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        match self {
            Self::More(stmt) => OrStmt::More(stmt.coagulate_vars(s_env, i_env)),
            Self::Last(stmt) => OrStmt::Last(stmt.coagulate_vars(s_env, i_env)),
        }
    }
}

impl<'src> Analyze<'src> for ast::OrBody<'src> {
    type Ouptut = OrStmt<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TBD: does `or` return a value?
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::More(stmt) => stmt.analyze(source, env, None).map(OrStmt::More),
            Self::Last(stmt) => stmt.analyze(source, env, None).map(OrStmt::Last),
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

impl<'src> GetType<'src> for IfStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
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
    type Ouptut = IfStmt<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TBD: does `if` return a value?
    ) -> Result<'src, Self::Ouptut> {
        let pred = self.pred.analyze(source, env, None)?;
        if let Some(ty) = pred.get_type(env) {
            ty.expect_type(
                &AbstractType::Concrete(StaticType::Primitive(PrimitiveType::Bool)),
                source,
                &self.pred,
            )
            .map_err(|e| vec![e])
            .and_then(|()| {
                Ok(IfStmt {
                    pred,
                    then: self.then.analyze(source, env, None)?,
                    or: self
                        .or
                        .as_ref()
                        .map(|x| x.body.analyze(source, env, None))
                        .transpose()?
                        .map(Box::new),
                })
            })
        } else {
            Err(vec![ContextError::node_error(
                source,
                &self.pred,
                ErrorType::TypeAmbiguity,
            )])
        }
    }
}

impl<'src> CementVars for IfStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = IfStmt<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        IfStmt {
            pred: self.pred.cement_vars(ranges),
            then: self.then.cement_vars(ranges),
            or: self.or.map(|or| Box::new(or.cement_vars(ranges))),
        }
    }
}

impl<'src> CoagulateVars<'src> for IfStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = IfStmt<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        IfStmt {
            pred: self.pred.coagulate_vars(s_env, i_env),
            then: self.then.coagulate_vars(s_env, i_env),
            or: self.or.map(|or| Box::new(or.coagulate_vars(s_env, i_env))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt<O, V> {
    pub block: BlockStmt<O, V>,
}

impl<'src> GetType<'src> for LoopStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Concrete(StaticType::Primitive(
            PrimitiveType::Nevr,
        )))
    }
}

impl<'src> Analyze<'src> for ast::LoopStmt<'src> {
    type Ouptut = LoopStmt<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TBD: does `loop` return a value?
    ) -> Result<'src, Self::Ouptut> {
        self.block
            .analyze(source, env, None)
            .map(|block| LoopStmt { block })
    }
}

impl<'src> CementVars for LoopStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = LoopStmt<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        LoopStmt {
            block: self.block.cement_vars(ranges),
        }
    }
}

impl<'src> CoagulateVars<'src> for LoopStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = LoopStmt<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        LoopStmt {
            block: self.block.coagulate_vars(s_env, i_env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CordStmt<O, V> {
    pub block: BlockStmt<O, V>,
}

impl<'src> GetType<'src> for CordStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Concrete(StaticType::Primitive(
            PrimitiveType::Nevr,
        )))
    }
}

impl<'src> Analyze<'src> for ast::CordStmt<'src> {
    type Ouptut = CordStmt<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TBD: does `cord` return a value?
    ) -> Result<'src, Self::Ouptut> {
        self.block
            .analyze(source, env, None)
            .map(|block| CordStmt { block })
    }
}

impl<'src> CementVars for CordStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = CordStmt<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        CordStmt {
            block: self.block.cement_vars(ranges),
        }
    }
}

impl<'src> CoagulateVars<'src> for CordStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = CordStmt<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        CordStmt {
            block: self.block.coagulate_vars(s_env, i_env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HaltStmt {
    // TBD: should `break value;` be supported?
}

impl<'src> GetType<'src> for HaltStmt {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Concrete(StaticType::Primitive(
            PrimitiveType::Nevr,
        )))
    }
}

impl<'src> Analyze<'src> for ast::HaltStmt<'src> {
    type Ouptut = HaltStmt;

    fn analyze(
        &self,
        _source: &'src str,
        _env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        Ok(HaltStmt {})
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipStmt {}

impl<'src> GetType<'src> for SkipStmt {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Concrete(StaticType::Primitive(
            PrimitiveType::Nevr,
        )))
    }
}

impl<'src> Analyze<'src> for ast::SkipStmt<'src> {
    type Ouptut = SkipStmt;

    fn analyze(
        &self,
        _source: &'src str,
        _env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        Ok(SkipStmt {})
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt<O, V> {
    pub stmts: Vec<Statement<O, V>>,
}

impl<'src> GetType<'src> for BlockStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
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
    type Ouptut = BlockStmt<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TBD: does `block` return a value?
    ) -> Result<'src, Self::Ouptut> {
        env.push_scope().unwrap_or_else(|| {
            todo!("warning: cannot push a scope here, need to be in a function");
        });
        self.body
            .iter()
            .filter_map(|decl| decl.analyze(source, env, None).transpose())
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

impl<'src> CementVars for BlockStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = BlockStmt<RuntimeOp<'src>, RuntimeVar<'src>>;

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

impl<'src> CoagulateVars<'src> for BlockStmt<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = BlockStmt<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        BlockStmt {
            stmts: self
                .stmts
                .into_iter()
                .map(|stmt| stmt.coagulate_vars(s_env, i_env))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<O, V> {
    Expr(ExprStmt<O, V>),
    Print(PrintStmt<O, V>),
    If(IfStmt<O, V>),
    Loop(LoopStmt<O, V>),
    Cord(CordStmt<O, V>),
    Halt(HaltStmt),
    Skip(SkipStmt),
    Block(BlockStmt<O, V>),
}

impl<'src> GetType<'src> for Statement<AnalysisOp<'src>, AnalysisVar<'src>> {
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
            Self::Loop(stmt) => stmt.get_type(env),
            Self::Cord(stmt) => stmt.get_type(env),
            Self::Halt(stmt) => stmt.get_type(env),
            Self::Skip(stmt) => stmt.get_type(env),
            Self::Block(stmt) => stmt.get_type(env),
        }
    }
}

impl<'src> Analyze<'src> for ast::Statement<'src> {
    type Ouptut = Statement<AnalysisOp<'src>, AnalysisVar<'src>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        // NOTE: need to change here if the statements get return values!
        match self {
            Self::Expr(stmt) => stmt.analyze(source, env, None).map(Statement::Expr),
            Self::Print(stmt) => stmt.analyze(source, env, None).map(Statement::Print),
            Self::If(stmt) => stmt.analyze(source, env, None).map(Statement::If),
            Self::Loop(stmt) => stmt.analyze(source, env, None).map(Statement::Loop),
            Self::Cord(stmt) => stmt.analyze(source, env, None).map(Statement::Cord),
            Self::Halt(stmt) => stmt.analyze(source, env, None).map(Statement::Halt),
            Self::Skip(stmt) => stmt.analyze(source, env, None).map(Statement::Skip),
            Self::Block(stmt) => stmt.analyze(source, env, None).map(Statement::Block),
        }
    }
}

impl<'src> CementVars for Statement<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Concrete = Statement<RuntimeOp<'src>, RuntimeVar<'src>>;

    fn cement_vars(self, ranges: &[Range<usize>]) -> Self::Concrete {
        match self {
            Self::Expr(stmt) => Statement::Expr(stmt.cement_vars(ranges)),
            Self::Print(stmt) => Statement::Print(stmt.cement_vars(ranges)),
            Self::If(stmt) => Statement::If(stmt.cement_vars(ranges)),
            Self::Loop(stmt) => Statement::Loop(stmt.cement_vars(ranges)),
            Self::Cord(stmt) => Statement::Cord(stmt.cement_vars(ranges)),
            Self::Halt(stmt) => Statement::Halt(stmt),
            Self::Skip(stmt) => Statement::Skip(stmt),
            Self::Block(stmt) => Statement::Block(stmt.cement_vars(ranges)),
        }
    }
}

impl<'src> CoagulateVars<'src> for Statement<AnalysisOp<'src>, AnalysisVar<'src>> {
    type Interp = Statement<InterpOp<'src>, InterpVar<'src>>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        match self {
            Self::Expr(stmt) => Statement::Expr(stmt.coagulate_vars(s_env, i_env)),
            Self::Print(stmt) => Statement::Print(stmt.coagulate_vars(s_env, i_env)),
            Self::If(stmt) => Statement::If(stmt.coagulate_vars(s_env, i_env)),
            Self::Loop(stmt) => Statement::Loop(stmt.coagulate_vars(s_env, i_env)),
            Self::Cord(stmt) => Statement::Cord(stmt.coagulate_vars(s_env, i_env)),
            Self::Halt(stmt) => Statement::Halt(stmt),
            Self::Skip(stmt) => Statement::Skip(stmt),
            Self::Block(stmt) => Statement::Block(stmt.coagulate_vars(s_env, i_env)),
        }
    }
}

impl<'src> Analyze<'src> for ast::DeclVar<'src> {
    type Ouptut = Option<ExprStmt<AnalysisOp<'src>, AnalysisVar<'src>>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        if let Some(ret) = ret
            && let Err(e) = ret.expect_type(
                &AbstractType::Concrete(StaticType::Primitive(PrimitiveType::None)),
                source,
                self,
            )
        {
            return Err(vec![e]);
        }
        let ident = self.ident.lex;
        let annotated_type = self.ty.as_ref().map(|annotation| {
            annotation
                .ty
                .analyze(source, env)
                .expect("infallible at the time of writing")
        });
        let expr = self
            .assignment
            .as_ref()
            .map(|x| x.expr.analyze(source, env, annotated_type.as_ref()))
            .transpose()?;

        #[cfg(debug_assertions)]
        {
            print!("{}: ", self.ident.lex);
            if let Some(annotated_type) = &annotated_type {
                print!("{annotated_type}");
            } else {
                print!("<implicit>");
            }
            print!(" = ");
            if let Some(expr_ty) = expr.as_ref().and_then(|x| x.get_type(env)) {
                println!("{expr_ty}");
            } else {
                println!("<late>");
            }
        }

        if let Some(annotated_type) = annotated_type.as_ref()
            && let Some(expr_ty) = expr.as_ref().and_then(|x| x.get_type(env))
            && let Err(e) = expr_ty.expect_type(
                annotated_type,
                source,
                &self
                    .assignment
                    .as_ref()
                    .expect("guarded by if condition")
                    .expr,
            )
        {
            return Err(vec![e]);
        }

        match self.var_kw.val {
            // Constant
            VarKeyword::Pvt => env
                .decl_pvt(
                    ident,
                    expr.map(|_| todo!("constant folding"))
                        .ok_or_else(|| vec![todo!("error: missing assignment for constant")])?,
                    Some(Box::new(self.ident)),
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
                    Some(Box::new(self.ident)),
                )
                .map_err(|e| todo!("error: {e}"))
                .and_then(|(unique_id, ty, is_global)| {
                    if let Some(t) = annotated_type {
                        if let Some(ty) = ty.as_ref() {
                            ty.expect_type(&t, source, self)?;
                        } else {
                            #[cfg(debug_assertions)]
                            println!("<inferrence> {ident}: {t}");
                            *ty = Some(t);
                        }
                    }
                    Ok(expr.map(|rhs| ExprStmt {
                        expr: Expr::Assignment(Box::new(Assignment {
                            lvalue: Expr::Variable(AnalysisVar {
                                unique_id,
                                ident,
                                is_global,
                                #[cfg(debug_assertions)]
                                _ty: ty.clone(),
                            }),
                            rhs,
                        })),
                    }))
                })
                .map_err(|e| vec![e]),
        }
    }
}

impl<'src> Analyze<'src> for ast::Declaration<'src> {
    type Ouptut = Option<Statement<AnalysisOp<'src>, AnalysisVar<'src>>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Var(decl) => decl
                .analyze(source, env, ret)
                .map(|x| x.map(Statement::Expr)),
            Self::Stmt(stmt) => stmt.analyze(source, env, ret).map(Some),
        }
    }
}

/// A post-analysis program ready for execution
#[derive(Debug, Clone, Default)]
pub struct Program<O, V> {
    pub stmts: Vec<Statement<O, V>>,
}

pub type RuntimeProgram<'src> = Program<RuntimeOp<'src>, RuntimeVar<'src>>;
pub type InterpProgram<'src> = Program<InterpOp<'src>, InterpVar<'src>>;

fn analyze_base<'src, O, V, F>(
    source: &'src str,
    prgm: &ast::Program<'src>,
    f: F,
) -> Result<'src, Program<O, V>>
where
    F: FnOnce(
        StaticEnvironment<'src>,
        Vec<Statement<AnalysisOp<'src>, AnalysisVar<'src>>>,
    ) -> Result<'src, Program<O, V>>,
{
    let mut env = StaticEnvironment::new();
    for (op, def) in builtin_ops::builtin_ops() {
        env.decl_op(op, def, None);
    }
    env.push_layer(); // HACK: remove this when functions are implemented
    prgm.decls
        .iter()
        .filter_map(|decl| decl.analyze(source, &mut env, None).transpose())
        .collect::<Result<'src, Vec<_>>>()
        .and_then(|stmts| f(env, stmts))
}

pub fn analyze_interp<'src>(
    source: &'src str,
    prgm: &ast::Program<'src>,
) -> Result<'src, (InterpProgram<'src>, InterpEnvironment<'src>)> {
    let mut i_env = InterpEnvironment::new();
    analyze_base(source, prgm, |mut s_env, stmts| {
        s_env.finalize_vars().map_err(|e| {
            e.into_iter()
                .map(|id| {
                    let decl = s_env.get_decl(id).expect("error IDs should be valid");
                    ContextError::node_error(
                        source,
                        decl.node
                            .as_deref()
                            .expect("builtins should not be ambiguous"),
                        ErrorType::TypeAmbiguity,
                    )
                })
                .collect::<Vec<_>>()
        })?;
        Ok(InterpProgram {
            stmts: stmts
                .into_iter()
                .map(|stmt| stmt.coagulate_vars(&s_env, &mut i_env))
                .collect(),
        })
    })
    .map(|prgm| (prgm, i_env))
}

pub fn analyze<'src>(
    source: &'src str,
    prgm: &ast::Program<'src>,
) -> Result<'src, RuntimeProgram<'src>> {
    analyze_base(source, prgm, |env, stmts| {
        let ranges = env.cement_ranges().map_err(|e| {
            for item in e {
                println!("error: size left indeterminant: {item:?}"); // TODO
            }
            Vec::new()
        })?;
        Ok(RuntimeProgram {
            stmts: stmts
                .into_iter()
                .map(|stmt| stmt.cement_vars(&ranges))
                .collect(),
        })
    })
}
