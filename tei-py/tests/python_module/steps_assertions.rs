//! Assertion helpers used by the behaviour-driven scenarios.

use super::state::{PythonModuleState, python_state};
use anyhow::{Context, Result, ensure};
use pyo3::{Python, types::PyAnyMethods};
use rstest_bdd_macros::then;
use tei_core::TeiDocument;
use tei_xml::emit_xml as emit_document_xml;

const _: fn() -> PythonModuleState = python_state;

fn assert_document_title(state: &PythonModuleState, expected: &str) -> Result<()> {
    Python::with_gil(|py| {
        state.with_document(py, |document| {
            let title: String = document.getattr("title")?.extract()?;
            ensure!(
                title == expected,
                "expected document title {expected:?}, found {title:?}"
            );
            Ok::<_, anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[then("the document title equals \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
pub(super) fn the_document_title_equals(
    #[from(python_state)] state: &PythonModuleState,
    expected: String,
) -> Result<()> {
    assert_document_title(state, expected.as_str())
}

#[then("construction fails mentioning \"{snippet}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
pub(super) fn construction_fails_mentioning(
    #[from(python_state)] state: &PythonModuleState,
    snippet: String,
) -> Result<()> {
    let message = state.error()?;
    ensure!(
        message.contains(&snippet),
        "error should mention {snippet:?}, found {message:?}"
    );
    Ok(())
}

#[then("the markup equals \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
pub(super) fn the_markup_equals(
    #[from(python_state)] state: &PythonModuleState,
    expected: String,
) -> Result<()> {
    let markup = state.markup()?;
    ensure!(
        markup == expected,
        "expected markup {expected:?}, found {markup:?}"
    );
    Ok(())
}

#[then("decoding the MessagePack payload yields a Document titled \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
pub(super) fn decoding_the_messagepack_payload_yields_document(
    #[from(python_state)] state: &PythonModuleState,
    expected: String,
) -> Result<()> {
    assert_document_title(state, expected.as_str())
}

#[then("the TEI XML output equals the canonical payload for \"{title}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
pub(super) fn the_tei_xml_output_equals_the_canonical_payload(
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
