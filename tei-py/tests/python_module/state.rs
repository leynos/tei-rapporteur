//! Python module fixture state and helpers shared across BDD steps.

use super::test_utils::payload_slot::PayloadSlot;
use anyhow::{Context, Result};
use pyo3::{
    Bound,
    prelude::*,
    types::{PyAnyMethods, PyModule},
};
use rstest::fixture;
use std::cell::RefCell;
use tei_py::{
    tei_rapporteur,
    test_support::{bootstrap_msgspec_attached, with_python},
};
use tei_serde::json::Value;

pub(super) struct PythonModuleState {
    module: RefCell<Option<Py<PyModule>>>,
    document: RefCell<Option<Py<PyAny>>>,
    markup: RefCell<Option<String>>,
    error: RefCell<Option<String>>,
    msgpack_payload: PayloadSlot<Vec<u8>>,
    xml_payload: PayloadSlot<String>,
    xml_output: PayloadSlot<String>,
    dict_payload: PayloadSlot<Value>,
    dict_output: PayloadSlot<Value>,
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
            dict_payload: PayloadSlot::new("dictionary payload"),
            dict_output: PayloadSlot::new("dictionary output"),
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
        self.dict_output.clear();
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
        self.dict_output.clear();
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

    pub(super) fn store_dict_payload(&self, payload: Value) {
        self.dict_payload.store(payload);
    }

    pub(super) fn dict_payload(&self) -> Result<Value> {
        self.dict_payload.load()
    }

    pub(super) fn store_dict_output(&self, payload: Value) {
        self.dict_output.store(payload);
        self.error.borrow_mut().take();
    }

    pub(super) fn dict_output(&self) -> Result<Value> {
        self.dict_output.load()
    }
}

#[cfg_attr(
    not(feature = "test-support"),
    expect(
        dead_code,
        reason = "rstest-bdd resolves this fixture by parameter name"
    )
)]
#[fixture]
pub(super) fn python_state() -> PythonModuleState {
    PythonModuleState::default()
}

pub(super) fn construct_python_document(state: &PythonModuleState, title: &str) -> Result<()> {
    with_python(|py| {
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

pub(super) fn module_is_initialised(state: &PythonModuleState) -> Result<()> {
    with_python(|py| {
        // Scenarios that require typed structs assert on bootstrap outcomes in
        // their own steps; ordinary module setup keeps the dependency best-effort.
        let _msgspec_bootstrapped = bootstrap_msgspec_attached(py);
        let module = PyModule::new(py, "tei_rapporteur")?;
        tei_rapporteur(py, &module)?;
        state.set_module(module.unbind());
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}
