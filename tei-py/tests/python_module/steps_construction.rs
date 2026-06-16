//! Steps covering module initialisation and document construction.

use super::state::{
    PythonModuleState, construct_python_document, module_is_initialised, python_state,
};
use anyhow::{Context, Result};
use pyo3::{Python, types::PyAnyMethods};
use pyo3_serde::to_pyobject;
use rstest_bdd_macros::{given, scenario, when};
use tei_core::{
    BodyBlock, Div, FileDesc, Head, Item, Label, List, P, TeiBody, TeiDocument, TeiHeader, TeiText,
};
use tei_py::projection::document_to_value;

const _: fn() -> PythonModuleState = python_state;

pub(super) fn div_body_document_fixture() -> Result<TeiDocument> {
    let header = TeiHeader::new(FileDesc::from_title_str("Bridgewater")?);
    let mut div = Div::new("show-notes")?;
    div.set_id("div1")?;
    div.set_subtype("chapter-markers")?;
    div.set_head(Head::from_text("Chapter markers")?);
    div.push_paragraph(P::from_text_segments(["Further reading"])?);

    let mut item = Item::from_text_segments(["Transcript"])?;
    item.set_label(Label::from_text("1.")?);
    let list = List::new([item])?;

    let mut child = Div::new("segment")?;
    child.set_subtype("guest-bio")?;
    child.set_head(Head::from_text("Guest bios")?);
    child.push_list(list);
    div.push_div(child);

    let text = TeiText::new(TeiBody::new([BodyBlock::Div(div)]));
    Ok(TeiDocument::new(header, text))
}

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

#[given("I construct a Document with div body content")]
pub(super) fn i_construct_a_document_with_div_body_content(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = document_to_value(&div_body_document_fixture()?)
        .context("serialising div fixture to JSON should succeed")?;
    Python::attach(|py| {
        state.with_module(py, |module| {
            let decoder = module
                .getattr("from_dict")
                .context("from_dict must be registered")?;
            let py_payload =
                to_pyobject(py, &payload).context("converting fixture to Python should succeed")?;
            match decoder.call1((py_payload,)) {
                Ok(document) => state.store_document(document.unbind()),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
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
    Python::attach(|py| {
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
    let markup = Python::attach(|py| {
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
