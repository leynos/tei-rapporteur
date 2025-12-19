//! `MessagePack` helpers used across the workspace.
//!
//! The module wraps `rmp-serde` so downstream crates can consume a stable API
//! surface without taking a direct dependency on `rmp-serde`.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Alias for `MessagePack` deserialization failures.
pub type MsgpackDecodeError = rmp_serde::decode::Error;

/// Alias for `MessagePack` serialization failures.
pub type MsgpackEncodeError = rmp_serde::encode::Error;

/// Serializes a value to `MessagePack` bytes using named fields.
///
/// # Errors
///
/// Returns [`MsgpackEncodeError`] when serialization fails.
pub fn to_vec_named<T>(value: &T) -> Result<Vec<u8>, MsgpackEncodeError>
where
    T: Serialize,
{
    rmp_serde::to_vec_named(value)
}

/// Deserializes a value from `MessagePack` bytes.
///
/// # Errors
///
/// Returns [`MsgpackDecodeError`] when deserialization fails.
pub fn from_slice<T>(bytes: &[u8]) -> Result<T, MsgpackDecodeError>
where
    T: DeserializeOwned,
{
    rmp_serde::from_slice(bytes)
}

#[cfg(test)]
mod tests {
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

        let payload = to_vec_named(&example).expect("serializing to `MessagePack` should succeed");
        let decoded: Example =
            from_slice(&payload).expect("deserializing `MessagePack` payload should succeed");

        assert_eq!(decoded, example);
    }

    #[test]
    fn rejects_empty_payloads() {
        let result: Result<u32, _> = from_slice(&[]);

        let error = result.expect_err("empty payloads must not deserialize");
        assert!(matches!(error, MsgpackDecodeError::InvalidMarkerRead(_)));
    }
}
