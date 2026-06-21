//! Compile-fail tests for the `tei-py` Rust API surface.

use std::{
    ffi::OsString,
    sync::{Mutex, OnceLock},
};

fn env_guard() -> &'static Mutex<()> {
    static ENV_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_GUARD.get_or_init(|| Mutex::new(()))
}

struct CargoTargetDirRestore(Option<OsString>);

impl CargoTargetDirRestore {
    fn capture() -> Self {
        Self(std::env::var_os("CARGO_TARGET_DIR"))
    }
}

impl Drop for CargoTargetDirRestore {
    fn drop(&mut self) {
        // SAFETY: access to the process-global environment is serialised by
        // `env_guard`, and this restores the value observed at test entry.
        unsafe {
            match self.0.take() {
                Some(dir) => std::env::set_var("CARGO_TARGET_DIR", dir),
                None => std::env::remove_var("CARGO_TARGET_DIR"),
            }
        }
    }
}

#[test]
fn ui() {
    let _guard = env_guard()
        .lock()
        .expect("CARGO_TARGET_DIR environment guard should not be poisoned");
    let _target_dir_restore = CargoTargetDirRestore::capture();

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
        // SAFETY: access to the process-global environment is serialised by
        // `env_guard`, and the original value is restored before returning.
        unsafe { std::env::set_var("CARGO_TARGET_DIR", dir) };
    }

    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
