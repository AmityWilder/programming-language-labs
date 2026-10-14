use crate::{
    evaluate,
    exec::{env::Environment, eval::RunValue},
    grammar::ast::{Declaration, Program, Statement},
    scanner::token::value::{Keyword, LexValue},
};

pub mod env;
pub mod eval;

fn execute_decl<'src, 'prgm>(
    source: &'src str,
    env: &mut Environment<'src>,
    decl: &'prgm Declaration<'src>,
) -> Result<(), Box<dyn 'prgm + std::error::Error>> {
    match decl {
        Declaration::Var(decl) => {
            let value = decl
                .assignment
                .as_ref()
                .map(|assignment| evaluate(source, env, &assignment.expr))
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
            if let Some(value) = value {
                let segment = env
                    .declare_dyn(
                        decl.ident.lex,
                        value.as_type(),
                        value.alignof(),
                        value.sizeof(),
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
                    RunValue::Type(_) => unimplemented!("typename should not exist at runtime"),
                }
            } else {
                todo!("variable declared without being defined; how big is it? what type is it?")
            }
        }

        Declaration::Stmt(stmt) => match stmt {
            Statement::Expr(expr) => _ = evaluate(source, env, &expr.expr)?, // TBD: what do we do with the output?

            Statement::Print(stmt) => {
                let value = evaluate(source, env, &stmt.group.expr)?;
                value.print();
            }

            Statement::If(stmt) => {
                let (mut pred, mut then) = (&stmt.cond.pred, &stmt.then);
                let do_block = loop {
                    let RunValue::Bool(x) = evaluate(source, env, pred)? else {
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
                        execute_decl(source, env, stmt)?;
                    }
                }
            }

            Statement::Block(block) => {
                for stmt in &block.body {
                    execute_decl(source, env, stmt)?;
                }
            }
        },
    }
    Ok(())
}

pub fn execute<'src, 'prgm>(
    source: &'src str,
    prgm: &'prgm Program<'src>,
) -> Result<(), Box<dyn 'prgm + std::error::Error>> {
    const DEFAULT_MEM: usize = (1 << 16) - 1;
    let mut env = Environment::with_capacity(DEFAULT_MEM);
    for decl in prgm {
        execute_decl(source, &mut env, decl)?;
    }
    Ok(())
}
