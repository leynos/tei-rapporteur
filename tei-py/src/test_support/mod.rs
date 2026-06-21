//! Test-only helpers shared across Rust unit tests and Python BDD suites.
//!
//! They use `PyO3`'s embedding API with the supported `PyO3` `0.28.x`
//! minor series to interact with an embedded Python interpreter. Their
//! primary job is bootstrapping `msgspec>=0.19,<0.20` with `uv` or `pip`
//! so Rust and Python BDD tests can import it.

mod bootstrap;

pub use bootstrap::{RunWithKwargsArgs, ensure_msgspec_installed, msgspec_available, run_with_kwargs};

#[cfg(test)]
mod tests;
