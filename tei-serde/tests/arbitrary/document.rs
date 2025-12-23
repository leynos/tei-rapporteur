//! Proptest strategy for complete `TeiDocument` instances.
//!
//! Combines header and text strategies to produce valid documents suitable
//! for round-trip testing.

use proptest::prelude::*;
use tei_core::TeiDocument;

use super::header::tei_header_strategy;
use super::text::{tei_text_strategy, text_only_tei_text_strategy};

/// Generates a complete `TeiDocument` with valid header and body.
///
/// Note: Does not guarantee document-level validation passes (e.g., speaker
/// cross-references, unique xml:id values). Use [`valid_tei_document_strategy`]
/// for documents that must pass `TeiDocument::validate()`.
pub fn tei_document_strategy() -> impl Strategy<Value = TeiDocument> {
    (tei_header_strategy(), tei_text_strategy())
        .prop_map(|(header, text)| TeiDocument::new(header, text))
}

/// Generates a `TeiDocument` that passes document-level validation.
///
/// This ensures xml:id uniqueness and speaker cross-reference integrity by
/// filtering out documents that fail `TeiDocument::validate()`.
pub fn valid_tei_document_strategy() -> impl Strategy<Value = TeiDocument> {
    tei_document_strategy().prop_filter("document must pass validation", |doc| {
        doc.validate().is_ok()
    })
}

/// Generates a text-only `TeiDocument` (no Hi or Pause elements in body).
///
/// This is needed for XML round-trip testing because `quick-xml`'s serde
/// integration does not support serializing complex inline structures.
pub fn text_only_tei_document_strategy() -> impl Strategy<Value = TeiDocument> {
    (tei_header_strategy(), text_only_tei_text_strategy())
        .prop_map(|(header, text)| TeiDocument::new(header, text))
}

/// Generates a text-only `TeiDocument` that passes document-level validation.
///
/// This is needed for XML round-trip testing because `quick-xml`'s serde
/// integration does not support serializing complex inline structures.
pub fn valid_text_only_tei_document_strategy() -> impl Strategy<Value = TeiDocument> {
    text_only_tei_document_strategy().prop_filter("document must pass validation", |doc| {
        doc.validate().is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    #[test]
    fn tei_document_strategy_produces_documents() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let doc = tei_document_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            assert!(
                !doc.title().as_str().trim().is_empty(),
                "document title must not be empty"
            );
        }
    }

    #[test]
    fn valid_tei_document_strategy_produces_valid_documents() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let doc = valid_tei_document_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            assert!(
                doc.validate().is_ok(),
                "document must pass validation: {:?}",
                doc.validate()
            );
        }
    }
}
