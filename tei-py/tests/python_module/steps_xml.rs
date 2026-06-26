//! XML-oriented steps and scenarios.

use super::state::{PythonModuleState, construct_python_document, python_state};
use anyhow::{Context, Result};
use pyo3::types::PyAnyMethods;
use rstest_bdd_macros::{given, scenario, when};
use tei_core::TeiDocument;
use tei_py::test_support::with_python;
use tei_xml::emit_xml as emit_document_xml;

const _: fn() -> PythonModuleState = python_state;

#[given("I provide TEI XML titled \"{title}\"")]
pub(super) fn i_provide_tei_xml_titled(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    let document = TeiDocument::from_title_str(title.as_str())
        .context("XML fixtures must construct valid documents")?;
    let xml = emit_document_xml(&document).context("emitting XML fixtures should succeed")?;
    state.store_xml_payload(xml);
    Ok(())
}

#[given("I provide an invalid TEI XML payload missing the header")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "rstest-bdd step signatures stay uniform even when storing literals"
)]
pub(super) fn i_provide_an_invalid_tei_xml_payload(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    state.store_xml_payload("<TEI><text><body/></text></TEI>".to_owned());
    Ok(())
}

#[when("I construct a Document with the XML control character fixture")]
pub(super) fn i_construct_the_xml_control_character_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    construct_python_document(state, "\u{0}")
}

#[when("I parse the TEI XML payload")]
pub(super) fn i_parse_the_tei_xml_payload(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let xml = state.xml_payload()?;
    with_python(|py| {
        state.with_module(py, |module| {
            let parser = module
                .getattr("parse_xml")
                .context("parse_xml must be registered")?;
            match parser.call1((xml.as_str(),)) {
                Ok(document) => state.store_document(document.unbind()),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[when("I emit the constructed Document to TEI XML")]
pub(super) fn i_emit_the_document_to_tei_xml(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    with_python(|py| {
        state.with_module(py, |module| {
            let emitter = module
                .getattr("emit_xml")
                .context("emit_xml must be registered")?;
            state.with_document(py, |document| {
                match emitter.call1((document,)) {
                    Ok(payload) => {
                        let xml: String = payload.extract()?;
                        state.store_xml_output(xml);
                    }
                    Err(error) => state.store_error(error.to_string()),
                }
                Ok::<(), anyhow::Error>(())
            })
        })
    })?;
    Ok(())
}

#[scenario(
    path = "tests/features/python_module.feature",
    name = "Parse TEI XML into a Document"
)]
#[expect(
    unused_variables,
    reason = "rstest-bdd matches scenario fixtures by parameter name"
)]
pub fn parses_tei_xml_payloads(python_state: PythonModuleState) {}

#[scenario(
    path = "tests/features/python_module.feature",
    name = "Reject malformed TEI XML payloads"
)]
#[expect(
    unused_variables,
    reason = "rstest-bdd matches scenario fixtures by parameter name"
)]
pub fn rejects_invalid_tei_xml_payloads(python_state: PythonModuleState) {}

#[scenario(
    path = "tests/features/python_module.feature",
    name = "Emit TEI XML for a Document"
)]
#[expect(
    unused_variables,
    reason = "rstest-bdd matches scenario fixtures by parameter name"
)]
pub fn emits_documents_to_tei_xml(python_state: PythonModuleState) {}

#[scenario(
    path = "tests/features/python_module.feature",
    name = "Reject TEI emission when XML would contain forbidden characters"
)]
#[expect(
    unused_variables,
    reason = "rstest-bdd matches scenario fixtures by parameter name"
)]
pub fn rejects_emit_xml_with_control_characters(python_state: PythonModuleState) {}
