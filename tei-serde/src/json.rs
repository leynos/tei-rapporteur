//! JSON helpers used across the workspace.
//!
//! The module wraps `serde_json` so downstream crates can consume a stable API
//! surface without taking a direct dependency on `serde_json`.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Alias for JSON values.
pub type Value = serde_json::Value;

/// Alias for JSON serialisation/deserialisation failures.
pub type JsonError = serde_json::Error;

/// Serialises a value into a compact JSON string.
///
/// # Errors
///
/// Returns [`JsonError`] when serialisation fails.
pub fn to_string<T>(value: &T) -> Result<String, JsonError>
where
    T: Serialize,
{
    serde_json::to_string(value)
}

/// Deserialises a value from JSON text.
///
/// # Errors
///
/// Returns [`JsonError`] when the input is not valid JSON or does not match
/// the target type.
pub fn from_str<T>(source: &str) -> Result<T, JsonError>
where
    T: DeserializeOwned,
{
    serde_json::from_str(source)
}

/// Serialises a value into a generic JSON tree.
///
/// # Errors
///
/// Returns [`JsonError`] when serialisation fails.
pub fn to_value<T>(value: &T) -> Result<Value, JsonError>
where
    T: Serialize,
{
    serde_json::to_value(value)
}

/// Deserialises a strongly typed value from a generic JSON tree.
///
/// # Errors
///
/// Returns [`JsonError`] when the JSON tree does not match the target type.
pub fn from_value<T>(value: Value) -> Result<T, JsonError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tei_core::TeiDocument;

    #[test]
    fn round_trips_tei_document_via_json_value() {
        let document = TeiDocument::from_title_str("Wolf 359")
            .expect("fixtures should construct a valid document");

        let payload = to_value(&document).expect("serialising to JSON value should succeed");
        let decoded: TeiDocument =
            from_value(payload).expect("deserialising JSON value should succeed");

        assert_eq!(decoded.title().as_str(), "Wolf 359");
    }

    #[test]
    fn rejects_blank_titles_during_deserialisation() {
        let document = TeiDocument::from_title_str("placeholder")
            .expect("fixtures should construct a valid document");
        let mut payload = to_value(&document).expect("serialising fixtures to JSON value");

        if let Some(title) = payload.pointer_mut("/teiHeader/fileDesc/title") {
            *title = Value::String("   ".to_owned());
        }

        let text = to_string(&payload).expect("serialising mutated payload should succeed");
        let result: Result<TeiDocument, _> = from_str(&text);

        let error = result.expect_err("blank titles must not deserialise");
        let message = error.to_string();
        assert!(
            message.contains("document title may not be empty"),
            "expected title validation error, got: {message}"
        );
    }
}
