//! JSON helpers used across the workspace.
//!
//! The module wraps `serde_json` so downstream crates can consume a stable API
//! surface without taking a direct dependency on `serde_json`.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Alias for JSON values.
pub type Value = serde_json::Value;

/// Alias for JSON serialization/deserialization failures.
pub type JsonError = serde_json::Error;

/// Serializes a value into a compact JSON string.
///
/// # Errors
///
/// Returns [`JsonError`] when serialization fails.
pub fn to_string<T>(value: &T) -> Result<String, JsonError>
where
    T: Serialize,
{
    serde_json::to_string(value)
}

/// Deserializes a value from JSON text.
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

/// Serializes a value into a generic JSON tree.
///
/// # Errors
///
/// Returns [`JsonError`] when serialization fails.
pub fn to_value<T>(value: &T) -> Result<Value, JsonError>
where
    T: Serialize,
{
    serde_json::to_value(value)
}

/// Deserializes a strongly typed value from a generic JSON tree.
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
    //! Unit tests for JSON encoding and decoding helpers.

    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn round_trips_values() {
        #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
        struct Example {
            title: String,
            count: u32,
        }

        let example = Example {
            title: "Wolf 359".to_owned(),
            count: 42,
        };
        let payload = to_value(&example).expect("serializing to JSON value should succeed");
        let decoded: Example =
            from_value(payload).expect("deserializing JSON value should succeed");

        assert_eq!(decoded, example);
    }

    #[test]
    fn rejects_invalid_json_payloads() {
        let error = from_str::<Value>("this is not JSON").expect_err("invalid JSON should fail");
        assert!(error.is_syntax(), "expected syntax error, got: {error}");
    }
}
