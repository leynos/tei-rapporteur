//! Property-based tests for round-trip integrity between formats.
//!
//! These tests verify that:
//! 1. Rust struct -> JSON -> Rust struct produces equal values
//! 2. Rust struct -> `MessagePack` -> Rust struct produces equal values
//! 3. Rust struct -> XML -> Rust struct produces semantically equal values
//!    (XML normalizes whitespace, so byte-for-byte equality is not expected)

mod arbitrary;

use proptest::prelude::*;
use tei_core::TeiDocument;

use arbitrary::document::{valid_tei_document_strategy, valid_text_only_tei_document_strategy};

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        max_shrink_iters: 1000,
        ..ProptestConfig::default()
    })]

    /// JSON round-trip: struct -> JSON string -> struct should be equal.
    #[test]
    fn json_round_trip_preserves_equality(doc in valid_tei_document_strategy()) {
        let json_string = tei_serde::json::to_string(&doc)
            .unwrap_or_else(|error| panic!("JSON serialization should succeed: {error}"));

        let decoded: TeiDocument = tei_serde::json::from_str(&json_string)
            .unwrap_or_else(|error| panic!("JSON deserialization should succeed: {error}"));

        prop_assert_eq!(doc, decoded, "JSON round-trip should preserve equality");
    }

    /// MessagePack round-trip: struct -> bytes -> struct should be equal.
    #[test]
    fn msgpack_round_trip_preserves_equality(doc in valid_tei_document_strategy()) {
        let bytes = tei_serde::msgpack::to_vec_named(&doc)
            .unwrap_or_else(|error| panic!("MessagePack serialization should succeed: {error}"));

        let decoded: TeiDocument = tei_serde::msgpack::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("MessagePack deserialization should succeed: {error}"));

        prop_assert_eq!(doc, decoded, "MessagePack round-trip should preserve equality");
    }

    /// XML round-trip: struct -> XML -> struct should be semantically equal.
    ///
    /// Note: This uses text-only documents because `quick-xml`'s serde integration
    /// does not support serializing Hi and Pause structs in `$value` fields.
    #[test]
    fn xml_round_trip_preserves_semantic_equality(doc in valid_text_only_tei_document_strategy()) {
        let xml_string = tei_xml::emit_xml(&doc)
            .unwrap_or_else(|error| panic!("XML emission should succeed: {error}"));

        let decoded = tei_xml::parse_xml(&xml_string)
            .unwrap_or_else(|error| panic!("XML parsing should succeed: {error}"));

        prop_assert_eq!(doc, decoded, "XML round-trip should preserve equality");
    }

    /// Cross-format consistency: JSON and MessagePack should decode to equal values.
    #[test]
    fn json_and_msgpack_produce_equal_results(doc in valid_tei_document_strategy()) {
        let json_string = tei_serde::json::to_string(&doc)
            .unwrap_or_else(|error| panic!("JSON serialization should succeed: {error}"));
        let json_decoded: TeiDocument = tei_serde::json::from_str(&json_string)
            .unwrap_or_else(|error| panic!("JSON deserialization should succeed: {error}"));

        let msgpack_bytes = tei_serde::msgpack::to_vec_named(&doc)
            .unwrap_or_else(|error| panic!("MessagePack serialization should succeed: {error}"));
        let msgpack_decoded: TeiDocument = tei_serde::msgpack::from_slice(&msgpack_bytes)
            .unwrap_or_else(|error| panic!("MessagePack deserialization should succeed: {error}"));

        prop_assert_eq!(
            json_decoded, msgpack_decoded,
            "JSON and MessagePack should decode to equal values"
        );
    }
}
