//! Property tests for `@corresp` pointer validation.

use anyhow::Context;
use proptest::prelude::*;
use tei_core::{Div, Item, List, PointerList, TeiDocument};

fn external_pointer() -> Result<BoxedStrategy<String>, Box<prop::string::Error>> {
    Ok(prop_oneof![
        prop::string::string_regex("urn:[a-z]{2,8}:[a-z0-9\\-]{1,20}:[a-z0-9]{8,16}")?,
        prop::string::string_regex("tag:[a-z0-9.\\-]{3,30},[0-9]{4}:[a-z0-9\\-]{1,20}")?,
        prop::string::string_regex("https://[a-z]{3,10}\\.[a-z]{2,6}/[a-z0-9/\\-]{1,30}")?,
    ]
    .boxed())
}

fn internal_pointer() -> Result<BoxedStrategy<String>, Box<prop::string::Error>> {
    Ok(prop::string::string_regex("#[a-zA-Z][a-zA-Z0-9_\\-]{1,20}")?.boxed())
}

fn document_with_corresp(corresp: &str) -> anyhow::Result<TeiDocument> {
    let document =
        TeiDocument::from_title_str("Corresp Property").context("document should construct")?;
    let mut item = Item::from_text_segments(["External reference"]).context("item should build")?;
    item.set_corresp(PointerList::new([corresp])?);
    let list = List::new([item]).context("list should be valid")?;
    let mut div = Div::new("references").context("division should be valid")?;
    div.push_list(list);
    let mut text = document.text().clone();
    text.body_mut().push_div(div);
    Ok(TeiDocument::new(document.header().clone(), text))
}

fn prop_result<T>(result: anyhow::Result<T>) -> Result<T, TestCaseError> {
    result.map_err(|error| TestCaseError::fail(error.to_string()))
}

mod corresp_validation_props {
    //! Property tests for @corresp pointer validation.
    use super::*;

    proptest! {
        #[test]
        fn external_corresp_pointers_pass_validation(pointer in external_pointer().expect("external pointer regexes should compile")) {
            let document = prop_result(document_with_corresp(&pointer))?;
            let validation = document.validate();
            prop_assert!(validation.is_ok(), "external pointer should validate: {validation:?}");
        }

        #[test]
        fn unresolved_internal_corresp_pointers_fail_validation(pointer in internal_pointer().expect("internal pointer regex should compile")) {
            let document = prop_result(document_with_corresp(&pointer))?;
            let error = document
                .validate()
                .err()
                .ok_or_else(|| TestCaseError::fail("unresolved internal pointer should fail"))?;
            let message = error.to_string();
            prop_assert!(
                message.contains("internal pointer"),
                "validation error should mention internal pointer: {message}"
            );
            prop_assert!(
                message.contains("does not resolve"),
                "validation error should mention unresolved target: {message}"
            );
        }
    }
}
