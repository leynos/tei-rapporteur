//! Behaviour-driven scenarios covering document title serialization.

use anyhow::{Context, Result, bail, ensure};
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

    fn raw_title(&self) -> Result<String> {
        self.raw_title
            .borrow()
            .as_ref()
            .cloned()
            .context("the scenario must define a title")
    }

    fn set_serialized(&self, result: Result<String, DocumentTitleError>) {
        *self.serialized.borrow_mut() = Some(result);
    }

    fn serialized(&self) -> Result<Result<String, DocumentTitleError>> {
        self.serialized
            .borrow()
            .as_ref()
            .cloned()
            .context("serialization must run before assertions")
    }

    fn set_document(&self, result: Result<TeiDocument, DocumentTitleError>) {
        *self.document.borrow_mut() = Some(result);
    }

    fn document(&self) -> Result<Result<TeiDocument, DocumentTitleError>> {
        self.document
            .borrow()
            .as_ref()
            .cloned()
            .context("document construction must run before assertions")
    }
}

/// Provides shared scenario state for title serialization steps.
#[fixture]
fn validated_state_result() -> Result<TitleState> {
    let state = TitleState::default();
    ensure!(
        state.raw_title.borrow().is_none(),
        "fresh title state must not contain a raw title"
    );
    ensure!(
        state.serialized.borrow().is_none(),
        "fresh title state must not contain serialized output"
    );
    ensure!(
        state.document.borrow().is_none(),
        "fresh title state must not contain documents"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> TitleState {
    match validated_state_result() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialise title state: {error}"),
    }
}

#[given("a document title \"{title}\"")]
fn a_document_title(#[from(validated_state)] state: &TitleState, title: String) -> Result<()> {
    state.set_raw_title(title);
    let _ = state.raw_title()?;
    Ok(())
}

#[given("no document title")]
fn no_document_title(#[from(validated_state)] state: &TitleState) -> Result<()> {
    state.set_raw_title(String::new());
    let _ = state.raw_title()?;
    Ok(())
}

#[when("I serialize the document title")]
fn i_serialize_the_document_title(#[from(validated_state)] state: &TitleState) -> Result<()> {
    let title = state.raw_title()?;
    let result = serialize_document_title(&title);
    state.set_serialized(result);
    Ok(())
}

#[when("I attempt to build the document")]
fn i_attempt_to_build_the_document(#[from(validated_state)] state: &TitleState) -> Result<()> {
    let title = state.raw_title()?;
    let result = TeiDocument::from_title_str(&title);
    state.set_document(result);
    Ok(())
}

#[then("the XML output is \"{expected}\"")]
fn the_xml_output_is(#[from(validated_state)] state: &TitleState, expected: String) -> Result<()> {
    let expected_markup = expected.into_boxed_str();
    let markup = state
        .serialized()?
        .context("expected successful serialization")?;
    ensure!(
        markup == expected_markup.as_ref(),
        "serialized markup mismatch: expected {expected_markup:?}, found {markup:?}"
    );
    Ok(())
}

#[then("title creation fails with \"{message}\"")]
fn title_creation_fails_with(
    #[from(validated_state)] state: &TitleState,
    message: String,
) -> Result<()> {
    let expected_message = message.into_boxed_str();
    let outcome = state.document()?;
    let Err(error) = outcome else {
        bail!("expected document creation to fail");
    };
    let actual_message = error.to_string();
    ensure!(
        actual_message == expected_message.as_ref(),
        "title error mismatch: expected {expected_message:?}, found {actual_message}"
    );
    Ok(())
}

#[scenario(path = "tests/features/title_serialization.feature", index = 0)]
fn serializes_a_valid_title(
    #[from(validated_state)] state: TitleState,
    #[from(validated_state_result)] result: Result<TitleState>,
) -> Result<()> {
    drop(state);
    let _ = result?;
    Ok(())
}

#[scenario(path = "tests/features/title_serialization.feature", index = 1)]
fn escapes_markup_significant_characters(
    #[from(validated_state)] state: TitleState,
    #[from(validated_state_result)] result: Result<TitleState>,
) -> Result<()> {
    drop(state);
    let _ = result?;
    Ok(())
}

#[scenario(path = "tests/features/title_serialization.feature", index = 2)]
fn rejects_an_empty_title(
    #[from(validated_state)] state: TitleState,
    #[from(validated_state_result)] result: Result<TitleState>,
) -> Result<()> {
    drop(state);
    let _ = result?;
    Ok(())
}
