use super::shared::*;
use anyhow::{Context, Result, bail, ensure};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rmp_serde::{from_slice, to_vec_named};
use rstest_bdd_macros::{given, scenario, then, when};
use serde_json::json;
use tei_core::TeiDocument;

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
#[given("I encode a MessagePack document titled \"{title}\"")]
fn i_encode_a_messagepack_document(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    let document = TeiDocument::from_title_str(title.as_str())
        .context("MessagePack fixtures must construct valid documents")?;
    let payload =
        to_vec_named(&document).context("serialising fixtures to MessagePack should succeed")?;
    state.store_msgpack_payload(payload);
    Ok(())
}

#[given("I provide an invalid MessagePack payload")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "rstest-bdd step signatures stay uniform even when storing literals"
)]
fn i_provide_an_invalid_messagepack_payload(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    state.store_msgpack_payload(b"this is not valid MessagePack".to_vec());
    Ok(())
}

#[given("I encode a MessagePack document missing required fields")]
fn i_encode_a_messagepack_document_missing_required_fields(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = to_vec_named(&json!({ "text": {} }))
        .context("serialising malformed MessagePack fixture should succeed")?;
    state.store_msgpack_payload(payload);
    Ok(())
}

#[when("I decode the MessagePack payload")]
fn i_decode_the_messagepack_payload(#[from(python_state)] state: &PythonModuleState) -> Result<()> {
    let payload = state.msgpack_payload()?;
    Python::with_gil(|py| {
        state.with_module(py, |module| {
            let decoder = module
                .getattr("from_msgpack")
                .context("from_msgpack must be registered")?;
            match decoder.call1((PyBytes::new_bound(py, &payload),)) {
                Ok(document) => state.store_document(document.unbind()),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[when("I encode the constructed Document to MessagePack")]
#[expect(
    clippy::excessive_nesting,
    reason = "rstest-bdd steps need nested Python contexts to access the module and stored Document"
)]
fn i_encode_the_document_to_messagepack(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    Python::with_gil(|py| {
        state.with_module(py, |module| {
            let encoder = module
                .getattr("to_msgpack")
                .context("to_msgpack must be registered")?;
            state.with_document(py, |document| {
                match encoder.call1((document,)) {
                    Ok(payload) => {
                        let bytes: Vec<u8> = payload.extract()?;
                        state.store_msgpack_payload(bytes);
                    }
                    Err(error) => state.store_error(error.to_string()),
                }
                Ok::<(), anyhow::Error>(())
            })
        })
    })?;
    Ok(())
}

#[when("I encode MessagePack without providing a Document")]
fn i_encode_messagepack_without_a_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    Python::with_gil(|py| {
        state.with_module(py, |module| {
            let encoder = module
                .getattr("to_msgpack")
                .context("to_msgpack must be registered")?;
            match encoder.call1(("not a document",)) {
                Ok(_) => bail!("encoding without a Document should fail"),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[then("decoding the MessagePack payload yields a Document titled \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
fn decoding_the_messagepack_payload_yields_document(
    #[from(python_state)] state: &PythonModuleState,
    expected: String,
) -> Result<()> {
    let payload = state.msgpack_payload()?;
    let document = from_slice::<TeiDocument>(&payload)
        .context("decoding stored MessagePack payload should succeed")?;
    ensure!(
        document.title().as_str() == expected,
        "expected Document title {expected:?}, found {:?}",
        document.title().as_str()
    );
    Ok(())
}

#[scenario(path = "tests/features/python_module.feature", index = 4)]
pub(super) fn decodes_messagepack_documents(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 5)]
pub(super) fn rejects_invalid_messagepack_payloads(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 6)]
pub(super) fn rejects_missing_field_messagepack_payloads(
    #[from(python_state)] _: PythonModuleState,
) {
}

#[scenario(path = "tests/features/python_module.feature", index = 7)]
pub(super) fn encodes_documents_to_messagepack(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 8)]
pub(super) fn rejects_to_msgpack_without_document(#[from(python_state)] _: PythonModuleState) {}
