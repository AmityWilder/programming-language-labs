use crate::{
    error::ContextError,
    exec::{env::Environment, eval::RunValue},
    grammar::ast::{Declaration, Program, Statement},
    scanner::token::value::{Keyword, LexValue},
};
use eval::Evaluate;

pub mod asm;
pub mod env;
pub mod error;
pub mod eval;

fn interpret<'src>(
    source: &'src str,
    env: &mut Environment<'src>,
    decl: &Declaration<'src>,
) -> Result<(), ContextError<'src>> {
    match decl {
        Declaration::Var(decl) => {
            let value = decl
                .assignment
                .as_ref()
                .map(|assignment| assignment.expr.evaluate(source, env))
                .transpose()?;
            #[expect(clippy::match_same_arms)]
            match decl.var_kw.val {
                LexValue::Keyword(Keyword::Uni) => (), // TODO
                LexValue::Keyword(Keyword::Pvt) => {
                    assert!(value.is_some(), "pvt must be assigned immediately");
                }
                LexValue::Keyword(Keyword::Let) => (), // TODO
                _ => unreachable!("should be caught by grammar"),
            }
            if let Some(value) = value.map(|x| -> RunValue<'static> {
                match x {
                    RunValue::None => RunValue::None,
                    RunValue::CoalesceNone => RunValue::CoalesceNone,
                    RunValue::Fail(e) => RunValue::Fail(e),
                    RunValue::Bool(x) => RunValue::Bool(x),
                    RunValue::UInt(x) => RunValue::UInt(x),
                    RunValue::SInt(x) => RunValue::SInt(x),
                    RunValue::Frac(x) => RunValue::Frac(x),
                    RunValue::Char(x) => RunValue::Char(x),
                    RunValue::Text(x) => RunValue::Text(std::borrow::Cow::Owned(x.into_owned())),
                    RunValue::Type(t) => RunValue::Type(t),
                    RunValue::Ptr(t, p) => RunValue::Ptr(t, p),
                }
            }) {
                let segment = env
                    .stack_mut()
                    .push(
                        decl.ident.lex,
                        value.typename(),
                        value.sizeof(),
                        value.alignof(),
                    )
                    .expect("out of memory"); // TODO: make this an error
                match value {
                    RunValue::None | RunValue::CoalesceNone => (),
                    RunValue::Fail(e) => todo!("how do we store fail? {e}"),
                    RunValue::Bool(x) => {
                        segment.copy_from_slice(std::slice::from_ref(&u8::from(x)));
                    }
                    RunValue::UInt(x) => segment.copy_from_slice(&x.to_ne_bytes()),
                    RunValue::SInt(x) => segment.copy_from_slice(&x.to_ne_bytes()),
                    RunValue::Frac(x) => segment.copy_from_slice(&x.to_ne_bytes()),
                    RunValue::Char(x) => segment.copy_from_slice(&x.to_u32().to_ne_bytes()),
                    RunValue::Text(x) => segment.copy_from_slice(x.as_bytes()), // TODO: length should be mutable for non-pvts!!
                    RunValue::Ptr(_, _) => todo!(),
                    RunValue::Type(_) => unimplemented!("typename should not exist at runtime"),
                }
            } else {
                todo!("variable declared without being defined; how big is it? what type is it?")
            }
        }

        Declaration::Stmt(stmt) => match stmt {
            Statement::Expr(expr) => _ = expr.expr.evaluate(source, env)?, // TBD: what do we do with the output?

            Statement::Print(stmt) => {
                let value = stmt.group.expr.evaluate(source, env)?;
                value.print();
            }

            Statement::If(stmt) => {
                let (mut pred, mut then) = (&stmt.cond.pred, &stmt.then);
                let do_block = loop {
                    let RunValue::Bool(x) = pred.evaluate(source, env)? else {
                        unimplemented!("should be caught by static analysis");
                    };
                    if x {
                        break Some(&then.then);
                    } else if let Some(or) = then.or.as_deref() {
                        if let Some(cond) = or.cond.as_ref() {
                            (pred, then) = (&cond.pred, &or.then);
                        } else {
                            debug_assert_eq!(
                                or.then.or, None,
                                "should not have `else {{}}` after `else {{}}`"
                            );
                            break Some(&or.then.then);
                        }
                    } else {
                        break None;
                    }
                };

                if let Some(block) = do_block {
                    for stmt in &block.body {
                        interpret(source, env, stmt)?;
                    }
                }
            }

            Statement::Block(block) => {
                for stmt in &block.body {
                    interpret(source, env, stmt)?;
                }
            }
        },
    }
    Ok(())
}

pub fn execute<'src>(source: &'src str, prgm: &Program<'src>) -> Result<(), ContextError<'src>> {
    const DEFAULT_MEM: usize = (1 << 16) - 1;
    let mut env = Environment::with_capacity(DEFAULT_MEM);
    for decl in prgm {
        interpret(source, &mut env, decl)?;
    }
    Ok(())
}
