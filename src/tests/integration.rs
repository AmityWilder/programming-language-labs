use crate::{
    Cli, compile,
    src_code::{
        code::{SourceCode, SourceLoc},
        map::SourceMap,
    },
};
use std::path::PathBuf;

macro_rules! integration_test {
    ($file:expr) => {{
        let mut source = SourceMap::new();
        let loc = SourceLoc::File(PathBuf::from($file));
        source.push(
            SourceCode::new(loc, include_str!(concat!("../../", $file)).to_string())
                .expect("should be valid size"),
        );
        let res = compile(&mut source, &Cli::default());
        res
    }};
}

#[test]
fn test_exec() {
    let res = integration_test!("example/exec.bats");
    assert!(res.0.is_ok());
}

#[test]
fn test_fruit() {
    let res = integration_test!("example/fruit.bats");
    assert!(res.0.is_ok());
}

#[test]
fn test_defer() {
    let res = integration_test!("example/defer.bats");
    assert!(res.0.is_ok());
}

#[test]
fn test_cond() {
    let res = integration_test!("example/cond.bats");
    assert!(res.0.is_ok());
}

#[test]
fn test_label() {
    let res = integration_test!("example/label.bats");
    assert!(res.0.is_ok());
}

#[test]
fn test_complex() {
    let res = integration_test!("example/complex.bats");
    assert!(res.0.is_ok());
}

#[test]
fn test_preproc() {
    let res = integration_test!("example/preproc.bats");
    assert!(res.0.is_ok());
}
