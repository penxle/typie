#[test]
fn compile_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/valid_types.rs");
}
