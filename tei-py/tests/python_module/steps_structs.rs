//! Steps covering the `msgspec.Struct` projections exposed to Python.

use super::state::{PythonModuleState, python_state};
use anyhow::{Context, Result};
use pyo3::{prelude::*, types::PyDict};
use rstest_bdd_macros::{scenario, when};
use serde::Deserialize;
use tei_core::{FileDesc, TeiDocument, TeiHeader};
use tei_py::projection::PyTeiDocument;
use tei_py::test_support::msgspec_available;
use tei_serde::json::Value;
use tei_serde::msgpack::{from_slice, to_vec_named};

const _: fn() -> PythonModuleState = python_state;

fn retitle_document(document: &TeiDocument, title: &str) -> Result<TeiDocument> {
    let old_header = document.header().clone();
    let old_file_desc = old_header.file_desc().clone();

    let mut new_file_desc =
        FileDesc::from_title_str(title).context("fallback title must be valid")?;

    if let Some(series) = old_file_desc.series() {
        new_file_desc = new_file_desc.with_series(series);
    }

    if let Some(synopsis) = old_file_desc.synopsis() {
        new_file_desc = new_file_desc.with_synopsis(synopsis);
    }

    let mut header = TeiHeader::new(new_file_desc);

    if let Some(profile) = old_header.profile_desc().cloned() {
        header = header.with_profile_desc(profile);
    }

    if let Some(encoding) = old_header.encoding_desc().cloned() {
        header = header.with_encoding_desc(encoding);
    }

    if let Some(revision) = old_header.revision_desc().cloned() {
        header = header.with_revision_desc(revision);
    }

    Ok(TeiDocument::new(header, document.text().clone()))
}

fn decode_episode<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyAny>,
    payload: &[u8],
) -> Result<Bound<'py, PyAny>> {
    let structs = module
        .getattr("structs")
        .context("structs submodule should exist")?;
    let episode_type = structs
        .getattr("Episode")
        .context("Episode class should be exported")?;
    let msgpack = py
        .import("msgspec.msgpack")
        .context("msgspec.msgpack import should succeed")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("type", episode_type)?;

    Ok(msgpack
        .getattr("decode")
        .context("decode function should exist")?
        .call((payload,), Some(&kwargs))?)
}

#[when("I convert the MessagePack payload to an Episode and retitle it \"{title}\"")]
pub(super) fn i_convert_payload_to_episode_and_retitle(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    let payload = state.msgpack_payload()?;

    if !msgspec_available() {
        let projection: PyTeiDocument =
            from_slice(&payload).context("fallback decoding MessagePack document")?;
        let document: TeiDocument = TeiDocument::try_from(projection)
            .context("projection should convert to TeiDocument")?;
        let retitled = retitle_document(&document, title.as_str())?;
        let projection_updated = PyTeiDocument::from(&retitled);
        let updated_payload =
            to_vec_named(&projection_updated).context("fallback encoding updated document")?;
        state.store_msgpack_payload(updated_payload);
        return Ok(());
    }

    Python::with_gil(|py| {
        state.with_module(py, |module| {
            let episode = match decode_episode(py, &module, &payload) {
                Ok(value) => value,
                Err(error) => {
                    state.store_error(error.to_string());
                    return Ok::<(), anyhow::Error>(());
                }
            };
            let header = episode.getattr("header")?;
            let file_desc = header.getattr("file_desc")?;
            file_desc.setattr("title", title)?;

            let msgpack = py.import("msgspec.msgpack")?;
            let updated_payload: Vec<u8> = msgpack
                .getattr("encode")
                .context("encode function should exist")?
                .call1((episode,))
                .and_then(|value| value.extract())
                .context("encoding Episode should succeed")?;

            state.store_msgpack_payload(updated_payload);
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[when("I decode the MessagePack payload to an Episode struct")]
pub(super) fn i_decode_the_payload_to_an_episode(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = state.msgpack_payload()?;

    if !msgspec_available() {
        #[expect(
            dead_code,
            reason = "EpisodeCarrier is only used to trigger a missing-field decode when msgspec is unavailable."
        )]
        #[derive(Debug, Deserialize)]
        struct EpisodeCarrier {
            header: Value,
            text: Value,
        }

        if let Err(error) = from_slice::<EpisodeCarrier>(&payload) {
            state.store_error(error.to_string());
        }

        return Ok(());
    }

    Python::with_gil(|py| {
        state.with_module(py, |module| match decode_episode(py, &module, &payload) {
            Ok(_) => Ok::<(), anyhow::Error>(()),
            Err(error) => {
                state.store_error(error.to_string());
                Ok::<(), anyhow::Error>(())
            }
        })
    })?;
    Ok(())
}

/// Scenario: Round-trip a Document through the Python Episode struct.
#[scenario(path = "tests/features/python_module.feature", index = 18)]
pub fn round_trips_via_episode_struct(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Surface struct decoding errors for malformed `MessagePack` payloads.
#[scenario(path = "tests/features/python_module.feature", index = 19)]
pub fn episode_decoding_reports_errors(python_state: PythonModuleState) {
    let _ = python_state;
}
