//! JSON Schema generation for the TEI Rapporteur data model.
//!
//! The schema is generated from the canonical Rust structs in `tei-core`, so
//! downstream consumers can validate persisted JSON payloads (for example
//! stored in a document database) without reverse-engineering the Serde
//! layout.

use schemars::Schema;
use serde_json::Value;
use tei_core::TeiDocument;

/// Identifier used for the published `TeiDocument` JSON Schema.
///
/// The identifier is versioned using the `tei-serde` crate version to ensure
/// callers can pin validations to a compatible schema snapshot.
#[must_use]
pub fn tei_document_schema_id() -> String {
    format!(
        "urn:tei-rapporteur:schema:tei-document:{}",
        env!("CARGO_PKG_VERSION")
    )
}

/// Generates the JSON Schema for [`TeiDocument`].
#[must_use]
pub fn tei_document_schema() -> Schema {
    let mut schema = schemars::schema_for!(TeiDocument);
    let object = schema.ensure_object();
    object.insert("$id".to_owned(), Value::String(tei_document_schema_id()));
    schema
}

/// Serializes the JSON Schema for [`TeiDocument`] as pretty-printed JSON.
///
/// # Errors
///
/// Returns [`serde_json::Error`] when serialization fails.
pub fn tei_document_schema_json_pretty() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&tei_document_schema())
}

#[cfg(test)]
mod tests {
    //! Unit tests for JSON Schema generation and snapshot consistency.

    use super::*;
    use std::fs;
    use std::io;
    use std::path::Path;

    #[test]
    fn schema_id_is_versioned() {
        let schema_id = tei_document_schema_id();
        assert!(
            schema_id.ends_with(env!("CARGO_PKG_VERSION")),
            "expected schema id to end with crate version, got {schema_id:?}"
        );
    }

    #[test]
    fn schema_exposes_document_properties() {
        let schema_json =
            tei_document_schema_json_pretty().expect("schema JSON must serialize successfully");
        let schema_value: serde_json::Value =
            serde_json::from_str(&schema_json).expect("schema JSON must be valid JSON");

        assert_eq!(
            schema_value
                .pointer("/properties/teiHeader/$ref")
                .and_then(|value| value.as_str()),
            Some("#/$defs/teiHeader")
        );
        assert_eq!(
            schema_value
                .pointer("/properties/text/$ref")
                .and_then(|value| value.as_str()),
            Some("#/$defs/text")
        );
    }

    #[test]
    fn published_schema_matches_generated_snapshot() {
        let version = env!("CARGO_PKG_VERSION");
        let schema_json =
            tei_document_schema_json_pretty().expect("schema JSON must serialize successfully");
        let expected = format!("{schema_json}\n");

        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let published_path = workspace_root
            .join("schemas")
            .join(format!("tei-document.schema.v{version}.json"));
        let published = fs::read_to_string(&published_path).unwrap_or_else(|error| match error.kind() {
            io::ErrorKind::NotFound => {
                panic!(
                    "read schema snapshot {}: {error} (snapshot missing — run `make json-schema` from the workspace root to generate it)",
                    published_path.display()
                );
            }
            _ => panic!("read schema snapshot {}: {error}", published_path.display()),
        });

        assert_eq!(published, expected, "schema snapshot is out of date");
    }
}
