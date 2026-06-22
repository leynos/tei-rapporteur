//! Property tests for `msgspec` bootstrap idempotency and thread safety.
//!
//! This module uses the mocks from `bootstrap_mocks` and restoration guards
//! from `test_helpers` to force the private bootstrap installer path without
//! invoking real package installation. The parent `mod` includes this module
//! alongside deterministic unit tests for the same helper surface.

use super::{
    super::{
        acquire_msgspec_bootstrap_lock_for_tests, ensure_msgspec_installed_unlocked_for_tests,
        force_msgspec_bootstrap_for_tests, reset_msgspec_init_for_tests,
    },
    bootstrap_mocks::setup_bootstrap_run_counter,
    test_helpers::{OwnedSubprocessRestoreGuard, acquire_subprocess_patch_lock},
};
use proptest::prelude::*;
use pyo3::Python;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        ..ProptestConfig::default()
    })]

    #[test]
    fn bootstrap_invariants_hold_without_process_isolation(
        repetitions in 1..=50u8,
        thread_count in 2..=32u8,
    ) {
        let bootstrap_guard = acquire_msgspec_bootstrap_lock_for_tests();
        let force_bootstrap_guard = force_msgspec_bootstrap_for_tests();
        let run_count = Arc::new(AtomicUsize::new(0));
        let setup_run_count = Arc::clone(&run_count);
        let subprocess_patch_guard = acquire_subprocess_patch_lock();

        let (globals, restored_patch_guard) = Python::attach(move |py| {
            py.import("msgspec")
                .expect("msgspec should be importable for bootstrap properties");
            let patch = setup_bootstrap_run_counter(py, setup_run_count, subprocess_patch_guard);
            reset_msgspec_init_for_tests();
            (patch.globals, patch.patch_guard)
        });
        let restore_guard = Python::attach(|py| OwnedSubprocessRestoreGuard {
            globals: globals.clone_ref(py),
            _patch_guard: restored_patch_guard,
        });

        let handles: Vec<_> = (0..usize::from(thread_count))
            .map(|_| thread::spawn(move || Python::attach(ensure_msgspec_installed_unlocked_for_tests)))
            .collect();

        for handle in handles {
            let result = handle.join().expect("bootstrap thread panicked");
            prop_assert!(result.is_ok());
        }

        for _ in 0..usize::from(repetitions) {
            let result = Python::attach(ensure_msgspec_installed_unlocked_for_tests);
            prop_assert!(result.is_ok());
        }

        drop(restore_guard);
        drop(force_bootstrap_guard);
        drop(bootstrap_guard);

        prop_assert_eq!(run_count.load(Ordering::SeqCst), 2);
    }
}
