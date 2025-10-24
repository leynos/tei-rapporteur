#![doc = "Core types for the Chutoro vector index."]

//! The `chutoro-core` crate implements a CPU-based Hierarchical Navigable Small
//! World (HNSW) index. It exposes a [`CpuHnsw`] type for insertion and search
//! alongside supporting error and parameter structures.

mod datasource;
pub mod hnsw;

pub use datasource::{DataSource, DataSourceError};
pub use hnsw::{CpuHnsw, HnswError, HnswParams};
