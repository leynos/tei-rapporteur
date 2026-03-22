//! Integration tests for pointer and certainty wrappers.

use tei_core::{Certainty, CertaintyValidationError, Pointer, PointerList};
use tei_serde::json;

#[test]
fn pointer_list_round_trips_as_attribute_text() {
    let pointers = PointerList::new(["#u1", "https://example.test/source"])
        .unwrap_or_else(|error| panic!("pointer list should validate: {error}"));

    let serialized = json::to_string(&pointers)
        .unwrap_or_else(|error| panic!("failed to serialize PointerList to JSON: {error}"));
    assert_eq!(serialized, "\"#u1 https://example.test/source\"");

    let deserialized = json::from_str::<PointerList>(&serialized)
        .unwrap_or_else(|error| panic!("pointer list should deserialize: {error}"));
    assert_eq!(deserialized, pointers);
}

#[test]
fn borrowed_pointer_list_is_directly_iterable() {
    let pointers = PointerList::new(["#u1", "#u2"])
        .unwrap_or_else(|error| panic!("pointer list should validate: {error}"));

    let collected: Vec<&str> = (&pointers).into_iter().map(Pointer::as_str).collect();

    assert_eq!(collected, vec!["#u1", "#u2"]);
}

#[test]
fn pointer_internal_id_detects_hash_targets() {
    let internal = Pointer::new("#u1").unwrap_or_else(|error| panic!("valid pointer: {error}"));
    let external = Pointer::new("https://example.test/u1")
        .unwrap_or_else(|error| panic!("valid pointer: {error}"));

    assert_eq!(internal.internal_id(), Some("u1"));
    assert_eq!(external.internal_id(), None);
}

#[test]
fn certainty_rejects_whitespace() {
    let result = Certainty::new("very high");
    assert!(matches!(
        result,
        Err(CertaintyValidationError::ContainsWhitespace)
    ));
}
