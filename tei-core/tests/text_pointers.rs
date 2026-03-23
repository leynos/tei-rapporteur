//! Integration tests for pointer and certainty wrappers.

use rstest::{fixture, rstest};
use tei_core::{Certainty, CertaintyValidationError, Pointer, PointerList};
use tei_serde::json;

#[fixture]
fn pointer_list_attr() -> PointerList {
    PointerList::new(["#u1", "https://example.test/source"])
        .unwrap_or_else(|error| panic!("pointer list should validate: {error}"))
}

#[fixture]
fn pointer_list_internal_refs() -> PointerList {
    PointerList::new(["#u1", "#u2"])
        .unwrap_or_else(|error| panic!("pointer list should validate: {error}"))
}

#[rstest]
fn pointer_list_round_trips_as_attribute_text(pointer_list_attr: PointerList) {
    let serialized =
        json::to_string(&pointer_list_attr).expect("failed to serialize PointerList to JSON");
    assert_eq!(serialized, "\"#u1 https://example.test/source\"");

    let deserialized =
        json::from_str::<PointerList>(&serialized).expect("pointer list should deserialize");
    assert_eq!(deserialized, pointer_list_attr);
}

#[rstest]
fn borrowed_pointer_list_is_directly_iterable(pointer_list_internal_refs: PointerList) {
    let collected: Vec<&str> = (&pointer_list_internal_refs)
        .into_iter()
        .map(Pointer::as_str)
        .collect();

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
