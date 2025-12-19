//! `MessagePack` helpers used across the workspace.
//!
//! The module wraps `rmp-serde` so downstream crates can consume a stable API
//! surface without taking a direct dependency on `rmp-serde`.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Alias for `MessagePack` deserialisation failures.
pub type MsgpackDecodeError = rmp_serde::decode::Error;

/// Alias for `MessagePack` serialisation failures.
pub type MsgpackEncodeError = rmp_serde::encode::Error;

/// Serialises a value to `MessagePack` bytes using named fields.
///
/// # Errors
///
/// Returns [`MsgpackEncodeError`] when serialisation fails.
pub fn to_vec_named<T>(value: &T) -> Result<Vec<u8>, MsgpackEncodeError>
where
    T: Serialize,
{
    rmp_serde::to_vec_named(value)
}

/// Deserialises a value from `MessagePack` bytes.
///
/// # Errors
///
/// Returns [`MsgpackDecodeError`] when deserialisation fails.
pub fn from_slice<T>(bytes: &[u8]) -> Result<T, MsgpackDecodeError>
where
    T: DeserializeOwned,
{
    rmp_serde::from_slice(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tei_core::TeiDocument;

    #[test]
    fn round_trips_tei_document_via_messagepack() {
        let document = TeiDocument::from_title_str("Wolf 359")
            .expect("fixtures should construct a valid document");

        let payload = to_vec_named(&document).expect("serialising to MessagePack should succeed");
        let decoded: TeiDocument =
            from_slice(&payload).expect("deserialising MessagePack payload should succeed");

        assert_eq!(decoded.title().as_str(), "Wolf 359");
    }

    #[test]
    fn rejects_empty_payloads() {
        let result: Result<TeiDocument, _> = from_slice(&[]);

        let error = result.expect_err("empty payloads must not deserialise");
        assert!(matches!(error, MsgpackDecodeError::InvalidMarkerRead(_)));
    }
}
