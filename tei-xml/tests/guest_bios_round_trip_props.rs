//! Property tests for guest-biography XML round-tripping.

use anyhow::Context;
use proptest::prelude::*;
use tei_core::{BodyBlock, Div, FileDesc, Item, Label, List, PointerList, TeiHeader, TeiText};
use tei_xml::{emit_xml, parse_xml};

fn external_pointer() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex("urn:[a-z]{2,8}:[a-z0-9\\-]{1,20}:[a-z0-9]{8,16}")
            .unwrap_or_else(|error| panic!("URN pointer regex should compile: {error}")),
        prop::string::string_regex("tag:[a-z0-9.\\-]{3,30},[0-9]{4}:[a-z0-9\\-]{1,20}")
            .unwrap_or_else(|error| panic!("tag pointer regex should compile: {error}")),
        prop::string::string_regex("https://[a-z]{3,10}\\.[a-z]{2,6}/[a-z0-9/\\-]{1,30}")
            .unwrap_or_else(|error| panic!("HTTPS pointer regex should compile: {error}")),
    ]
}

fn document_with_guest_bios_corresp(corresp: &str) -> anyhow::Result<tei_core::TeiDocument> {
    let file_desc = FileDesc::from_title_str("Guest Biography Fixture")?;
    let header = TeiHeader::new(file_desc);

    let label = Label::from_text("Ada Lovelace")?;
    let mut item = Item::from_text_segments(["Mathematician and computing pioneer."])?;
    item.set_id("guest-bio-ada")?;
    item.set_corresp(PointerList::new([corresp])?);
    item.set_label(label);

    let mut list = List::new([item])?;
    list.set_id("guest-bio-list")?;

    let mut div = Div::new("guest-bios")?;
    div.set_id("guest-bios")?;
    div.push_list(list);

    let body = tei_core::TeiBody::new([BodyBlock::Div(div)]);
    let text = TeiText::new(body);
    Ok(tei_core::TeiDocument::new(header, text))
}

fn prop_result<T>(result: anyhow::Result<T>) -> Result<T, TestCaseError> {
    result.map_err(|error| TestCaseError::fail(error.to_string()))
}

mod guest_bios_round_trip_props {
    use super::*;

    proptest! {
        #[test]
        fn guest_bio_external_corresp_survives_round_trip(pointer in external_pointer()) {
            let document = prop_result(document_with_guest_bios_corresp(&pointer))?;
            let emitted = prop_result(emit_xml(&document).context("guest-bios TEI should emit"))?;
            let expected_corresp = format!("corresp=\"{pointer}\"");
            prop_assert!(
                emitted.contains(&expected_corresp),
                "emitted XML should preserve generated @corresp value"
            );
            let reparsed =
                prop_result(parse_xml(&emitted).context("emitted guest-bios TEI should parse"))?;
            prop_result(reparsed.validate().context("reparsed guest-bios TEI should validate"))?;
        }
    }
}
