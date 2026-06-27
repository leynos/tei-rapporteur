//! Steps covering the `msgspec.Struct` projections exposed to Python.

use super::state::{PythonModuleState, python_state};
use anyhow::{Context, Result};
use pyo3::{Bound, prelude::*, types::PyDict};
use rstest_bdd_macros::{scenario, when};
use tei_py::test_support::{bootstrap_msgspec, with_python};

pub(super) fn decode_episode<'py>(
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

    anyhow::ensure!(
        bootstrap_msgspec(),
        "msgspec bootstrap should succeed for Episode retitle tests"
    );

    with_python(|py| {
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

    anyhow::ensure!(
        bootstrap_msgspec(),
        "msgspec bootstrap should succeed for Episode decoding tests"
    );

    with_python(|py| {
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

/// Scenario: Round-trip `MessagePack` via the Episode struct.
#[scenario(
    path = "tests/features/python_module.feature",
    name = "Round-trip MessagePack via the Episode struct"
)]
#[expect(
    unused_variables,
    reason = "rstest-bdd injects the state fixture into generated step calls"
)]
pub fn round_trips_via_episode_struct(python_state: PythonModuleState) {}

/// Scenario: Report msgspec errors for malformed payloads.
#[scenario(
    path = "tests/features/python_module.feature",
    name = "Report msgspec errors for malformed payloads"
)]
#[expect(
    unused_variables,
    reason = "rstest-bdd injects the state fixture into generated step calls"
)]
pub fn episode_decoding_reports_errors(python_state: PythonModuleState) {}
