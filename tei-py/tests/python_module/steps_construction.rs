//! Steps covering module initialisation and document construction.

use super::state::{
    PythonModuleState, construct_python_document, module_is_initialised, python_state,
};
use anyhow::{Context, Result};
use pyo3::{Python, types::PyAnyMethods};
use rstest_bdd_macros::{given, scenario, when};

const _: fn() -> PythonModuleState = python_state;

#[given("the tei_rapporteur Python module is initialised")]
pub(super) fn module_is_initialised_step(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    module_is_initialised(state)
}

#[when("I construct a Document titled \"{title}\"")]
pub(super) fn i_construct_a_document(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    construct_python_document(state, &title)
}

#[when("I construct a Document with the XML special characters fixture")]
pub(super) fn i_construct_the_xml_special_fixture_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    construct_python_document(state, "Special <Title> & \"Quotes\" and 'Apostrophes'")
}

#[when("I emit title markup for \"{title}\"")]
pub(super) fn i_emit_title_markup(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    Python::with_gil(|py| {
        state.with_module(py, |module| {
            let emit = module
                .getattr("emit_title_markup")
                .context("emit_title_markup must be registered")?;
            match emit.call1((title.as_str(),)) {
                Ok(markup) => state.store_markup(markup.extract::<String>()?),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[when("I emit markup from the constructed Document")]
pub(super) fn i_emit_markup_from_the_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let markup = Python::with_gil(|py| {
        state.with_document(py, |document| {
            let markup: String = document.call_method0("emit_title_markup")?.extract()?;
            Ok::<_, anyhow::Error>(markup)
        })
    })?;
    state.store_markup(markup);
    Ok(())
}

#[scenario(path = "tests/features/python_module.feature", index = 0)]
pub fn constructs_a_document(python_state: PythonModuleState) {
    let _ = python_state;
}

#[scenario(path = "tests/features/python_module.feature", index = 1)]
pub fn rejects_blank_titles(python_state: PythonModuleState) {
    let _ = python_state;
}

#[scenario(path = "tests/features/python_module.feature", index = 2)]
pub fn emits_title_markup(python_state: PythonModuleState) {
    let _ = python_state;
}

#[scenario(path = "tests/features/python_module.feature", index = 3)]
pub fn document_markup_escapes_special_characters(python_state: PythonModuleState) {
    let _ = python_state;
}
