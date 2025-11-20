//! Behaviour-driven tests for the `tei_rapporteur` Python module.
#![expect(
    clippy::self_named_module_files,
    reason = "The Gherkin .feature assets intentionally reuse the module name"
)]
#[path = "python_module/core.rs"]
mod core;
#[path = "python_module/shared.rs"]
mod shared;
#[path = "python_module/xml.rs"]
mod xml;
