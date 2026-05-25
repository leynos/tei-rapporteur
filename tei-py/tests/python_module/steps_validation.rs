//! Steps covering document validation via the Python module.

use super::state::{PythonModuleState, python_state};
use anyhow::{Result, ensure};
use pyo3::{Python, types::PyAnyMethods};
use rstest_bdd_macros::{scenario, then, when};

const _: fn() -> PythonModuleState = python_state;

#[when("I validate the constructed Document")]
pub(super) fn i_validate_the_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    Python::attach(|py| {
        state.with_document(py, |document| match document.call_method0("validate") {
            Ok(_) => Ok(()),
            Err(error) => {
                state.store_error(error.to_string());
                Ok(())
            }
        })
    })
}

#[then("validation succeeds")]
pub(super) fn validation_succeeds(#[from(python_state)] state: &PythonModuleState) -> Result<()> {
    // If no error was stored, validation succeeded
    ensure!(
        state.error().is_err(),
        "expected validation to succeed but an error was recorded: {}",
        state.error().unwrap_or_default()
    );
    Ok(())
}

/// Scenario: Validate a well-formed Document.
#[scenario(path = "tests/features/python_module.feature", index = 22)]
pub fn validates_well_formed_document(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Reject Documents with duplicate xml:id values.
#[scenario(path = "tests/features/python_module.feature", index = 23)]
pub fn rejects_duplicate_identifiers(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Reject Documents with unknown speaker references.
#[scenario(path = "tests/features/python_module.feature", index = 24)]
pub fn rejects_unknown_speakers(python_state: PythonModuleState) {
    let _ = python_state;
}
