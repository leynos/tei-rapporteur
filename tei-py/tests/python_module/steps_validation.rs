//! Steps covering document validation via the Python module.

use super::state::{PythonModuleState, python_state};
use anyhow::{Result, ensure};
use pyo3::types::PyAnyMethods;
use rstest_bdd_macros::{scenario, then, when};
use tei_py::test_support::with_python;

#[when("I validate the constructed Document")]
pub(super) fn i_validate_the_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    with_python(|py| {
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
#[scenario(
    path = "tests/features/python_module.feature",
    name = "Validate a well-formed Document"
)]
pub fn validates_well_formed_document(#[from(python_state)] _python_state: PythonModuleState) {}

/// Scenario: Reject Documents with duplicate xml:id values.
#[scenario(
    path = "tests/features/python_module.feature",
    name = "Reject Documents with duplicate xml:id values"
)]
pub fn rejects_duplicate_identifiers(#[from(python_state)] _python_state: PythonModuleState) {}

/// Scenario: Reject Documents with unknown speaker references.
#[scenario(
    path = "tests/features/python_module.feature",
    name = "Reject Documents with unknown speaker references"
)]
pub fn rejects_unknown_speakers(#[from(python_state)] _python_state: PythonModuleState) {}
