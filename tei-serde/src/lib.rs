//! Serialization helpers for TEI Rapporteur.
//!
//! This crate centralizes JSON, `MessagePack`, and `JSON Schema` support for
//! the workspace. The rest of the crates depend on these helpers instead of
//! pulling in `serde_json` or `rmp-serde` directly.

pub mod json;
pub mod msgpack;
pub mod schema;

pub use rmp_serde;
pub use serde_json;
