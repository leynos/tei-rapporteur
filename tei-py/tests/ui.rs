//! Compile-fail UI tests for the `tei-py` Rust API surface.
//!
//! This module is a [`trybuild`] test harness. The single `ui` test calls
//! [`trybuild::TestCases::compile_fail`] with the glob `tests/ui/*.rs`, which
//! discovers every `*.rs` fixture under `tei-py/tests/ui/` and compiles each
//! one as an independent external crate. Each fixture **must** have a committed
//! `.stderr` snapshot alongside it (same stem, `.stderr` extension); trybuild
//! diffs the compiler output against that snapshot and fails the test on any
//! divergence.
//!
//! To add a new fixture: create the `.rs` file, run
//! `cargo test -p tei-py --test ui`, copy the generated snapshot from
//! `tei-py/wip/` to `tei-py/tests/ui/`, then commit both files together.

#[test]
fn ui() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
