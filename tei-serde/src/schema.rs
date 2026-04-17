//! JSON Schema generation for the TEI Rapporteur data model.
//!
//! The schema is generated from the canonical Rust structs in `tei-core`, so
//! downstream consumers can validate persisted JSON payloads (for example
//! stored in a document database) without reverse-engineering the Serde
//! layout.

use schemars::Schema;
use serde_json::{Map, Value};
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
    apply_profile_constraints(object);
    schema
}

fn apply_profile_constraints(schema: &mut Map<String, Value>) {
    let Some(definitions) = schema.get_mut("$defs").and_then(Value::as_object_mut) else {
        return;
    };

    apply_refs_decl_constraints(definitions);
    apply_citation_constraints(definitions);
    apply_stand_off_constraints(definitions);
    apply_div_type_constraints(definitions);
    apply_div_subtype_constraints(definitions);
    apply_head_constraints(definitions);
}

fn apply_refs_decl_constraints(definitions: &mut Map<String, Value>) {
    let Some(refs_decl) = definitions
        .get_mut("refsDecl")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    refs_decl.insert("minProperties".to_owned(), Value::from(1));
    refs_decl.insert(
        "required".to_owned(),
        Value::Array(vec![Value::String("citeStructure".to_owned())]),
    );

    let Some(properties) = refs_decl
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let Some(cite_structure) = properties
        .get_mut("citeStructure")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    cite_structure.insert("minItems".to_owned(), Value::from(1));
}

fn apply_citation_constraints(definitions: &mut Map<String, Value>) {
    let Some(cite_structure) = definitions
        .get_mut("citeStructure")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    if let Some(properties) = cite_structure
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        set_min_length(properties, "@match", 1);
        set_min_length(properties, "@unit", 1);
        set_min_length(properties, "@use", 1);
        set_min_length(properties, "@delim", 1);
        set_min_items(properties, "citeStructure", 1);
    }

    let Some(cite_data) = definitions
        .get_mut("citeData")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    if let Some(properties) = cite_data
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        set_min_length(properties, "@property", 1);
        set_min_length(properties, "@use", 1);
    }
}

fn apply_stand_off_constraints(definitions: &mut Map<String, Value>) {
    let Some(span) = definitions.get_mut("span").and_then(Value::as_object_mut) else {
        return;
    };
    span.insert(
        "anyOf".to_owned(),
        Value::Array(vec![
            required_fields(["@target"]),
            required_fields(["@from"]),
        ]),
    );
    span.insert(
        "allOf".to_owned(),
        Value::Array(vec![if_then_required("@to", ["@from"])]),
    );

    let Some(span_group) = definitions
        .get_mut("spanGrp")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let Some(properties) = span_group
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let Some(kind) = properties.get_mut("@type").and_then(Value::as_object_mut) else {
        return;
    };
    kind.insert("pattern".to_owned(), Value::String("\\S".to_owned()));
}

fn apply_div_type_constraints(definitions: &mut Map<String, Value>) {
    apply_non_empty_string_definition(definitions, "DivType");
}

fn apply_div_subtype_constraints(definitions: &mut Map<String, Value>) {
    apply_non_empty_string_definition(definitions, "DivSubtype");
}

fn apply_head_constraints(definitions: &mut Map<String, Value>) {
    let Some(head) = definitions.get_mut("head").and_then(Value::as_object_mut) else {
        return;
    };
    head.insert(
        "required".to_owned(),
        Value::Array(vec![Value::String("$value".to_owned())]),
    );

    let Some(properties) = head.get_mut("properties").and_then(Value::as_object_mut) else {
        return;
    };
    if let Some(value_schema) = properties.get_mut("$value").and_then(Value::as_object_mut) {
        value_schema.remove("default");
    }
    set_min_items(properties, "$value", 1);
}

fn apply_non_empty_string_definition(definitions: &mut Map<String, Value>, name: &str) {
    let Some(schema) = definitions.get_mut(name).and_then(Value::as_object_mut) else {
        return;
    };
    // Enforce non-empty, non-whitespace validation matching the Rust newtypes.
    schema.insert("minLength".to_owned(), Value::from(1));
    schema.insert("pattern".to_owned(), Value::String("\\S".to_owned()));
}

fn set_min_length(properties: &mut Map<String, Value>, name: &str, min_length: u64) {
    let Some(schema) = properties.get_mut(name).and_then(Value::as_object_mut) else {
        return;
    };
    schema.insert("minLength".to_owned(), Value::from(min_length));
}

fn set_min_items(properties: &mut Map<String, Value>, name: &str, min_items: u64) {
    let Some(schema) = properties.get_mut(name).and_then(Value::as_object_mut) else {
        return;
    };
    schema.insert("minItems".to_owned(), Value::from(min_items));
}

fn required_fields<const N: usize>(fields: [&str; N]) -> Value {
    let mut object = Map::new();
    object.insert(
        "required".to_owned(),
        Value::Array(
            fields
                .into_iter()
                .map(|field| Value::String(field.to_owned()))
                .collect(),
        ),
    );
    let mut properties = Map::new();
    for field in fields {
        let mut property = Map::new();
        property.insert("type".to_owned(), Value::String("string".to_owned()));
        property.insert("minLength".to_owned(), Value::from(1));
        properties.insert(field.to_owned(), Value::Object(property));
    }
    object.insert("properties".to_owned(), Value::Object(properties));
    Value::Object(object)
}

fn if_then_required<const N: usize>(present_field: &str, required_fields: [&str; N]) -> Value {
    let mut condition = Map::new();
    condition.insert(
        "required".to_owned(),
        Value::Array(vec![Value::String(present_field.to_owned())]),
    );

    let mut consequence = Map::new();
    consequence.insert(
        "required".to_owned(),
        Value::Array(
            required_fields
                .into_iter()
                .map(|field| Value::String(field.to_owned()))
                .collect(),
        ),
    );
    let mut properties = Map::new();
    for field in required_fields {
        let mut property = Map::new();
        property.insert("type".to_owned(), Value::String("string".to_owned()));
        property.insert("minLength".to_owned(), Value::from(1));
        properties.insert(field.to_owned(), Value::Object(property));
    }
    consequence.insert("properties".to_owned(), Value::Object(properties));

    let mut rule = Map::new();
    rule.insert("if".to_owned(), Value::Object(condition));
    rule.insert("then".to_owned(), Value::Object(consequence));
    Value::Object(rule)
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
