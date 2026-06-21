//! Compile-fail tests for the `tei-py` Rust API surface.

#[test]
fn ui() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
