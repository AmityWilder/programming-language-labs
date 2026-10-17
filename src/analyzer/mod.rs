use crate::{
    Todo,
    analyzer::{
        static_env::StaticEnvironment,
        types::{AbstractType, StaticType},
    },
    error::ContextError,
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
    fn get_type(&self) -> AbstractType<'src>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operands {
    Binary { lhs: Expr, rhs: Expr },
    UnaryPre { rhs: Expr },
    UnaryPost { lhs: Expr },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    pub op: Operator,
    pub operands: Operands,
}

impl<'src> Expression<'src> for Operation {
    fn get_type(&self) -> AbstractType<'src> {
        todo!()
    }
}

// TODO: what about literals for custom types and arrays?
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    None(none),
    Nevr(nevr),
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
    fn get_type(&self) -> AbstractType<'src> {
        #[expect(clippy::ignored_unit_patterns, reason = "prettier this way")]
        AbstractType::Concrete(match self {
            Self::None(_) => StaticType::None,
            Self::Nevr(_) => StaticType::Nevr,
            Self::Fail(_) => StaticType::Fail,
            Self::Bool(_) => StaticType::Bool,
            Self::Byte(_) => StaticType::Byte,
            Self::Uint(_) => StaticType::Uint,
            Self::Sint(_) => StaticType::Sint,
            Self::Flpt(_) => StaticType::Flpt,
            Self::Char(_) => StaticType::Char,
            Self::Text(_) => StaticType::Text,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variable {}

impl<'src> Expression<'src> for Variable {
    fn get_type(&self) -> AbstractType<'src> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Op(Box<Operation>),
    Literal(Literal),
    Variable(Variable),
}

impl<'src> Expression<'src> for Expr {
    fn get_type(&self) -> AbstractType<'src> {
        match self {
            Self::Op(x) => x.get_type(),
            Self::Literal(x) => x.get_type(),
            Self::Variable(x) => x.get_type(),
        }
    }
}

impl<'src> Analyze<'src> for ast::Expr<'src> {
    type Ouptut = Expr;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self {
            Self::Binary(x) => x.analyze(source, env),
            Self::Unary(x) => x.analyze(source, env),
            Self::Literal(x) => x.analyze(source, env).map(Expr::Literal),
            Self::Variable(x) => x.analyze(source, env),
            Self::Grouping(x) => x.expr.analyze(source, env),
            Self::Type(x) => todo!(),
        }
    }
}

impl<'src> Analyze<'src> for ast::Literal<'src> {
    type Ouptut = Literal;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        todo!()
    }
}

impl<'src> Analyze<'src> for ast::Variable<'src> {
    type Ouptut = Expr;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let name = self.ident.lex;
        env.get(name)
            .map(|var| todo!()) // remember: could be a constant
            .ok_or_else(|| todo!("error: variable {name} not in scope"))
    }
}

impl<'src> Analyze<'src> for ast::Unary<'src> {
    type Ouptut = Expr;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let op = self.op.val;

        todo!()
    }
}

impl<'src> Analyze<'src> for ast::Binary<'src> {
    type Ouptut = Expr;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        let op = self.op.val;

        todo!()
    }
}

/// An [`Expr`] that returns [`none`]
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    expr: Expr,
}

impl<'src> Analyze<'src> for ast::ExprStmt<'src> {
    type Ouptut = ExprStmt;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintStmt {}

impl<'src> Analyze<'src> for ast::PrintStmt<'src> {
    type Ouptut = PrintStmt;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrStmt {
    More(IfStmt),
    Last(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    /// Must output [`bool`]
    pub pred: Expr,
    pub then: BlockStmt,
    pub or: Option<Box<OrStmt>>,
}

impl<'src> Analyze<'src> for ast::IfStmt<'src> {
    type Ouptut = IfStmt;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    pub stmts: Vec<Statement>,
}

impl<'src> Analyze<'src> for ast::BlockStmt<'src> {
    type Ouptut = BlockStmt;

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
pub enum Statement {
    Expr(ExprStmt),
    Print(PrintStmt),
    If(IfStmt),
    Block(BlockStmt),
}

impl<'src> Analyze<'src> for ast::Statement<'src> {
    type Ouptut = Statement;

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
    type Ouptut = Option<ExprStmt>;

    fn analyze(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
    ) -> Result<'src, Self::Ouptut> {
        match self.var_kw.val {
            // Constant
            LexValue::Keyword(Keyword::Pvt) => todo!(),

            // Global
            LexValue::Keyword(Keyword::Uni) => todo!(),

            // Local
            LexValue::Keyword(Keyword::Let) => todo!(),

            _ => unimplemented!("should be caught by grammar"),
        }
    }
}

impl<'src> Analyze<'src> for ast::Declaration<'src> {
    type Ouptut = Option<Statement>;

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
pub struct Program {
    stmts: Vec<Statement>,
}

pub fn analyze<'src>(source: &'src str, prgm: &ast::Program<'src>) -> Result<'src, Program> {
    let mut env = StaticEnvironment::new();
    prgm.decls
        .iter()
        .filter_map(|decl| decl.analyze(source, &mut env).transpose())
        .collect::<Result<'src, Vec<_>>>()
        .map(|stmts| Program { stmts })
}
