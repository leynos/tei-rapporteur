//! Behaviour-driven coverage for the `tei_rapporteur` Python module.

use anyhow::{Context, Result, bail, ensure};
use pyo3::Bound;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};
use rmp_serde::to_vec_named;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::TeiDocument;
use tei_py::tei_rapporteur;

// Keep feature files and steps aligned with the compiled binary.
const _: &str = include_str!("features/python_module.feature");

/// Holds optional Python objects for behaviour-driven tests.
///
/// `Py<PyModule>` and `Py<PyAny>` references live in `RefCell<Option<_>>` slots so
/// the step functions can mutate shared state while respecting the GIL. The
/// `markup` and `error` slots capture outputs from previous steps, and every field
/// starts as `None` until the appropriate step initialises it.
#[derive(Default)]
struct PythonModuleState {
    module: RefCell<Option<Py<PyModule>>>,
    document: RefCell<Option<Py<PyAny>>>,
    markup: RefCell<Option<String>>,
    error: RefCell<Option<String>>,
    msgpack_payload: RefCell<Option<Vec<u8>>>,
}

impl PythonModuleState {
    /// Stores the module returned by `PyModule::new_bound`, retaining ownership of
    /// the `Py<PyModule>` so later steps can bind it to whichever GIL token they
    /// currently possess.
    fn set_module(&self, module: Py<PyModule>) {
        *self.module.borrow_mut() = Some(module);
    }

    /// Borrows the stored module, clones the `Py<PyModule>`, and binds it to the
    /// supplied `Python<'py>` token before executing the provided closure.
    ///
    /// Returns an error (surfacing as a failing scenario) if the module has not yet
    /// been initialised. Typical usage binds inside `Python::with_gil`:
    ///
    /// ```
    /// state.with_module(py, |module| module.getattr("Document"))?
    /// ```
    fn with_module<'py, T>(
        &self,
        py: Python<'py>,
        op: impl FnOnce(Bound<'py, PyModule>) -> Result<T>,
    ) -> Result<T> {
        let guard = self.module.borrow();
        let Some(module) = guard.as_ref() else {
            bail!("module must be initialised before use");
        };
        let bound = module.clone_ref(py).into_bound(py);
        op(bound)
    }

    fn store_document(&self, document: Py<PyAny>) {
        *self.document.borrow_mut() = Some(document);
        self.markup.borrow_mut().take();
        self.error.borrow_mut().take();
    }

    fn with_document<'py, T>(
        &self,
        py: Python<'py>,
        op: impl FnOnce(Bound<'py, PyAny>) -> Result<T>,
    ) -> Result<T> {
        let guard = self.document.borrow();
        let Some(document) = guard.as_ref() else {
            bail!("document must be constructed before assertions");
        };
        let bound = document.clone_ref(py).into_bound(py);
        op(bound)
    }

    fn store_markup(&self, value: String) {
        *self.markup.borrow_mut() = Some(value);
        self.error.borrow_mut().take();
        self.document.borrow_mut().take();
    }

    fn markup(&self) -> Result<String> {
        self.markup
            .borrow()
            .as_ref()
            .cloned()
            .context("markup must be generated before asserting on it")
    }

    fn store_error(&self, message: String) {
        self.error.borrow_mut().replace(message);
        self.document.borrow_mut().take();
        self.markup.borrow_mut().take();
    }

    fn error(&self) -> Result<String> {
        self.error
            .borrow()
            .as_ref()
            .cloned()
            .context("expected an error but none was recorded")
    }

    fn store_msgpack_payload(&self, payload: Vec<u8>) {
        *self.msgpack_payload.borrow_mut() = Some(payload);
    }

    fn msgpack_payload(&self) -> Result<Vec<u8>> {
        self.msgpack_payload
            .borrow()
            .as_ref()
            .cloned()
            .context("MessagePack payload must be prepared before decoding")
    }
}

#[fixture]
fn python_state() -> PythonModuleState {
    PythonModuleState::default()
}

fn construct_python_document(state: &PythonModuleState, title: &str) -> Result<()> {
    Python::with_gil(|py| {
        state.with_module(py, |module| {
            let document_class = module
                .getattr("Document")
                .context("Document class should be registered")?;
            match document_class.call1((title,)) {
                Ok(document) => state.store_document(document.unbind()),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[given("the tei_rapporteur Python module is initialised")]
fn module_is_initialised(#[from(python_state)] state: &PythonModuleState) -> Result<()> {
    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "tei_rapporteur")?;
        tei_rapporteur(py, &module)?;
        state.set_module(module.unbind());
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

// rstest-bdd placeholders own their `String` values.
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
    reason = "rstest-bdd step signatures stay uniform even for infallible steps"
)]
fn i_provide_an_invalid_messagepack_payload(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    state.store_msgpack_payload(b"this is not valid MessagePack".to_vec());
    Ok(())
}

// rstest-bdd placeholders own their `String` values.
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
#[when("I construct a Document titled \"{title}\"")]
fn i_construct_a_document(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    construct_python_document(state, &title)
}

#[when("I construct a Document with the XML special characters fixture")]
fn i_construct_the_xml_special_fixture_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    construct_python_document(state, r#"Special <Title> & "Quotes" and 'Apostrophes'"#)
}

#[when("I emit title markup for \"{title}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
fn i_emit_title_markup(
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
fn i_emit_markup_from_the_document(#[from(python_state)] state: &PythonModuleState) -> Result<()> {
    let markup = Python::with_gil(|py| {
        state.with_document(py, |document| {
            let markup: String = document.call_method0("emit_title_markup")?.extract()?;
            Ok::<_, anyhow::Error>(markup)
        })
    })?;
    state.store_markup(markup);
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

#[then("the document title equals \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
fn the_document_title_equals(
    #[from(python_state)] state: &PythonModuleState,
    expected: String,
) -> Result<()> {
    Python::with_gil(|py| {
        state.with_document(py, |document| {
            let title: String = document.getattr("title")?.extract()?;
            ensure!(
                title == expected,
                "expected document title {expected:?}, found {title:?}"
            );
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[then("construction fails mentioning \"{snippet}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
fn construction_fails_mentioning(
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
fn the_markup_equals(
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

#[scenario(path = "tests/features/python_module.feature", index = 0)]
fn constructs_a_document(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 1)]
fn rejects_blank_titles(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 2)]
fn emits_title_markup(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 3)]
fn document_markup_escapes_special_characters(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 4)]
fn decodes_messagepack_documents(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 5)]
fn rejects_invalid_messagepack_payloads(#[from(python_state)] _: PythonModuleState) {}
