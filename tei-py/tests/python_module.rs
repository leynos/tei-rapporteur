//! Behaviour-driven tests for the `tei_rapporteur` Python module.
#![expect(
    clippy::self_named_module_files,
    reason = "The Gherkin .feature assets intentionally reuse the module name"
)]

const _: &str = include_str!("features/python_module.feature");

#[path = "python_module/state.rs"]
mod state;
#[path = "python_module/steps_assertions.rs"]
mod steps_assertions;
#[path = "python_module/steps_construction.rs"]
mod steps_construction;
#[path = "python_module/steps_dict.rs"]
mod steps_dict;
#[path = "python_module/steps_msgpack.rs"]
mod steps_msgpack;
#[path = "python_module/steps_xml.rs"]
mod steps_xml;
#[path = "python_module/test_utils/mod.rs"]
mod test_utils;
