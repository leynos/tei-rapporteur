//! Behaviour-driven tests for document-level validation.

use anyhow::{Context, Result, anyhow, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{Div, Item, List, P, ProfileDesc, TeiDocument, TeiError, Utterance};
use tei_test_helpers::expect_validated_state;

mod stand_off;

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

    fn document(&self) -> Result<std::cell::Ref<'_, TeiDocument>> {
        std::cell::Ref::filter_map(self.document.borrow(), Option::as_ref)
            .map_err(|_| anyhow!("scenario should configure the TEI document"))
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
fn the_profile_includes_speaker(
    #[from(validated_state)] state: &ValidationState,
    speaker: String,
) -> Result<()> {
    state.update_document(|document| add_speaker(document, speaker.as_str()))
}

#[given("the profile is declared but empty")]
fn the_profile_is_declared_but_empty(
    #[from(validated_state)] state: &ValidationState,
) -> Result<()> {
    state.update_document(|document| Ok(declare_empty_profile(document)))
}

#[when("I add a paragraph \"{content}\" with id \"{identifier}\"")]
fn i_add_a_paragraph(
    #[from(validated_state)] state: &ValidationState,
    content: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| add_paragraph(document, &content, &identifier))
}

#[when("I add an utterance for \"{speaker}\" saying \"{content}\" with id \"{identifier}\"")]
fn i_add_an_utterance(
    #[from(validated_state)] state: &ValidationState,
    speaker: String,
    content: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut utterance =
            Utterance::from_text_segments(Some(speaker.as_str()), [content.as_str()])
                .context("utterance should be valid")?;
        utterance
            .set_id(identifier.as_str())
            .context("identifier should validate")?;
        Ok(add_utterance_to_document(document, utterance))
    })
}

#[when("I add a division \"{div_type}\" containing an item with id \"{identifier}\"")]
fn i_add_a_division_containing_an_item_with_id(
    #[from(validated_state)] state: &ValidationState,
    div_type: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut item = Item::from_text_segments(["Linked resource"]).context("item should be valid")?;
        item.set_id(identifier.as_str())
            .context("identifier should validate")?;
        let list = List::new([item]).context("list should be valid")?;
        let mut div = Div::new(div_type.as_str()).context("division should be valid")?;
        div.push_list(list);
        let mut text = document.text().clone();
        text.body_mut().push_div(div);
        Ok(TeiDocument::new(document.header().clone(), text))
    })
}

#[when("I add a division \"{div_type}\" containing an item with corresp \"{corresp}\"")]
fn i_add_a_division_containing_an_item_with_corresp(
    #[from(validated_state)] state: &ValidationState,
    div_type: String,
    corresp: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut item =
            Item::from_text_segments(["External reference"]).context("item should be valid")?;
        item.set_corresp(tei_core::PointerList::new([corresp.as_str()])?);
        let list = List::new([item]).context("list should be valid")?;
        let mut div = Div::new(div_type.as_str()).context("division should be valid")?;
        div.push_list(list);
        let mut text = document.text().clone();
        text.body_mut().push_div(div);
        Ok(TeiDocument::new(document.header().clone(), text))
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
fn validation_fails_with(
    #[from(validated_state)] state: &ValidationState,
    message: String,
) -> Result<()> {
    let error = state.last_error().context("expected a validation error")?;

    ensure!(
        matches!(error, TeiError::Validation(_)),
        "expected validation error to be TeiError::Validation, found '{error}'"
    );

    let actual = error.to_string();
    ensure!(
        actual.contains(&message),
        "validation error should contain '{message}', found '{actual}'"
    );
    Ok(())
}

/// Builds a new document whose header declares an empty profile.
///
/// Tests use this to confirm speaker references fail validation when a cast
/// exists but contains no speakers. The helper is infallible, so it returns a
/// bare `TeiDocument` to avoid needless wrapping lint noise.
fn declare_empty_profile(document: &TeiDocument) -> TeiDocument {
    let header = document
        .header()
        .clone()
        .with_profile_desc(ProfileDesc::new());

    TeiDocument::new(header, document.text().clone())
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

fn add_utterance_to_document(document: &TeiDocument, utterance: Utterance) -> TeiDocument {
    let mut text = document.text().clone();
    text.body_mut().push_utterance(utterance);
    TeiDocument::new(document.header().clone(), text)
}

// Scenario indices are coupled to validation.feature ordering. Update the
// indices below if scenarios are reordered or inserted; the guard test at the
// end of this module ensures the names stay aligned with the feature file.
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
fn rejects_header_body_identifier_clashes(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 3)]
fn rejects_header_annotation_system_clashes(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 4)]
fn rejects_unknown_speakers(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 5)]
fn rejects_speakers_when_cast_is_empty(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 6)]
fn allows_speakers_without_cast(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 10)]
fn rejects_duplicate_identifiers_inside_divisions(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(path = "tests/features/validation.feature", index = 11)]
fn rejects_unresolved_item_corresp_pointers_inside_divisions(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[test]
fn validation_feature_scenario_order_matches_expectations() {
    use gherkin::Feature;

    let feature = Feature::parse_path(
        "tests/features/validation.feature",
        gherkin::GherkinEnv::default(),
    )
    .expect("parse validation.feature");
    let names: Vec<&str> = feature
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .collect();

    let expected = [
        "Accepting unique ids and declared speakers",
        "Rejecting duplicate xml:id values",
        "Rejecting header and body identifier clashes",
        "Rejecting duplicate header annotation system identifiers",
        "Rejecting unknown speaker references",
        "Rejecting speakers when the cast is empty",
        "Allowing speakers when the cast list is absent",
        "Accepting stand-off spans that target existing utterances",
        "Rejecting stand-off spans that target missing ids",
        "Rejecting stand-off spans without anchors",
        "Rejecting duplicate xml:id values inside divisions",
        "Rejecting unresolved item corresp pointers inside divisions",
    ];

    assert_eq!(
        names, expected,
        "Scenario indices in validation_behaviour.rs must stay aligned with validation.feature"
    );
}
