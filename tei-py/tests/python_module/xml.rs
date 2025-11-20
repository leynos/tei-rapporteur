use super::shared::*;
use anyhow::{Context, Result, ensure};
use pyo3::prelude::*;
use rstest_bdd_macros::{given, scenario, then, when};
use tei_core::TeiDocument;
use tei_xml::emit_xml as emit_document_xml;

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
#[given("I provide TEI XML titled \"{title}\"")]
fn i_provide_tei_xml_titled(
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
fn i_provide_an_invalid_tei_xml_payload(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    state.store_xml_payload("<TEI><text><body/></text></TEI>".to_owned());
    Ok(())
}

#[when("I construct a Document with the XML control character fixture")]
fn i_construct_the_xml_control_character_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    construct_python_document(state, "\u{0}")
}

#[when("I parse the TEI XML payload")]
fn i_parse_the_tei_xml_payload(#[from(python_state)] state: &PythonModuleState) -> Result<()> {
    let xml = state.xml_payload()?;
    Python::with_gil(|py| {
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
#[expect(
    clippy::excessive_nesting,
    reason = "rstest-bdd steps need nested Python contexts to access the module and stored Document"
)]
fn i_emit_the_document_to_tei_xml(#[from(python_state)] state: &PythonModuleState) -> Result<()> {
    Python::with_gil(|py| {
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

#[then("the TEI XML output equals the canonical payload for \"{title}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
fn the_tei_xml_output_equals_the_canonical_payload(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    let document = TeiDocument::from_title_str(title.as_str())
        .context("expected title should construct a valid document")?;
    let expected = emit_document_xml(&document).context("expected canonical XML emission")?;
    let actual = state.xml_output()?;
    ensure!(
        actual == expected,
        "expected TEI XML {expected:?}, found {actual:?}"
    );
    Ok(())
}

#[scenario(path = "tests/features/python_module.feature", index = 9)]
pub(super) fn parses_tei_xml_payloads(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 10)]
pub(super) fn rejects_invalid_tei_xml_payloads(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 11)]
pub(super) fn emits_documents_to_tei_xml(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 12)]
pub(super) fn rejects_emit_xml_with_control_characters(#[from(python_state)] _: PythonModuleState) {
}
