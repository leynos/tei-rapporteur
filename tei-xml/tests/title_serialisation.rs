use std::cell::RefCell;
use std::convert::Infallible;
use std::str::FromStr;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use tei_core::{DocumentTitleError, TeiDocument};
use tei_xml::serialise_document_title;

#[derive(Default)]
struct TitleState {
    raw_title: RefCell<Option<String>>,
    serialised: RefCell<Option<Result<String, DocumentTitleError>>>,
    document: RefCell<Option<Result<TeiDocument, DocumentTitleError>>>,
}

struct StepString(String);

impl StepString {
    fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for StepString {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.to_owned()))
    }
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

    fn set_serialised(&self, result: Result<String, DocumentTitleError>) {
        *self.serialised.borrow_mut() = Some(result);
    }

    fn serialised(&self) -> Result<String, DocumentTitleError> {
        self.serialised
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| panic!("serialisation must run before assertions"))
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

#[fixture]
pub fn state() -> TitleState {
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

#[when("I serialise the document title")]
fn i_serialise_the_document_title(state: &TitleState) {
    let result = serialise_document_title(&state.raw_title());
    state.set_serialised(result);
}

#[when("I attempt to build the document")]
fn i_attempt_to_build_the_document(state: &TitleState) {
    let result = TeiDocument::from_title_str(&state.raw_title());
    state.set_document(result);
}

#[then("the XML output is \"{expected}\"")]
fn the_xml_output_is(state: &TitleState, expected: StepString) {
    let markup = match state.serialised() {
        Ok(value) => value,
        Err(error) => panic!("expected successful serialisation: {error}"),
    };
    assert_eq!(markup, expected.into_inner());
}

#[then("title creation fails with \"{message}\"")]
fn title_creation_fails_with(state: &TitleState, message: StepString) {
    let Err(error) = state.document() else {
        panic!("expected document creation to fail");
    };
    assert_eq!(error.to_string(), message.into_inner());
}

#[scenario(path = "tests/features/title_serialisation.feature", index = 0)]
fn serialises_a_valid_title(state: TitleState) {
    let _ = state;
}

#[scenario(path = "tests/features/title_serialisation.feature", index = 1)]
fn escapes_markup_significant_characters(state: TitleState) {
    let _ = state;
}

#[scenario(path = "tests/features/title_serialisation.feature", index = 2)]
fn rejects_an_empty_title(state: TitleState) {
    let _ = state;
}
