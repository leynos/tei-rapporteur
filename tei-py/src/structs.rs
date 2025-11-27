//! Registers the Python `msgspec.Struct` projections as a submodule.
//!
//! The definitions live in `python/structs.py` so they remain readable while
//! being embedded into the extension at compile time. The module is published
//! as `tei_rapporteur.structs` when the extension loads.

use pyo3::{
    Bound, PyResult, Python,
    exceptions::PyModuleNotFoundError,
    types::{PyAnyMethods, PyModule, PyModuleMethods},
};

const STRUCTS_MODULE_NAME: &str = "tei_rapporteur.structs";
const STRUCTS_FILENAME: &str = "tei_rapporteur/structs.py";
const STRUCTS_SOURCE: &str = include_str!("../python/structs.py");

/// Adds the `tei_rapporteur.structs` module to the parent extension module.
pub fn register_structs_module(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    #[expect(
        deprecated,
        reason = "PyO3 0.23 retains from_code_bound; new API requires CStr plumbing we avoid here"
    )]
    let structs = match PyModule::from_code_bound(
        py,
        STRUCTS_SOURCE,
        STRUCTS_FILENAME,
        STRUCTS_MODULE_NAME,
    ) {
        Ok(module) => module,
        Err(error) => {
            if error.is_instance_of::<PyModuleNotFoundError>(py) {
                // msgspec missing; skip registering structs while leaving core bindings intact.
                py.run_bound(
                    concat!(
                        "import warnings; warnings.warn(",
                        "'msgspec not installed; tei_rapporteur.structs unavailable', ",
                        "RuntimeWarning)",
                    ),
                    None,
                    None,
                )?;
                return Ok(());
            }
            return Err(error);
        }
    };

    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(STRUCTS_MODULE_NAME, &structs)?;

    parent.add_submodule(&structs)?;
    parent.setattr("structs", &structs)?;

    Ok(())
}
