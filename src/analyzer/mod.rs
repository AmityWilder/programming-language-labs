use crate::{
    Todo,
    analyzer::{
        static_env::{OpOverload, OperandTypes, StaticEnvironment, Var},
        types::{AbstractType, StaticType},
    },
    error::{ContextError, ErrorType, OpSide},
    grammar::ast,
    scanner::token::value::{Keyword, LexValue, Operator},
    types::*,
};

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
    /// The type output by the expression
    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>>;
}

const BUILTIN_OPS: [OpOverload<'_>; 1] = [OpOverload {
    op: Operator::And,
    operands: OperandTypes::Binary(
        AbstractType::Concrete(StaticType::Bool),
        AbstractType::Concrete(StaticType::Bool),
    ),
    output: AbstractType::Concrete(StaticType::Bool),
}];

#[derive(Debug, Clone, PartialEq)]
pub enum Operands<'src> {
    Binary(Expr<'src>, Expr<'src>),
    UnaryPre(Expr<'src>),
    UnaryPost(Expr<'src>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operation<'src> {
    pub op: Operator,
    pub operands: Operands<'src>,
}

impl<'src> Expression<'src> for Operation<'src> {
    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        env.get_overload(self).map(|x| &x.output)
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
    fn get_type<'ty>(
        &'ty self,
        _: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        Some(match self {
            Self::None => const { &AbstractType::Concrete(StaticType::None) },
            Self::Nevr => const { &AbstractType::Concrete(StaticType::Nevr) },
            Self::Fail(_) => const { &AbstractType::Concrete(StaticType::Fail) },
            Self::Bool(_) => const { &AbstractType::Concrete(StaticType::Bool) },
            Self::Byte(_) => const { &AbstractType::Concrete(StaticType::Byte) },
            Self::Uint(_) => const { &AbstractType::Concrete(StaticType::Uint) },
            Self::Sint(_) => const { &AbstractType::Concrete(StaticType::Sint) },
            Self::Flpt(_) => const { &AbstractType::Concrete(StaticType::Flpt) },
            Self::Char(_) => const { &AbstractType::Concrete(StaticType::Char) },
            Self::Text(_) => const { &AbstractType::Concrete(StaticType::Text) },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable<'src> {
    pub ident: &'src str,
}

impl<'src> Expression<'src> for Variable<'src> {
    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        env.get(self.ident).and_then(|v| v.get_type(env))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Op(Box<Operation<'src>>),
    Literal(Literal),
    Variable(Variable<'src>),
}

impl<'src> Expression<'src> for Expr<'src> {
    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        match self {
            Self::Op(x) => x.get_type(env),
            Self::Literal(x) => x.get_type(env),
            Self::Variable(x) => x.get_type(env),
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
            .map(|_| Expr::Variable(Variable { ident })) // remember: could be a constant
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
        self.operand.analyze(source, env).map(|operand| Operation {
            op: self.op.val,
            operands: match self.side {
                OpSide::Left => Operands::UnaryPost(operand),
                OpSide::Right => Operands::UnaryPre(operand),
            },
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
        Ok(Operation {
            op: self.op.val,
            operands: Operands::Binary(
                self.lhs.analyze(source, env)?,
                self.rhs.analyze(source, env)?,
            ),
        })
    }
}

/// An [`Expr`] that returns [`none`]
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt<'src> {
    expr: Expr<'src>,
}

impl<'src> Analyze<'src> for ast::ExprStmt<'src> {
    type Ouptut = ExprStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        self.expr.analyze(source, env).and_then(|expr| {
            if let Some(ty) = expr.get_type(env) {
                if *ty == AbstractType::Concrete(StaticType::None) {
                    Ok(ExprStmt { expr })
                } else {
                    Err(todo!("error: expected none, found {ty:?}"))
                }
            } else {
                Err(todo!(
                    "error: expression has no type\n expression: {expr:?}"
                ))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt<'src> {
    expr: Expr<'src>,
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
                if ty == &AbstractType::Concrete(StaticType::Text) {
                    Ok(PrintStmt { expr })
                } else {
                    Err(todo!("error: expected text, found {ty:?}"))
                }
            } else {
                Err(todo!(
                    "error: expression has no type\n expression: {expr:?}"
                ))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrStmt<'src> {
    More(IfStmt<'src>),
    Last(BlockStmt<'src>),
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

impl<'src> Analyze<'src> for ast::IfStmt<'src> {
    type Ouptut = IfStmt<'src>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let pred = self.pred.analyze(source, env)?;
        if let Some(ty) = pred.get_type(env) {
            if ty == &AbstractType::Concrete(StaticType::Bool) {
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
            } else {
                Err(todo!("error: expected a bool, found {ty:?}"))
            }
        } else {
            Err(todo!("error: predicate has no type\n predicate: {pred:?}"))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt<'src> {
    pub stmts: Vec<Statement<'src>>,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'src> {
    Expr(ExprStmt<'src>),
    Print(PrintStmt<'src>),
    If(IfStmt<'src>),
    Block(BlockStmt<'src>),
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
                    expr.as_ref().and_then(|x| x.get_type(env)).cloned(),
                );
                Ok(expr.map(|expr| ExprStmt { expr }))
            }

            // Local
            LexValue::Keyword(Keyword::Let) => env
                .decl_local_var(
                    self.ident.lex,
                    expr.as_ref().and_then(|x| x.get_type(env)).cloned(),
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program<'src> {
    stmts: Vec<Statement<'src>>,
}

pub fn analyze<'src>(source: &'src str, prgm: &ast::Program<'src>) -> Result<'src, Program<'src>> {
    let mut env = StaticEnvironment::new(BUILTIN_OPS.to_vec());
    prgm.decls
        .iter()
        .filter_map(|decl| decl.analyze(source, &mut env).transpose())
        .collect::<Result<'src, Vec<_>>>()
        .map(|stmts| Program { stmts })
}
