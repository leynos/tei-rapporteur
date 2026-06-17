//! Property tests for `msgspec` bootstrap idempotency and thread safety.

use super::{
    super::ensure_msgspec_installed,
    bootstrap_mocks::{remove_msgspec_from_modules, setup_bootstrap_run_counter},
    test_helpers::restore_subprocess_run,
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
        // `MSGSPEC_INIT` is process-wide, so each generated case needs its own
        // nextest process to exercise the forced bootstrap path. One case still
        // verifies the correctness goal: a generated N drives repeated calls or
        // spawned threads while the exact subprocess count proves the `Once`
        // guard ran the installer once and only once.
        cases: 1,
        ..ProptestConfig::default()
    })]

    #[test]
    fn idempotency_holds_over_arbitrary_repetitions(repetitions in 1..=50u8) {
        let run_count = Arc::new(AtomicUsize::new(0));

        let globals = Python::attach(|py| {
            let g = setup_bootstrap_run_counter(py, Arc::clone(&run_count));
            remove_msgspec_from_modules(py);
            g
        });

        for _ in 0..usize::from(repetitions) {
            let result = Python::attach(ensure_msgspec_installed);
            prop_assert!(result.is_ok());
        }

        let restored_after_bootstrap = Python::attach(|py| {
            restore_subprocess_run(py, globals.bind(py));
            ensure_msgspec_installed(py)
        })
        .is_ok();
        prop_assert!(restored_after_bootstrap);

        prop_assert_eq!(run_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn no_panics_under_variable_thread_count(thread_count in 2..=32u8) {
        let run_count = Arc::new(AtomicUsize::new(0));
        let globals = Python::attach(|py| {
            let globals = setup_bootstrap_run_counter(py, Arc::clone(&run_count));
            remove_msgspec_from_modules(py);

            globals
        });

        let handles: Vec<_> = (0..usize::from(thread_count))
            .map(|_| thread::spawn(move || Python::attach(ensure_msgspec_installed)))
            .collect();

        for handle in handles {
            let result = handle.join().expect("bootstrap thread panicked");
            prop_assert!(result.is_ok());
        }

        let restored_after_bootstrap = Python::attach(|py| {
            restore_subprocess_run(py, globals.bind(py));
            ensure_msgspec_installed(py)
        })
        .is_ok();
        prop_assert!(restored_after_bootstrap);

        prop_assert_eq!(run_count.load(Ordering::SeqCst), 2);
    }
}
