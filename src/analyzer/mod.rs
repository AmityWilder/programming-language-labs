use crate::{
    analyzer::{
        static_env::{
            EnvItem, EnvKey, OpOverload, OperandTypes, StaticEnvironment, StaticItem, VarType,
        },
        types::{
            AbstractType, Fallible, FunShape, FunType, OrType, PrimitiveType, StaticType,
            TypeCompatible,
        },
    },
    error::{ContextError, ErrorType, Failure, ImmutableAssignment, OpSide},
    grammar::ast::{self, AstNode, AstNodeFinal, VarKeyword},
    interp::{
        env::{InterpEnvironment, InterpVar},
        value::{InterpBinaryOpFn, InterpLabel, InterpOp, InterpUnaryOpFn, InterpVal},
    },
    scanner::token::value::{Keyword, LexValue, Operator},
    types::{byte, flpt, sint, text, uint},
};
use std::cell::RefCell;
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
pub enum FnDef<L, O, V> {
    LangOpB(InterpBinaryOpFn),
    LangOpU(InterpUnaryOpFn),
    UserDef(BlockStmt<L, O, V>),
}

pub type AnalysisFnDef<'src> = FnDef<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpFnDef<'src> = FnDef<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

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
pub enum Operands<L, O, V> {
    Binary(Expr<L, O, V>, Expr<L, O, V>),
    UnaryPre(Expr<L, O, V>),
    UnaryPost(Expr<L, O, V>),
}

pub type AnalysisOperands<'src> =
    Operands<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpOperands<'src> = Operands<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> AnalysisOperands<'src> {
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

impl<'src> CoagulateVars<'src> for AnalysisOperands<'src> {
    type Interp = InterpOperands<'src>;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AnalysisLabel<'src> {
    pub name: &'src str,
}

impl<'src> Analyze<'src> for ast::Label<'src> {
    type Ouptut = AnalysisLabel<'src>;

    fn analyze(
        &self,
        _source: &'src str,
        _env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        // TODO: validation? of any kind?
        Ok(AnalysisLabel {
            name: self.name.lex,
        })
    }
}

impl<'src> Analyze<'src> for ast::ScopeLabel<'src> {
    type Ouptut = AnalysisLabel<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        self.label.analyze(source, env, ret)
    }
}

impl<'src> CoagulateVars<'src> for AnalysisLabel<'src> {
    type Interp = InterpLabel<'src>;

    fn coagulate_vars(
        self,
        _s_env: &StaticEnvironment<'src>,
        _i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        InterpLabel { name: self.name }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisOp<'src> {
    Binary(
        (FunShape<'src>, InterpBinaryOpFn),
        Expr<AnalysisLabel<'src>, Self, AnalysisVar<'src>>,
        Expr<AnalysisLabel<'src>, Self, AnalysisVar<'src>>,
    ),
    UnaryPre(
        (FunShape<'src>, InterpUnaryOpFn),
        Expr<AnalysisLabel<'src>, Self, AnalysisVar<'src>>,
    ),
    UnaryPost(
        (FunShape<'src>, InterpUnaryOpFn),
        Expr<AnalysisLabel<'src>, Self, AnalysisVar<'src>>,
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
                Self::Binary((f, _), _, _)
                | Self::UnaryPre((f, _), _)
                | Self::UnaryPost((f, _), _) => f,
            }
            .ret,
        )
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
            Self::Binary((shape, interp_fun), lhs, rhs) => InterpOp::Binary(
                shape,
                interp_fun,
                lhs.coagulate_vars(s_env, i_env),
                rhs.coagulate_vars(s_env, i_env),
            ),
            Self::UnaryPre((shape, interp_fun), rhs) => {
                InterpOp::UnaryPre(shape, interp_fun, rhs.coagulate_vars(s_env, i_env))
            }
            Self::UnaryPost((shape, interp_fun), lhs) => {
                InterpOp::UnaryPost(shape, interp_fun, lhs.coagulate_vars(s_env, i_env))
            }
        }
    }
}

// TODO: what about literals for custom types and arrays?
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    None,
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
    InptText,
    InptChar,
    InptByte,
}

impl<'src> GetType<'src> for Directive {
    type Output<'ty>
        = AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(match self {
            Self::InptText => AbstractType::Text,
            Self::InptChar => AbstractType::Char,
            Self::InptByte => AbstractType::Byte,
        })
    }
}

impl<'src> Analyze<'src> for ast::Directive<'src> {
    type Ouptut = Directive;

    fn analyze(
        &self,
        source: &'src str,
        _env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TODO: how can we use this to specialize?
    ) -> Result<'src, Self::Ouptut> {
        match self.kw.val {
            ast::DirectiveKw::Inpt => match ret {
                Some(&AbstractType::Text) | None => Ok(Directive::InptText),
                Some(&AbstractType::Char) => Ok(Directive::InptChar),
                Some(&AbstractType::Byte) => Ok(Directive::InptByte),

                Some(actual) => Err(vec![ContextError::node_error(
                    source,
                    &self.kw,
                    ErrorType::TypeMismatch(Box::new((
                        AbstractType::Opaque(Box::new(TypeBound::Any(vec![
                            TypeBound::Exactly(AbstractType::Text),
                            TypeBound::Exactly(AbstractType::Char),
                        ]))),
                        actual.clone(),
                    ))),
                )]),
            },
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
                            .insert(self.unique_id, RefCell::new(InterpVal::Uninit(ty.clone())));
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
pub struct Assignment<L, O, V> {
    /// Must evaluate to mutable
    pub lvalue: Expr<L, O, V>,
    pub rhs: Expr<L, O, V>,
}

pub type AnalysisAssignment<'src> =
    Assignment<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpAssignment<'src> = Assignment<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisAssignment<'src> {
    type Output<'ty>
        = PrimitiveType
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(PrimitiveType::None)
    }
}

impl<'src> CoagulateVars<'src> for AnalysisAssignment<'src> {
    type Interp = InterpAssignment<'src>;

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
    fn analyze(&self, _source: &'src str, _env: &StaticEnvironment<'src>) -> AbstractType<'src> {
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

        AbstractType::Concrete(if let Some(or_ty) = self.or_ty.as_ref() {
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
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<L, O, V> {
    Block(Box<BlockExpr<L, O, V>>),
    Assignment(Box<Assignment<L, O, V>>),
    Op(Box<O>),
    Literal(Literal),
    Directive(Directive),
    Variable(V),
}

pub type AnalysisExpr<'src> = Expr<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpExpr<'src> = Expr<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisExpr<'src> {
    type Output<'ty>
        = ExprType<'src, 'ty>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            Self::Block(x) => x.get_type(env).map(ExprType::Directive),
            Self::Assignment(x) => x.get_type(env).map(ExprType::Literal),
            Self::Op(x) => x.get_type(env).map(ExprType::Static),
            Self::Literal(x) => x.get_type(env).map(ExprType::Literal),
            Self::Directive(x) => x.get_type(env).map(ExprType::Directive),
            Self::Variable(x) => x.get_type(env).map(ExprType::Variable),
        }
    }
}

impl<'src> Analyze<'src> for ast::Expr<'src> {
    type Ouptut = AnalysisExpr<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Block(x) => x.analyze(source, env, ret).map(Box::new).map(Expr::Block),
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

impl<'src> CoagulateVars<'src> for AnalysisExpr<'src> {
    type Interp = InterpExpr<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        match self {
            Self::Block(x) => Expr::Block(Box::new(x.coagulate_vars(s_env, i_env))),
            Self::Assignment(x) => Expr::Assignment(Box::new(x.coagulate_vars(s_env, i_env))),
            Self::Op(x) => Expr::Op(Box::new(x.coagulate_vars(s_env, i_env))),
            Self::Literal(x) => Expr::Literal(x),
            Self::Directive(x) => Expr::Directive(x),
            Self::Variable(x) => Expr::Variable(x.coagulate_vars(s_env, i_env)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockExpr<L, O, V> {
    If(IfStmt<L, O, V>),
    Loop(LoopStmt<L, O, V>),
    Cord(CordStmt<L, O, V>),
    Block(BlockStmt<L, O, V>),
}

pub type AnalysisBlockExpr<'src> =
    BlockExpr<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpBlockExpr<'src> = BlockExpr<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisBlockExpr<'src> {
    type Output<'ty>
        = AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TODO: that's a lot of cloning
        match self {
            BlockExpr::If(stmt) => stmt.get_type(env).map(|x| x.to_abstract()),
            BlockExpr::Loop(stmt) => stmt.get_type(env).map(|x| x.to_abstract()),
            BlockExpr::Cord(stmt) => stmt.get_type(env).map(|x| x.to_abstract()),
            BlockExpr::Block(stmt) => stmt.get_type(env).map(|x| x.to_abstract()),
        }
    }
}

impl<'src> Analyze<'src> for ast::BlockExpr<'src> {
    type Ouptut = AnalysisBlockExpr<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            ast::BlockExpr::If(stmt) => stmt.analyze(source, env, ret).map(BlockExpr::If),
            ast::BlockExpr::Loop(stmt) => stmt.analyze(source, env, ret).and_then(|x| match x {
                Statement::Loop(expr) => Ok(BlockExpr::Loop(expr)),
                Statement::Cord(expr) => Ok(BlockExpr::Cord(expr)),

                Statement::For(_) | Statement::Rep(_) => Err(vec![ContextError::node_error(
                    source,
                    &stmt.kw,
                    ErrorType::FallibleBlockExpr,
                )]),

                Statement::Expr(_)
                | Statement::Print(_)
                | Statement::If(_)
                | Statement::Halt(_)
                | Statement::Skip(_)
                | Statement::Block(_)
                | Statement::Defer(_) => unreachable!("analysis came from a LoopBlock"),
            }),
            ast::BlockExpr::Block(stmt) => stmt.analyze(source, env, ret).map(BlockExpr::Block),
        }
    }
}

impl<'src> CoagulateVars<'src> for AnalysisBlockExpr<'src> {
    type Interp = InterpBlockExpr<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        match self {
            Self::If(x) => BlockExpr::If(x.coagulate_vars(s_env, i_env)),
            Self::Loop(x) => BlockExpr::Loop(x.coagulate_vars(s_env, i_env)),
            Self::Cord(x) => BlockExpr::Cord(x.coagulate_vars(s_env, i_env)),
            Self::Block(x) => BlockExpr::Block(x.coagulate_vars(s_env, i_env)),
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
            LexValue::ByteLiteral(x) => Literal::Byte(x),
            LexValue::UIntLiteral(x) => Literal::Uint(x),
            LexValue::SIntLiteral(x) => Literal::Sint(x),
            LexValue::FlptLiteral(x) => Literal::Flpt(x),
            LexValue::CharLiteral(x) => Literal::Char(x.ch),
            LexValue::TextLiteral(x) => Literal::Text(
                x.process()
                    .map_err(|esc| {
                        vec![ContextError::token_error(
                            source,
                            Some(&self.value),
                            ErrorType::InvalidEscape(esc),
                        )]
                    })?
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
    type Ouptut = AnalysisExpr<'src>;

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

fn find_unary_overload<'env, 'src>(
    op: Operator,
    side: OpSide,
    ty: &AbstractType<'src>,
    env: &'env StaticEnvironment<'src>,
    ret: Option<&AbstractType<'src>>,
) -> Option<&'env OpOverload<'src>> {
    let mut iter = env
        .lookup(&op)
        .map(|(_, item)| match &item.val {
            EnvItem::Op(overload) => overload,
            _ => unreachable!("only operators should be stored with operator keys"),
        })
        .filter(|overload| ret.is_none_or(|ret| overload.output.is_compatible(ret)));
    match side {
        OpSide::Postfix => iter.find(|overload| {
            matches!(&overload.operands,
                OperandTypes::UnaryPost(t) if t == ty
            )
        }),
        OpSide::Prefix => iter.find(|overload| {
            matches!(&overload.operands,
                OperandTypes::UnaryPre(t) if t == ty
            )
        }),
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
            let overload = find_unary_overload(self.op.val, self.side, &ty, env, ret);
            if let Some(overload) = overload {
                let interp_def = match overload.def {
                    FnDef::LangOpU(interp_def) => interp_def,
                    FnDef::LangOpB(_) | FnDef::UserDef(_) => unimplemented!(),
                };
                let fun = overload.fun.clone();
                Ok(match self.side {
                    OpSide::Prefix => AnalysisOp::UnaryPre((fun, interp_def), operand),
                    OpSide::Postfix => AnalysisOp::UnaryPost((fun, interp_def), operand),
                })
            }
            // HACK: `Exists` should be generic
            else if let OpSide::Prefix = self.side
                && matches!(self.op.val, Operator::Exists)
            {
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
                        builtin_ops::interp_exists_def,
                    ),
                    operand,
                ))
            } else {
                let e = ContextError::node_error(
                    source,
                    &self.op,
                    ErrorType::Unsupported {
                        op: self.op.val.to_token_eq(),
                        side: self.side,
                        ty,
                        range: self.operand.range(source),
                        candidates: env
                            .lookup(&self.op.val)
                            .filter_map(|(_, item)| match &item.val {
                                EnvItem::Op(overload) => match (&overload.operands, self.side) {
                                    (OperandTypes::UnaryPost(t), OpSide::Postfix)
                                    | (OperandTypes::UnaryPre(t), OpSide::Prefix) => {
                                        Some(((t.clone(),), overload.output.clone()))
                                    }
                                    _ => None,
                                },
                                _ => unreachable!(
                                    "only operators should be stored with operator keys"
                                ),
                            })
                            .collect(),
                    },
                );
                Err(vec![e])
            }
        })
    }
}

fn find_binary_overload<'env, 'src>(
    op: Operator,
    l_ty: &AbstractType<'src>,
    r_ty: &AbstractType<'src>,
    env: &'env StaticEnvironment<'src>,
    ret: Option<&AbstractType<'src>>,
) -> Option<&'env OpOverload<'src>> {
    let mut iter = env
        .lookup(&op)
        .map(|(_, item)| match &item.val {
            EnvItem::Op(overload) => overload,
            _ => unreachable!("only operators should be stored with operator keys"),
        })
        .filter(|overload| ret.is_none_or(|ret| overload.output.is_compatible(ret)));
    iter.find(|overload| {
        matches!(&overload.operands,
            OperandTypes::Binary(l, r) if l == l_ty && r == r_ty
        )
    })
}

impl<'src> ast::Binary<'src> {
    fn analyze_assignment(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, AnalysisExpr<'src>> {
        let mut lhs = self.lhs.analyze(source, env, None)?;
        let l_ty = lhs.get_type(env).map(|t| t.to_abstract());
        let rhs = self.rhs.analyze(source, env, l_ty.as_ref())?;
        let r_ty = rhs.get_type(env).map(|t| t.to_abstract());
        if let Some(ret) = ret
            && let Err(e) = ret.expect_type(&AbstractType::None, source, self)
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
                        _id,
                        &mut StaticItem {
                            key: EnvKey::Ident(ident),
                            val: EnvItem::Var(ref mut var_ty, _),
                            ..
                        },
                    )) = env.lookup_mut(var.ident).next()
                    else {
                        unreachable!("Expr::Variable should only result from a StaticVar::Var");
                    };
                    #[cfg(debug_assertions)]
                    println!("<inferrence> {ident}: {r_ty}");
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
                    Expr::Block(_) => todo!(),
                    Expr::Variable(_) => unreachable!(),
                    Expr::Op(_) => ImmutableAssignment::Expression,
                    Expr::Assignment(_) => ImmutableAssignment::Assignment,
                    Expr::Literal(_) => ImmutableAssignment::Literal,
                    Expr::Directive(directive) => match directive {
                        Directive::InptByte | Directive::InptChar | Directive::InptText => {
                            ImmutableAssignment::Keyword(Keyword::Inpt)
                        }
                    },
                }),
            )])
        }
    }

    fn analyze_conversion(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, AnalysisExpr<'src>> {
        let lhs = self.lhs.analyze(source, env, None)?;
        let r_ty = if let ast::Expr::Type(type_expr) = &self.rhs {
            type_expr.analyze(source, env)
        } else {
            unimplemented!("should be handled by grammar");
        };
        let l_ty = lhs.get_type(env).map(|t| t.to_abstract());
        if let Some(l_ty) = l_ty {
            print!("{l_ty}");
        } else {
            print!("<implicit>");
        }
        print!(" {} {r_ty}", self.op.lex);
        if let Some(ret) = ret {
            println!("{{hint: {ret}}}");
        } else {
            println!();
        }
        todo!("conversion");
    }

    fn analyze_op(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, AnalysisExpr<'src>> {
        let lhs = self.lhs.analyze(source, env, None)?;
        let rhs = self.rhs.analyze(source, env, None)?;
        let l_ty = lhs.get_type(env).map(|t| t.to_abstract());
        let r_ty = rhs.get_type(env).map(|t| t.to_abstract());
        match (l_ty, r_ty) {
            (Some(l_ty), Some(r_ty)) => find_binary_overload(self.op.val, &l_ty, &r_ty, env, ret)
                .ok_or_else(|| {
                    let e = ContextError::node_error(
                        source,
                        &self.op,
                        ErrorType::Incompatible {
                            op: self.op.val.to_token_eq(),
                            l_range: self.lhs.range(source),
                            r_range: self.rhs.range(source),
                            l_ty,
                            r_ty,
                            candidates: env
                                .lookup(&self.op.val)
                                .filter_map(|(_, item)| match &item.val {
                                    EnvItem::Op(overload) => match &overload.operands {
                                        OperandTypes::Binary(lhs, rhs) => Some((
                                            (lhs.clone(), rhs.clone()),
                                            overload.output.clone(),
                                        )),
                                        OperandTypes::UnaryPost(_) | OperandTypes::UnaryPre(_) => {
                                            None
                                        }
                                    },
                                    _ => unreachable!(
                                        "only operators should be stored with operator keys"
                                    ),
                                })
                                .collect(),
                        },
                    );
                    vec![e]
                })
                .map(|overload| {
                    let interp_def = match &overload.def {
                        FnDef::LangOpB(interp_def) => interp_def,
                        FnDef::LangOpU(_) | FnDef::UserDef(_) => {
                            unimplemented!()
                        }
                    };
                    Expr::Op(Box::new(AnalysisOp::Binary(
                        (overload.fun.clone(), *interp_def),
                        lhs,
                        rhs,
                    )))
                }),

            (l_ty, r_ty) => Err(std::iter::chain(
                l_ty.is_none()
                    .then(|| ContextError::node_error(source, &self.lhs, ErrorType::TypeAmbiguity)),
                r_ty.is_none()
                    .then(|| ContextError::node_error(source, &self.rhs, ErrorType::TypeAmbiguity)),
            )
            .collect::<Vec<ContextError<'src>>>()),
        }
    }
}

impl<'src> Analyze<'src> for ast::Binary<'src> {
    type Ouptut = AnalysisExpr<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        match self.op.val {
            Operator::Assign => self.analyze_assignment(source, env, ret),
            Operator::Convert | Operator::Transmute => self.analyze_conversion(source, env, ret),
            _ => self.analyze_op(source, env, ret),
        }
    }
}

/// An [`Expr`] that returns [`none`]
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt<L, O, V> {
    pub expr: Expr<L, O, V>,
}

pub type AnalysisExprStmt<'src> =
    ExprStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpExprStmt<'src> = ExprStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> Analyze<'src> for ast::ExprStmt<'src> {
    type Ouptut = AnalysisExprStmt<'src>;

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
                    ty.expect_type(&AbstractType::None, source, &self.expr)
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

impl<'src> CoagulateVars<'src> for AnalysisExprStmt<'src> {
    type Interp = InterpExprStmt<'src>;

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
pub struct PrintStmt<L, O, V> {
    pub expr: Expr<L, O, V>,
}

pub type AnalysisPrintStmt<'src> =
    PrintStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpPrintStmt<'src> = PrintStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> Analyze<'src> for ast::PrintStmt<'src> {
    type Ouptut = AnalysisPrintStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        const EXPR_TY: &AbstractType = &AbstractType::Text;
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

impl<'src> CoagulateVars<'src> for AnalysisPrintStmt<'src> {
    type Interp = InterpPrintStmt<'src>;

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
pub enum OrStmt<L, O, V> {
    More(IfStmt<L, O, V>),
    Last(BlockStmt<L, O, V>),
}

pub type AnalysisOrStmt<'src> = OrStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpOrStmt<'src> = OrStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisOrStmt<'src> {
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

impl<'src> CoagulateVars<'src> for AnalysisOrStmt<'src> {
    type Interp = InterpOrStmt<'src>;

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
    type Ouptut = AnalysisOrStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>, // TBD: does `or` return a value?
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::More(stmt) => stmt.analyze(source, env, None).map(OrStmt::More),
            Self::Last(stmt) => stmt.analyze(source, env, None).map(OrStmt::Last),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt<L, O, V> {
    /// Must output [`bool`]
    pub pred: Expr<L, O, V>,
    pub then: BlockStmt<L, O, V>,
    pub or: Option<Box<OrStmt<L, O, V>>>,
}

pub type AnalysisIfStmt<'src> = IfStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpIfStmt<'src> = IfStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisIfStmt<'src> {
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
    type Ouptut = AnalysisIfStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>, // TBD: does `if` return a value?
    ) -> Result<'src, Self::Ouptut> {
        let pred = self.pred.analyze(source, env, None)?;
        if let Some(ty) = pred.get_type(env) {
            ty.expect_type(&AbstractType::Bool, source, &self.pred)
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

impl<'src> CoagulateVars<'src> for AnalysisIfStmt<'src> {
    type Interp = InterpIfStmt<'src>;

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
pub struct LoopStmt<L, O, V> {
    pub label: Option<L>,
    pub block: BlockStmt<L, O, V>,
}

pub type AnalysisLoopStmt<'src> =
    LoopStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpLoopStmt<'src> = LoopStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for LoopStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Nevr)
    }
}

impl<'src> CoagulateVars<'src> for AnalysisLoopStmt<'src> {
    type Interp = InterpLoopStmt<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        LoopStmt {
            label: self.label.map(|label| label.coagulate_vars(s_env, i_env)),
            block: self.block.coagulate_vars(s_env, i_env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CordStmt<L, O, V> {
    pub label: Option<L>,
    pub block: BlockStmt<L, O, V>,
}

pub type AnalysisCordStmt<'src> =
    CordStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpCordStmt<'src> = CordStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisCordStmt<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Nevr)
    }
}

impl<'src> CoagulateVars<'src> for AnalysisCordStmt<'src> {
    type Interp = InterpCordStmt<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        CordStmt {
            label: self.label.map(|label| label.coagulate_vars(s_env, i_env)),
            block: self.block.coagulate_vars(s_env, i_env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt<L, O, V> {
    pub label: Option<L>,
    pub binding_decl: V,
    pub iter: Expr<L, O, V>, // TBD: what makes an expression iterable?
    pub block: BlockStmt<L, O, V>,
}

pub type AnalysisForStmt<'src> = ForStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpForStmt<'src> = ForStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisForStmt<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        todo!()
    }
}

impl<'src> CoagulateVars<'src> for AnalysisForStmt<'src> {
    type Interp = InterpForStmt<'src>;

    fn coagulate_vars(
        self,
        _s_env: &StaticEnvironment<'src>,
        _i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepStmt<L, O, V> {
    pub label: Option<L>,
    pub pred: Expr<L, O, V>,
    pub block: BlockStmt<L, O, V>,
}

pub type AnalysisRepStmt<'src> = RepStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpRepStmt<'src> = RepStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisRepStmt<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Nevr)
    }
}

impl<'src> CoagulateVars<'src> for AnalysisRepStmt<'src> {
    type Interp = InterpRepStmt<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        RepStmt {
            label: self.label.map(|x| x.coagulate_vars(s_env, i_env)),
            pred: self.pred.coagulate_vars(s_env, i_env),
            block: self.block.coagulate_vars(s_env, i_env),
        }
    }
}

impl<'src> Analyze<'src> for ast::LoopBlock<'src> {
    type Ouptut = AnalysisStatement<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        let label = self
            .label
            .as_ref()
            .map(|l| l.analyze(source, env, None))
            .transpose()?;
        Ok(match &self.kind {
            ast::LoopKind::Loop => Statement::Loop(LoopStmt {
                label,
                block: self.block.analyze(source, env, ret)?,
            }),
            ast::LoopKind::Cord => Statement::Cord(CordStmt {
                label,
                block: self.block.analyze(source, env, ret)?,
            }),
            ast::LoopKind::Rep { pred } => Statement::Rep(RepStmt {
                label,
                pred: pred.analyze(source, env, ret)?,
                block: self.block.analyze(source, env, ret)?,
            }),
            ast::LoopKind::For { bind, in_kw, iter } => Statement::For(ForStmt {
                label,
                binding_decl: todo!(), // TODO: this only supports the simplest form
                iter: iter.analyze(source, env, ret)?, // TBD: what is an iterable?
                block: self.block.analyze(source, env, ret)?,
            }),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HaltStmt<L, O, V> {
    pub label: Option<L>,
    pub value: Option<Expr<L, O, V>>,
}

pub type AnalysisHaltStmt<'src> =
    HaltStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpHaltStmt<'src> = HaltStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisHaltStmt<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        Some(&AbstractType::None)
    }
}

impl<'src> Analyze<'src> for ast::HaltStmt<'src> {
    type Ouptut = AnalysisHaltStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TODO: whose hint is this?
    ) -> Result<'src, Self::Ouptut> {
        Ok(HaltStmt {
            label: self
                .label
                .as_ref()
                .map(|label| label.analyze(source, env, None))
                .transpose()?,
            value: self
                .value
                .as_ref()
                .map(|value| value.analyze(source, env, ret))
                .transpose()?,
        })
    }
}

impl<'src> CoagulateVars<'src> for AnalysisHaltStmt<'src> {
    type Interp = InterpHaltStmt<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        InterpHaltStmt {
            label: self.label.map(|label| label.coagulate_vars(s_env, i_env)),
            value: self.value.map(|value| value.coagulate_vars(s_env, i_env)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipStmt<L> {
    pub label: Option<L>,
}

pub type AnalysisSkipStmt<'src> = SkipStmt<AnalysisLabel<'src>>;
pub type InterpSkipStmt<'src> = SkipStmt<InterpLabel<'src>>;

impl<'src> GetType<'src> for AnalysisSkipStmt<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&'ty self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        // TBD: should loops be expressions or just statements?
        Some(&AbstractType::Nevr)
    }
}

impl<'src> Analyze<'src> for ast::SkipStmt<'src> {
    type Ouptut = AnalysisSkipStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        _ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        Ok(SkipStmt {
            label: self
                .label
                .as_ref()
                .map(|label| label.analyze(source, env, None))
                .transpose()?,
        })
    }
}

impl<'src> CoagulateVars<'src> for AnalysisSkipStmt<'src> {
    type Interp = InterpSkipStmt<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        InterpSkipStmt {
            label: self.label.map(|label| label.coagulate_vars(s_env, i_env)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt<L, O, V> {
    pub stmts: Vec<Statement<L, O, V>>,
}

pub type AnalysisBlockStmt<'src> =
    BlockStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpBlockStmt<'src> = BlockStmt<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisBlockStmt<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        self.stmts
            .last()
            .map_or(Some(&AbstractType::None), |stmt| stmt.get_type(env))
        // TODO: find all cases where the block can be returned from
    }
}

fn analyze_defers<'ast, 'src: 'ast, I>(
    iter: I,
    source: &'src str,
    env: &mut StaticEnvironment<'src>,
    ret: Option<&AbstractType<'src>>, // TBD: does `block` return a value?
) -> Result<'src, AnalysisBlockStmt<'src>>
where
    I: IntoIterator<Item = &'ast ast::Declaration<'src>>,
{
    let mut iter = iter.into_iter();
    let (lo, hi) = iter.size_hint();
    let mut stmts = Vec::with_capacity(hi.unwrap_or(lo));
    while let Some(decl) = iter.next() {
        if let ast::Declaration::Defer(decl) = decl {
            stmts.push(Statement::Defer(DeferNode {
                on_pop: decl.on_pop.analyze(source, env, ret)?,
                ret_param: decl
                    .param
                    .as_ref()
                    .map(|x| {
                        let ident = x.param.ident.lex;
                        env.decl_var(
                            ident,
                            ret.cloned(),
                            true,
                            Some(Box::new(x.param.ident)), // TODO: what if there's multiple?
                        )
                        .map(|(unique_id, _, is_global)| AnalysisVar {
                            unique_id,
                            ident,
                            is_global,
                            _ty: None,
                        })
                        .map_err(|e| vec![ContextError::node_error(source, &x.param, e.into())])
                    })
                    .transpose()?,
                inner: analyze_defers(iter, source, env, ret)?,
            }));
            break;
        } else if let Some(stmt) = decl.analyze(source, env, None).transpose() {
            stmts.push(stmt?);
        }
    }
    Ok(BlockStmt { stmts })
}

impl<'src> Analyze<'src> for ast::BlockStmt<'src> {
    type Ouptut = AnalysisBlockStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>, // TBD: does `block` return a value?
    ) -> Result<'src, Self::Ouptut> {
        env.push_scope().unwrap_or_else(|| {
            todo!("warning: cannot push a scope here, need to be in a function");
        });
        let res = analyze_defers(&self.body, source, env, ret);
        if env.pop_scope().is_none() {
            todo!("scope pushes and pops should be balanced - tried to pop global scope");
        }
        res
    }
}

impl<'src> CoagulateVars<'src> for AnalysisBlockStmt<'src> {
    type Interp = InterpBlockStmt<'src>;

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
pub struct DeferNode<L, O, V> {
    pub inner: BlockStmt<L, O, V>,
    pub ret_param: Option<V>,
    pub on_pop: BlockStmt<L, O, V>,
}

pub type AnalysisDeferNode<'src> =
    DeferNode<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpDeferNode<'src> = DeferNode<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisDeferNode<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        // TBD: can defer change the return type?
        self.on_pop
            .get_type(env)
            .filter(|ty| !ty.is_compatible(&AbstractType::None))
            .or_else(|| self.inner.get_type(env))
    }
}

impl<'src> CoagulateVars<'src> for AnalysisDeferNode<'src> {
    type Interp = InterpDeferNode<'src>;

    fn coagulate_vars(
        self,
        s_env: &StaticEnvironment<'src>,
        i_env: &mut InterpEnvironment<'src>,
    ) -> Self::Interp {
        DeferNode {
            inner: self.inner.coagulate_vars(s_env, i_env),
            ret_param: self.ret_param.map(|x| x.coagulate_vars(s_env, i_env)),
            on_pop: self.on_pop.coagulate_vars(s_env, i_env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<L, O, V> {
    Expr(ExprStmt<L, O, V>),
    Print(PrintStmt<L, O, V>),
    If(IfStmt<L, O, V>),
    Loop(LoopStmt<L, O, V>),
    Cord(CordStmt<L, O, V>),
    For(ForStmt<L, O, V>),
    Rep(RepStmt<L, O, V>),
    Halt(HaltStmt<L, O, V>),
    Skip(SkipStmt<L>),
    Block(BlockStmt<L, O, V>),
    Defer(DeferNode<L, O, V>),
}

pub type AnalysisStatement<'src> =
    Statement<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpStatement<'src> = Statement<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

impl<'src> GetType<'src> for AnalysisStatement<'src> {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty,
        Self: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            Self::Expr(_) | Self::Print(_) => Some(&AbstractType::None),
            Self::If(stmt) => stmt.get_type(env),
            Self::Loop(stmt) => stmt.get_type(env),
            Self::Cord(stmt) => stmt.get_type(env),
            Self::For(stmt) => stmt.get_type(env),
            Self::Rep(stmt) => stmt.get_type(env),
            Self::Halt(stmt) => stmt.get_type(env),
            Self::Skip(stmt) => stmt.get_type(env),
            Self::Block(stmt) => stmt.get_type(env),
            Self::Defer(node) => node.get_type(env),
        }
    }
}

impl<'src> Analyze<'src> for ast::Statement<'src> {
    type Ouptut = AnalysisStatement<'src>;

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
            Self::Loop(stmt) => stmt.analyze(source, env, None),
            Self::Halt(stmt) => stmt.analyze(source, env, None).map(Statement::Halt),
            Self::Skip(stmt) => stmt.analyze(source, env, None).map(Statement::Skip),
            Self::Block(stmt) => stmt.analyze(source, env, None).map(Statement::Block),
        }
    }
}

impl<'src> CoagulateVars<'src> for AnalysisStatement<'src> {
    type Interp = InterpStatement<'src>;

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
            Self::For(stmt) => Statement::For(stmt.coagulate_vars(s_env, i_env)),
            Self::Rep(stmt) => Statement::Rep(stmt.coagulate_vars(s_env, i_env)),
            Self::Halt(stmt) => Statement::Halt(stmt.coagulate_vars(s_env, i_env)),
            Self::Skip(stmt) => Statement::Skip(stmt.coagulate_vars(s_env, i_env)),
            Self::Block(stmt) => Statement::Block(stmt.coagulate_vars(s_env, i_env)),
            Self::Defer(node) => Statement::Defer(node.coagulate_vars(s_env, i_env)),
        }
    }
}

impl<'src> Analyze<'src> for ast::DeclVar<'src> {
    type Ouptut = Option<ExprStmt<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> Result<'src, Self::Ouptut> {
        if let Some(ret) = ret
            && let Err(e) = ret.expect_type(&AbstractType::None, source, self)
        {
            return Err(vec![e]);
        }
        let ident = self.binding.ident.lex; // TODO: what if there's multiple?
        let annotated_type = self
            .ty
            .as_ref()
            .map(|annotation| annotation.ty.analyze(source, env));
        let expr = self
            .assignment
            .as_ref()
            .map(|x| x.expr.analyze(source, env, annotated_type.as_ref()))
            .transpose()?;
        let ty = annotated_type.clone().or_else(|| {
            expr.as_ref()
                .and_then(|x| x.get_type(env).map(|t| t.to_abstract()))
        });

        #[cfg(debug_assertions)]
        {
            print!("{}: ", self.binding.ident.lex); // TODO: what if there's multiple?
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

        if let Some(annotated_type) = &annotated_type
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
                    Some(Box::new(self.binding.ident)), // TODO: what if there's multiple?
                )
                .map(|()| None)
                .map_err(|e| {
                    vec![ContextError::node_error(
                        source,
                        self.ty
                            .as_ref()
                            .map(|x| -> &dyn AstNode { &x.ty })
                            .or_else(|| {
                                self.assignment
                                    .as_ref()
                                    .map(|x| -> &dyn AstNode { &x.expr })
                            })
                            .expect(
                                "type must have come from one of these two to produce an error",
                            ),
                        e.into(),
                    )]
                }),

            // Variable
            VarKeyword::Uni | VarKeyword::Let => env
                .decl_var(
                    ident,
                    ty,
                    true,
                    Some(Box::new(self.binding.ident)), // TODO: what if there's multiple?
                )
                .map_err(|e| {
                    ContextError::node_error(
                        source,
                        self.ty
                            .as_ref()
                            .map(|x| -> &dyn AstNode { &x.ty })
                            .or_else(|| {
                                self.assignment
                                    .as_ref()
                                    .map(|x| -> &dyn AstNode { &x.expr })
                            })
                            .expect(
                                "type must have come from one of these two to produce an error",
                            ),
                        e.into(),
                    )
                })
                .and_then(|(unique_id, ty, is_global)| {
                    if let Some(t) = annotated_type {
                        if let Some(ty) = ty {
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
    type Ouptut = Option<AnalysisStatement<'src>>;

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
            Self::Defer(_) => {
                // TODO: error when at global scope
                unimplemented!("should be handled by block")
            }
            Self::Stmt(stmt) => stmt.analyze(source, env, ret).map(Some),
        }
    }
}

/// A post-analysis program ready for execution
#[derive(Debug, Clone, Default)]
pub struct Program<L, O, V> {
    pub stmts: Vec<Statement<L, O, V>>,
}

pub type AnalysisProgram<'src> = Program<AnalysisLabel<'src>, AnalysisOp<'src>, AnalysisVar<'src>>;
pub type InterpProgram<'src> = Program<InterpLabel<'src>, InterpOp<'src>, InterpVar<'src>>;

fn analyze_base<'src, L, O, V, F>(
    source: &'src str,
    prgm: &ast::Program<'src>,
    f: F,
) -> Result<'src, Program<L, O, V>>
where
    F: FnOnce(
        StaticEnvironment<'src>,
        Vec<AnalysisStatement<'src>>,
    ) -> Result<'src, Program<L, O, V>>,
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
