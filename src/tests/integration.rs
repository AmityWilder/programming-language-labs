use crate::{
    Cli,
    analyzer::static_env::scope::env::ScopeEnvironment,
    compile,
    src_code::{
        code::{SourceCode, SourceLoc},
        map::SourceMap,
    },
};
use std::path::PathBuf;

macro_rules! integration_test {
    ($file:expr, $modname:expr) => {{
        let mut source = SourceMap::new();
        let loc = SourceLoc::File(PathBuf::from($file), $modname.to_string());
        source.push(
            SourceCode::new(loc, include_str!(concat!("../../", $file)).to_string())
                .expect("should be valid size"),
        );
        let mut diagnostics = Vec::new();
        let mut scope_env = ScopeEnvironment::new();
        let res = compile(
            &Cli::default(),
            &mut source,
            &mut diagnostics,
            &mut scope_env,
        );
        (res, diagnostics)
    }};
}

#[test]
fn test_exec() {
    let res = integration_test!("example/exec.bats", "exec");
    res.0.expect("should compile");
}

#[test]
fn test_fruit() {
    let res = integration_test!("example/fruit.bats", "fruit");
    res.0.expect("should compile");
}

#[test]
fn test_dfer() {
    let res = integration_test!("example/dfer.bats", "dfer");
    res.0.expect("should compile");
}

#[test]
fn test_cond() {
    let res = integration_test!("example/cond.bats", "cond");
    res.0.expect("should compile");
}

#[test]
fn test_label() {
    let res = integration_test!("example/label.bats", "label");
    res.0.expect("should compile");
}

#[test]
fn test_complex() {
    let res = integration_test!("example/complex.bats", "complex");
    res.0.expect("should compile");
}

#[test]
fn test_preproc() {
    let res = integration_test!("example/preproc.bats", "preproc");
    res.0.expect("should compile");
}
