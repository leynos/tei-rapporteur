//! Proptest strategies for generating valid TEI documents.
//!
//! This module provides strategies that generate `TeiDocument` instances
//! respecting all validation constraints (non-empty titles, valid identifiers,
//! non-blank text segments). The generated documents can be used for
//! property-based round-trip testing between XML, JSON, and `MessagePack`.

pub mod document;
pub mod header;
pub mod inline;
pub mod primitives;
pub mod text;
