use anyhow::{Context, Result, ensure};
use pyo3::{
    Bound,
    prelude::*,
    types::{PyAnyMethods, PyModule},
};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_py::tei_rapporteur;

// Keep feature files and steps aligned with the compiled binary.
pub(super) const _: &str = include_str!("../features/python_module.feature");

pub(super) struct PayloadSlot<T> {
    slot: RefCell<Option<T>>,
    description: &'static str,
}

impl<T> PayloadSlot<T> {
    pub(super) const fn new(description: &'static str) -> Self {
        Self {
            slot: RefCell::new(None),
            description,
        }
    }

    pub(super) fn store(&self, value: T) {
        *self.slot.borrow_mut() = Some(value);
    }

    pub(super) fn clear(&self) {
        self.slot.borrow_mut().take();
    }
}

impl<T> PayloadSlot<T>
where
    T: Clone,
{
    pub(super) fn load(&self) -> Result<T> {
        self.slot
            .borrow()
            .as_ref()
            .cloned()
            .with_context(|| format!("{} must be prepared before use", self.description))
    }
}

/// Holds optional Python objects for behaviour-driven tests.
pub(super) struct PythonModuleState {
    pub(super) module: RefCell<Option<Py<PyModule>>>,
    pub(super) document: RefCell<Option<Py<PyAny>>>,
    pub(super) markup: RefCell<Option<String>>,
    pub(super) error: RefCell<Option<String>>,
    pub(super) msgpack_payload: PayloadSlot<Vec<u8>>,
    pub(super) xml_payload: PayloadSlot<String>,
    pub(super) xml_output: PayloadSlot<String>,
}

impl Default for PythonModuleState {
    fn default() -> Self {
        Self {
            module: RefCell::new(None),
            document: RefCell::new(None),
            markup: RefCell::new(None),
            error: RefCell::new(None),
            msgpack_payload: PayloadSlot::new("MessagePack payload"),
            xml_payload: PayloadSlot::new("XML payload"),
            xml_output: PayloadSlot::new("XML output"),
        }
    }
}

impl PythonModuleState {
    pub(super) fn set_module(&self, module: Py<PyModule>) {
        *self.module.borrow_mut() = Some(module);
    }

    pub(super) fn with_module<'py, T>(
        &self,
        py: Python<'py>,
        op: impl FnOnce(Bound<'py, PyModule>) -> Result<T>,
    ) -> Result<T> {
        let module = {
            let guard = self.module.borrow();
            guard
                .as_ref()
                .map(|module| module.clone_ref(py))
                .context("module must be initialised before use")?
        };
        op(module.into_bound(py))
    }

    pub(super) fn store_document(&self, document: Py<PyAny>) {
        *self.document.borrow_mut() = Some(document);
        self.markup.borrow_mut().take();
        self.error.borrow_mut().take();
        self.xml_output.clear();
    }

    pub(super) fn with_document<'py, T>(
        &self,
        py: Python<'py>,
        op: impl FnOnce(Bound<'py, PyAny>) -> Result<T>,
    ) -> Result<T> {
        let document = {
            let guard = self.document.borrow();
            guard
                .as_ref()
                .map(|document| document.clone_ref(py))
                .context("document must be constructed before assertions")?
        };
        op(document.into_bound(py))
    }

    pub(super) fn store_markup(&self, value: String) {
        *self.markup.borrow_mut() = Some(value);
        self.error.borrow_mut().take();
        self.document.borrow_mut().take();
    }

    pub(super) fn markup(&self) -> Result<String> {
        self.markup
            .borrow()
            .as_ref()
            .cloned()
            .context("markup must be generated before asserting on it")
    }

    pub(super) fn store_error(&self, message: String) {
        self.error.borrow_mut().replace(message);
        self.document.borrow_mut().take();
        self.markup.borrow_mut().take();
        self.xml_output.clear();
    }

    pub(super) fn error(&self) -> Result<String> {
        self.error
            .borrow()
            .as_ref()
            .cloned()
            .context("expected an error but none was recorded")
    }

    pub(super) fn store_msgpack_payload(&self, payload: Vec<u8>) {
        self.msgpack_payload.store(payload);
    }

    pub(super) fn msgpack_payload(&self) -> Result<Vec<u8>> {
        self.msgpack_payload.load()
    }

    pub(super) fn store_xml_payload(&self, payload: String) {
        self.xml_payload.store(payload);
        self.xml_output.clear();
    }

    pub(super) fn xml_payload(&self) -> Result<String> {
        self.xml_payload.load()
    }

    pub(super) fn store_xml_output(&self, payload: String) {
        self.xml_output.store(payload);
        self.error.borrow_mut().take();
    }

    pub(super) fn xml_output(&self) -> Result<String> {
        self.xml_output.load()
    }
}

#[fixture]
pub(super) fn python_state() -> PythonModuleState {
    PythonModuleState::default()
}

pub(super) fn construct_python_document(state: &PythonModuleState, title: &str) -> Result<()> {
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
pub(super) fn module_is_initialised(#[from(python_state)] state: &PythonModuleState) -> Result<()> {
    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "tei_rapporteur")?;
        tei_rapporteur(py, &module)?;
        state.set_module(module.unbind());
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
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
    construct_python_document(state, r#"Special <Title> & "Quotes" and 'Apostrophes'"#)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
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

#[then("the document title equals \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders own their `String` values"
)]
pub(super) fn the_document_title_equals(
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
            Ok::<_, anyhow::Error>(())
        })
    })?;
    Ok(())
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

#[scenario(path = "tests/features/python_module.feature", index = 0)]
pub(super) fn constructs_a_document(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 1)]
pub(super) fn rejects_blank_titles(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 2)]
pub(super) fn emits_title_markup(#[from(python_state)] _: PythonModuleState) {}

#[scenario(path = "tests/features/python_module.feature", index = 3)]
pub(super) fn document_markup_escapes_special_characters(
    #[from(python_state)] _: PythonModuleState,
) {
}
