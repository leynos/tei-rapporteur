//! Behaviour-driven tests for document-level validation.

use anyhow::{Context, Result, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{P, ProfileDesc, TeiDocument, TeiError, Utterance};
use tei_test_helpers::expect_validated_state;

#[derive(Default)]
struct ValidationState {
    document: RefCell<Option<TeiDocument>>,
    last_error: RefCell<Option<TeiError>>,
}

impl ValidationState {
    fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
        self.clear_error();
    }

    fn document(&self) -> Result<TeiDocument> {
        self.document
            .borrow()
            .clone()
            .context("scenario should configure the TEI document")
    }

    fn update_document<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&TeiDocument) -> Result<TeiDocument>,
    {
        let mut slot = self.document.borrow_mut();
        let next_document = {
            let document = slot
                .as_ref()
                .context("scenario should configure the TEI document")?;
            updater(document)?
        };
        *slot = Some(next_document);
        Ok(())
    }

    fn record_error(&self, error: TeiError) {
        *self.last_error.borrow_mut() = Some(error);
    }

    fn clear_error(&self) {
        *self.last_error.borrow_mut() = None;
    }

    fn last_error(&self) -> Option<TeiError> {
        self.last_error.borrow().clone()
    }
}

fn build_state() -> Result<ValidationState> {
    let state = ValidationState::default();
    ensure!(
        state.document.borrow().is_none(),
        "fresh validation state must start without a document",
    );
    ensure!(
        state.last_error.borrow().is_none(),
        "fresh validation state must not record errors",
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> ValidationState {
    expect_validated_state(build_state(), "validation")
}

#[fixture]
fn validated_state_result() -> Result<ValidationState> {
    build_state()
}

#[given("a TEI document titled \"{title}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn a_tei_document_titled(
    #[from(validated_state)] state: &ValidationState,
    title: String,
) -> Result<()> {
    let document = TeiDocument::from_title_str(title.as_str())
        .context("document should construct from title")?;
    state.set_document(document);
    Ok(())
}

#[given("the profile includes speaker \"{speaker}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_profile_includes_speaker(
    #[from(validated_state)] state: &ValidationState,
    speaker: String,
) -> Result<()> {
    state.update_document(|document| add_speaker(document, speaker.as_str()))
}

#[when("I add a paragraph \"{content}\" with id \"{identifier}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn i_add_a_paragraph(
    #[from(validated_state)] state: &ValidationState,
    content: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| add_paragraph(document, &content, &identifier))
}

#[when("I add an utterance for \"{speaker}\" saying \"{content}\" with id \"{identifier}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn i_add_an_utterance(
    #[from(validated_state)] state: &ValidationState,
    speaker: String,
    content: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| {
        add_utterance(document, &speaker, &content, Some(identifier.as_str()))
    })
}

#[when("I validate the document")]
fn i_validate_the_document(#[from(validated_state)] state: &ValidationState) -> Result<()> {
    let document = state.document()?;
    match document.validate() {
        Ok(()) => state.clear_error(),
        Err(error) => state.record_error(error),
    }

    Ok(())
}

#[then("validation succeeds")]
fn validation_succeeds(#[from(validated_state)] state: &ValidationState) -> Result<()> {
    ensure!(
        state.last_error().is_none(),
        "expected validation to succeed"
    );
    Ok(())
}

#[then("validation fails with \"{message}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn validation_fails_with(
    #[from(validated_state)] state: &ValidationState,
    message: String,
) -> Result<()> {
    let error = state.last_error().context("expected a validation error")?;
    let actual = error.to_string();
    ensure!(
        actual.contains(&message),
        "validation error should contain '{message}', found '{actual}'"
    );
    Ok(())
}

fn add_speaker(document: &TeiDocument, speaker: &str) -> Result<TeiDocument> {
    let mut profile = document
        .header()
        .profile_desc()
        .cloned()
        .unwrap_or_else(ProfileDesc::new);
    profile
        .add_speaker(speaker)
        .context("speaker should validate")?;
    let header = document.header().clone().with_profile_desc(profile);
    let text = document.text().clone();

    Ok(TeiDocument::new(header, text))
}

fn add_paragraph(document: &TeiDocument, content: &str, identifier: &str) -> Result<TeiDocument> {
    let mut paragraph = P::from_text_segments([content]).context("paragraph should be valid")?;
    paragraph
        .set_id(identifier)
        .context("identifier should validate")?;

    let mut text = document.text().clone();
    text.body_mut().push_paragraph(paragraph);

    Ok(TeiDocument::new(document.header().clone(), text))
}

fn add_utterance(
    document: &TeiDocument,
    speaker: &str,
    content: &str,
    identifier: Option<&str>,
) -> Result<TeiDocument> {
    let mut utterance = Utterance::from_text_segments(Some(speaker), [content])
        .context("utterance should be valid")?;
    if let Some(id) = identifier {
        utterance.set_id(id).context("identifier should validate")?;
    }

    let mut text = document.text().clone();
    text.body_mut().push_utterance(utterance);

    Ok(TeiDocument::new(document.header().clone(), text))
}

#[scenario(path = "tests/features/validation.feature", index = 0)]
fn accepts_unique_ids_and_declared_speakers(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 1)]
fn rejects_duplicate_identifiers(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 2)]
fn rejects_unknown_speakers(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 3)]
fn allows_speakers_without_cast(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}
