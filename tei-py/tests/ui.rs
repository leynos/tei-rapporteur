//! Compile-fail tests for the `tei-py` Rust API surface.

#[test]
fn ui() {
    // When the workspace was compiled with cargo-llvm-cov (or any tool that
    // redirects the target directory), forward that directory to trybuild so
    // its sub-cargo invocation can reuse the already-compiled artefacts
    // instead of starting a cold build.
    //
    // cargo-llvm-cov exports CARGO_LLVM_COV_TARGET_DIR; a plain nextest run
    // or a CI step may instead set CARGO_TARGET_DIR directly.  We honour
    // whichever is present, preferring CARGO_TARGET_DIR.
    if std::env::var_os("CARGO_TARGET_DIR").is_none()
        && let Ok(dir) = std::env::var("CARGO_LLVM_COV_TARGET_DIR")
    {
        // SAFETY: single-threaded at this point in the test harness.
        unsafe { std::env::set_var("CARGO_TARGET_DIR", dir) };
    }

    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
