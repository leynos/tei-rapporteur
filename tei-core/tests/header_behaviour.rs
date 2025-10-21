//! Behaviour-driven tests for TEI header assembly and validation.
#![allow(
    clippy::expect_used,
    reason = "Tests abort with explicit messages when scenario state is missing."
)]

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::{cell::RefCell, fmt::Display};
use tei_core::{
    AnnotationSystem, DocumentTitleError, EncodingDesc, FileDesc, HeaderValidationError,
    ProfileDesc, RevisionChange, RevisionDesc, TeiDocument, TeiHeader, TeiText,
};

#[derive(Default)]
struct HeaderState {
    title: RefCell<Option<String>>,
    profile: RefCell<ProfileDesc>,
    encoding: RefCell<EncodingDesc>,
    revision: RefCell<RevisionDesc>,
    document: RefCell<Option<Result<TeiDocument, DocumentTitleError>>>,
    revision_attempt: RefCell<Option<Result<RevisionChange, HeaderValidationError>>>,
    pending_revision_description: RefCell<Option<String>>,
}

impl HeaderState {
    fn set_title(&self, title: String) {
        *self.title.borrow_mut() = Some(title);
    }

    fn title(&self) -> String {
        self.title
            .borrow()
            .as_ref()
            .cloned()
            .expect("scenario must declare a document title")
    }

    fn profile(&self) -> ProfileDesc {
        self.profile.borrow().clone()
    }

    fn profile_mut(&self) -> std::cell::RefMut<'_, ProfileDesc> {
        self.profile.borrow_mut()
    }

    fn encoding(&self) -> EncodingDesc {
        self.encoding.borrow().clone()
    }

    fn encoding_mut(&self) -> std::cell::RefMut<'_, EncodingDesc> {
        self.encoding.borrow_mut()
    }

    fn revision(&self) -> RevisionDesc {
        self.revision.borrow().clone()
    }

    fn revision_mut(&self) -> std::cell::RefMut<'_, RevisionDesc> {
        self.revision.borrow_mut()
    }

    fn set_document(&self, result: Result<TeiDocument, DocumentTitleError>) {
        *self.document.borrow_mut() = Some(result);
    }

    fn document(&self) -> Result<TeiDocument, DocumentTitleError> {
        self.document
            .borrow()
            .as_ref()
            .cloned()
            .expect("document construction must run before assertions")
    }

    fn set_revision_attempt(&self, attempt: Result<RevisionChange, HeaderValidationError>) {
        *self.revision_attempt.borrow_mut() = Some(attempt);
    }

    fn revision_attempt(&self) -> Result<RevisionChange, HeaderValidationError> {
        self.revision_attempt
            .borrow()
            .as_ref()
            .cloned()
            .expect("revision attempt must run before assertions")
    }

    fn set_pending_revision_description(&self, description: String) {
        *self.pending_revision_description.borrow_mut() = Some(description);
    }

    fn pending_revision_description(&self) -> Option<String> {
        self.pending_revision_description.borrow().clone()
    }
}

fn expect_ok<T, E>(result: Result<T, E>, message: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(error) => panic!("{message}: {error}"),
    }
}

fn expect_document(state: &HeaderState) -> TeiDocument {
    expect_ok(state.document(), "document should be valid")
}

fn expect_profile_desc(state: &HeaderState) -> ProfileDesc {
    expect_document(state)
        .header()
        .profile_desc()
        .cloned()
        .expect("profile metadata should be present")
}

fn expect_encoding_desc(state: &HeaderState) -> EncodingDesc {
    expect_document(state)
        .header()
        .encoding_desc()
        .cloned()
        .expect("encoding metadata should be present")
}

fn expect_revision_desc(state: &HeaderState) -> RevisionDesc {
    expect_document(state)
        .header()
        .revision_desc()
        .cloned()
        .expect("revision metadata should be present")
}

#[fixture]
fn state() -> HeaderState {
    HeaderState::default()
}

#[given("a document title \"{title}\"")]
fn a_document_title(state: &HeaderState, title: String) {
    state.set_title(title);
}

#[given("a profile synopsis \"{synopsis}\"")]
fn a_profile_synopsis(state: &HeaderState, synopsis: String) {
    let updated = state.profile().with_synopsis(synopsis);
    *state.profile_mut() = updated;
}

#[given("a recording language \"{language}\"")]
fn a_recording_language(state: &HeaderState, language: String) {
    expect_ok(
        state.profile_mut().add_language(language),
        "language should be recorded",
    );
}

#[given("a cast member \"{speaker}\"")]
fn a_cast_member(state: &HeaderState, speaker: String) {
    expect_ok(
        state.profile_mut().add_speaker(speaker),
        "speaker should be recorded",
    );
}

#[given("an annotation system \"{identifier}\" described as \"{description}\"")]
fn an_annotation_system(state: &HeaderState, identifier: String, description: String) {
    let system = expect_ok(
        AnnotationSystem::new(identifier, description),
        "annotation system should validate",
    );
    state.encoding_mut().add_annotation_system(system);
}

#[given("a revision change \"{description}\"")]
fn a_revision_change(state: &HeaderState, description: String) {
    let change = expect_ok(
        RevisionChange::new(description, ""),
        "revision description should validate",
    );
    state.revision_mut().add_change(change);
}

#[given("an empty revision description")]
fn an_empty_revision_description(state: &HeaderState) {
    state.set_pending_revision_description(String::new());
}

#[when("I assemble the TEI document")]
fn i_assemble_the_tei_document(state: &HeaderState) {
    let result = (|| {
        let file_desc = FileDesc::from_title_str(&state.title())?;
        let mut header = TeiHeader::new(file_desc);

        let profile = state.profile();
        if !profile.is_empty() {
            header = header.with_profile_desc(profile);
        }

        let encoding = state.encoding();
        if !encoding.is_empty() {
            header = header.with_encoding_desc(encoding);
        }

        let revision = state.revision();
        if !revision.is_empty() {
            header = header.with_revision_desc(revision);
        }

        Ok(TeiDocument::new(header, TeiText::empty()))
    })();

    state.set_document(result);
}

#[when("I attempt to record the revision")]
fn i_attempt_to_record_the_revision(state: &HeaderState) {
    let description = state
        .pending_revision_description()
        .expect("scenario must configure the revision attempt");
    let attempt = RevisionChange::new(description, "");
    state.set_revision_attempt(attempt);
}

#[then("the document title should be \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_document_title_should_be(state: &HeaderState, expected: String) {
    let document = expect_document(state);
    assert_eq!(document.title().as_str(), expected.as_str());
}

#[then("the profile languages should include \"{language}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_profile_languages_should_include(state: &HeaderState, language: String) {
    let profile = expect_profile_desc(state);
    assert!(
        profile
            .languages()
            .iter()
            .any(|item| item == language.as_str())
    );
}

#[then("the profile speakers should include \"{speaker}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_profile_speakers_should_include(state: &HeaderState, speaker: String) {
    let profile = expect_profile_desc(state);
    assert!(
        profile
            .speakers()
            .iter()
            .any(|item| item == speaker.as_str())
    );
}

#[then("the header should record an annotation system \"{identifier}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_header_should_record_an_annotation_system(state: &HeaderState, identifier: String) {
    let encoding = expect_encoding_desc(state);
    assert!(
        encoding
            .annotation_systems()
            .iter()
            .any(|system| system.identifier() == identifier.as_str())
    );
}

#[then("the header should record the revision note \"{description}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_header_should_record_the_revision_note(state: &HeaderState, description: String) {
    let revision = expect_revision_desc(state);
    assert!(
        revision
            .changes()
            .iter()
            .any(|change| change.description() == description.as_str())
    );
}

#[then("header validation fails with \"{message}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn header_validation_fails_with(state: &HeaderState, message: String) {
    let Err(error) = state.revision_attempt() else {
        panic!("expected revision validation to fail");
    };
    assert_eq!(error.to_string(), message);
}

#[scenario(path = "tests/features/header.feature", index = 0)]
fn assembles_a_header(state: HeaderState) {
    let _ = state;
}

#[scenario(path = "tests/features/header.feature", index = 1)]
fn rejects_blank_revision_notes(state: HeaderState) {
    let _ = state;
}
