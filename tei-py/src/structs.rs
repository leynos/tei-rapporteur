//! Registers the Python `msgspec.Struct` projections as a submodule.
//!
//! The public façade lives in `python/structs.py`, with supporting body and
//! event definitions split into embedded helper modules so the Python sources
//! stay readable while being compiled into the extension. The public module is
//! published as `tei_rapporteur.structs` when the extension loads.

use pyo3::{
    Bound, PyResult, Python,
    exceptions::{PyModuleNotFoundError, PyRuntimeWarning, PyValueError},
    types::{PyAnyMethods, PyModule, PyModuleMethods},
};
use std::ffi::CString;

const STRUCTS_MODULE_NAME: &str = "tei_rapporteur.structs";
const STRUCTS_FILENAME: &str = "tei_rapporteur/structs.py";
const STRUCTS_SOURCE: &str = include_str!("../python/structs.py");
const STRUCTS_COMMON_MODULE_NAME: &str = "_tei_rapporteur_structs_common";
const STRUCTS_COMMON_FILENAME: &str = "tei_rapporteur/_structs_common.py";
const STRUCTS_COMMON_SOURCE: &str = include_str!("../python/structs_common.py");
const STRUCTS_BODY_MODULE_NAME: &str = "_tei_rapporteur_structs_body";
const STRUCTS_BODY_FILENAME: &str = "tei_rapporteur/_structs_body.py";
const STRUCTS_BODY_SOURCE: &str = include_str!("../python/structs_body.py");
const STRUCTS_EVENTS_MODULE_NAME: &str = "_tei_rapporteur_structs_events";
const STRUCTS_EVENTS_FILENAME: &str = "tei_rapporteur/_structs_events.py";
const STRUCTS_EVENTS_SOURCE: &str = include_str!("../python/structs_events.py");

/// Adds the `tei_rapporteur.structs` module to the parent extension module.
pub fn register_structs_module(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;

    let common = match module_from_source(
        py,
        STRUCTS_COMMON_SOURCE,
        STRUCTS_COMMON_FILENAME,
        STRUCTS_COMMON_MODULE_NAME,
    ) {
        Ok(module) => module,
        Err(error) => return handle_structs_import_error(py, error),
    };
    modules.set_item(STRUCTS_COMMON_MODULE_NAME, &common)?;

    let body = match module_from_source(
        py,
        STRUCTS_BODY_SOURCE,
        STRUCTS_BODY_FILENAME,
        STRUCTS_BODY_MODULE_NAME,
    ) {
        Ok(module) => module,
        Err(error) => return handle_structs_import_error(py, error),
    };
    modules.set_item(STRUCTS_BODY_MODULE_NAME, &body)?;

    let events = match module_from_source(
        py,
        STRUCTS_EVENTS_SOURCE,
        STRUCTS_EVENTS_FILENAME,
        STRUCTS_EVENTS_MODULE_NAME,
    ) {
        Ok(module) => module,
        Err(error) => return handle_structs_import_error(py, error),
    };
    modules.set_item(STRUCTS_EVENTS_MODULE_NAME, &events)?;

    let structs =
        match module_from_source(py, STRUCTS_SOURCE, STRUCTS_FILENAME, STRUCTS_MODULE_NAME) {
            Ok(module) => module,
            Err(error) => return handle_structs_import_error(py, error),
        };
    modules.set_item(STRUCTS_MODULE_NAME, &structs)?;

    parent.add_submodule(&structs)?;
    parent.setattr("structs", &structs)?;

    Ok(())
}

fn module_from_source<'py>(
    py: Python<'py>,
    source: &str,
    filename: &str,
    module_name: &str,
) -> PyResult<Bound<'py, PyModule>> {
    let source_cstr = cstring(source, "source")?;
    let filename_cstr = cstring(filename, "filename")?;
    let module_name_cstr = cstring(module_name, "module name")?;
    PyModule::from_code(py, &source_cstr, &filename_cstr, &module_name_cstr)
}

fn cstring(value: &str, label: &str) -> PyResult<CString> {
    CString::new(value)
        .map_err(|error| PyValueError::new_err(format!("embedded NUL byte in {label}: {error}")))
}

fn handle_structs_import_error(py: Python<'_>, error: pyo3::PyErr) -> PyResult<()> {
    if error.is_instance_of::<PyModuleNotFoundError>(py) {
        let missing_module = error
            .value(py)
            .getattr("name")
            .ok()
            .and_then(|name| name.extract::<String>().ok());

        if missing_module.as_deref() == Some("msgspec") {
            // msgspec missing; skip registering structs while leaving core bindings intact.
            let warnings = py.import("warnings")?;
            warnings.call_method1(
                "warn",
                (
                    "msgspec not installed; tei_rapporteur.structs unavailable",
                    py.get_type::<PyRuntimeWarning>(),
                ),
            )?;
            Ok(())
        } else {
            Err(error)
        }
    } else {
        Err(error)
    }
}
