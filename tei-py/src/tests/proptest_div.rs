//! Property-based round-trip tests for div-containing documents.
//!
//! Verifies that arbitrary `Div` structures survive both the dictionary and
//! `MessagePack` serialization paths without data loss.
//!
//! `prop_compose!` closures must yield a value, so the fallible `tei-core`
//! constructors they call have nowhere to propagate an error. Every such call
//! goes through the single documented panic boundary
//! [`tei_test_helpers::ExpectValid::expect_valid`] rather than an ad-hoc
//! `expect`, keeping the strategies free of scattered panics.

use crate::projection::{document_to_value, value_to_document};
use proptest::prelude::*;
use tei_core::{
    BodyBlock, Div, Head, Item, Label, List, P, TeiBody, TeiDocument, TeiHeader, TeiText,
};
use tei_test_helpers::ExpectValid;

fn ident_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,8}"
}

fn text_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z]{1,20}"
}

prop_compose! {
    fn arb_label()(text in text_strategy()) -> Label {
        Label::from_text(text).expect_valid("strategy-generated label text")
    }
}

prop_compose! {
    fn arb_item()(
        text in text_strategy(),
        label in proptest::option::of(arb_label()),
    ) -> Item {
        let mut item =
            Item::from_text_segments([text]).expect_valid("strategy-generated item text");
        if let Some(l) = label {
            item.set_label(l);
        }
        item
    }
}

prop_compose! {
    fn arb_list()(items in proptest::collection::vec(arb_item(), 1..4_usize)) -> List {
        List::new(items).expect_valid("strategy-generated non-empty item list")
    }
}

prop_compose! {
    fn arb_head()(text in text_strategy()) -> Head {
        Head::from_text(text).expect_valid("strategy-generated head text")
    }
}

prop_compose! {
    fn arb_paragraph()(text in text_strategy()) -> P {
        P::from_text_segments([text]).expect_valid("strategy-generated paragraph text")
    }
}

prop_compose! {
    fn arb_div()(
        div_type in ident_strategy(),
        subtype in proptest::option::of(ident_strategy()),
        head in proptest::option::of(arb_head()),
        list in arb_list(),
        paragraph in proptest::option::of(arb_paragraph()),
    ) -> Div {
        let mut div = Div::new(div_type).expect_valid("strategy-generated div type");
        if let Some(st) = subtype {
            div.set_subtype(st).expect_valid("strategy-generated div subtype");
        }
        if let Some(h) = head {
            div.set_head(h);
        }
        if let Some(p) = paragraph {
            div.push_paragraph(p);
        }
        div.push_list(list);
        div
    }
}

prop_compose! {
    fn arb_div_document()(
        title in "[A-Za-z][A-Za-z ]{0,18}[A-Za-z]",
        div in arb_div(),
    ) -> TeiDocument {
        let header = TeiHeader::new(
            tei_core::FileDesc::from_title_str(&title).expect_valid("strategy-generated title"),
        );
        let text = TeiText::new(TeiBody::new([BodyBlock::Div(div)]));
        TeiDocument::new(header, text)
    }
}

proptest! {
    #[test]
    fn div_document_survives_dictionary_round_trip(doc in arb_div_document()) {
        let value = document_to_value(&doc)
            .map_err(|e| TestCaseError::fail(format!("serialization failed: {e}")))?;
        let recovered = value_to_document(&value)
            .map_err(|e| TestCaseError::fail(format!("deserialization failed: {e}")))?;

        prop_assert_eq!(
            doc,
            recovered,
            "document should survive the dictionary round-trip without data loss"
        );
    }

    #[test]
    fn div_document_survives_msgpack_round_trip(doc in arb_div_document()) {
        use tei_serde::msgpack::to_vec_named;

        let projection = crate::projection::PyTeiDocument::from(&doc);
        let bytes = to_vec_named(&projection)
            .map_err(|e| TestCaseError::fail(format!("msgpack encoding failed: {e}")))?;
        let recovered = crate::document_from_msgpack(&bytes)
            .map_err(|e| TestCaseError::fail(format!("msgpack decoding failed: {e}")))?;

        prop_assert_eq!(
            doc,
            recovered,
            "document should survive the MessagePack round-trip without data loss"
        );
    }
}
