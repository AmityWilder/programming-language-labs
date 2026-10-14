use crate::{
    grammar::ast::{Declaration, Statement},
    scanner::token::value::{Keyword, LexValue},
};

pub mod env;
pub mod eval;

pub fn execute<'src, 'prgm, I>(prgm: I) -> Result<(), Box<dyn 'prgm + std::error::Error>>
where
    'src: 'prgm,
    I: IntoIterator<Item = &'prgm Declaration<'src>>,
{
    for decl in prgm {
        match decl {
            Declaration::Var(decl) => match decl.var_kw.val {
                LexValue::Keyword(Keyword::Uni) => todo!(),
                LexValue::Keyword(Keyword::Pvt) => todo!(),
                LexValue::Keyword(Keyword::Let) => todo!(),
                _ => unreachable!("should be caught by grammar"),
            },

            Declaration::Stmt(stmt) => match stmt {
                Statement::Expr(expr) => todo!(),
                Statement::If(if_stmt) => todo!(),
                Statement::Block(block) => return execute(block.body.as_slice()),
            },
        }
    }
    Ok(())
}
