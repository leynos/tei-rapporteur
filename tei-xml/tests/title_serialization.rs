//! Behaviour-driven scenarios covering document title serialization.

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{DocumentTitleError, TeiDocument};
use tei_xml::serialize_document_title;

#[derive(Default)]
struct TitleState {
    raw_title: RefCell<Option<String>>,
    serialized: RefCell<Option<Result<String, DocumentTitleError>>>,
    document: RefCell<Option<Result<TeiDocument, DocumentTitleError>>>,
}

impl TitleState {
    fn set_raw_title(&self, title: String) {
        *self.raw_title.borrow_mut() = Some(title);
    }

    fn raw_title(&self) -> String {
        self.raw_title
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| panic!("the scenario must define a title"))
    }

    fn set_serialized(&self, result: Result<String, DocumentTitleError>) {
        *self.serialized.borrow_mut() = Some(result);
    }

    fn serialized(&self) -> Result<String, DocumentTitleError> {
        self.serialized
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| panic!("serialization must run before assertions"))
    }

    fn set_document(&self, result: Result<TeiDocument, DocumentTitleError>) {
        *self.document.borrow_mut() = Some(result);
    }

    fn document(&self) -> Result<TeiDocument, DocumentTitleError> {
        self.document
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| panic!("document construction must run before assertions"))
    }
}

/// Provides shared scenario state for title serialization steps.
#[fixture]
fn state() -> TitleState {
    TitleState::default()
}

#[given("a document title \"{title}\"")]
fn a_document_title(state: &TitleState, title: String) {
    state.set_raw_title(title);
}

#[given("no document title")]
fn no_document_title(state: &TitleState) {
    state.set_raw_title(String::new());
}

#[when("I serialize the document title")]
fn i_serialize_the_document_title(state: &TitleState) {
    let result = serialize_document_title(&state.raw_title());
    state.set_serialized(result);
}

#[when("I attempt to build the document")]
fn i_attempt_to_build_the_document(state: &TitleState) {
    let result = TeiDocument::from_title_str(&state.raw_title());
    state.set_document(result);
}

#[then("the XML output is \"{expected}\"")]
fn the_xml_output_is(state: &TitleState, expected: String) {
    let expected = expected.into_boxed_str();
    let markup = match state.serialized() {
        Ok(value) => value,
        Err(error) => panic!("expected successful serialization: {error}"),
    };
    assert_eq!(markup, expected.as_ref());
}

#[then("title creation fails with \"{message}\"")]
fn title_creation_fails_with(state: &TitleState, message: String) {
    let message = message.into_boxed_str();
    let Err(error) = state.document() else {
        panic!("expected document creation to fail");
    };
    assert_eq!(error.to_string(), message.as_ref());
}

#[scenario(path = "tests/features/title_serialization.feature", index = 0)]
fn serializes_a_valid_title(state: TitleState) {
    let _ = state;
}

#[scenario(path = "tests/features/title_serialization.feature", index = 1)]
fn escapes_markup_significant_characters(state: TitleState) {
    let _ = state;
}

#[scenario(path = "tests/features/title_serialization.feature", index = 2)]
fn rejects_an_empty_title(state: TitleState) {
    let _ = state;
}
