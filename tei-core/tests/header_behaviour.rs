//! Behaviour-driven tests for TEI header assembly and validation.

use anyhow::{Context, Result, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{
    AnnotationSystem, EncodingDesc, FileDesc, HeaderValidationError, ProfileDesc, RevisionChange,
    RevisionDesc, TeiDocument, TeiError, TeiHeader, TeiText,
};

#[derive(Default)]
struct HeaderState {
    title: RefCell<Option<String>>,
    profile: RefCell<ProfileDesc>,
    encoding: RefCell<EncodingDesc>,
    revision: RefCell<RevisionDesc>,
    document: RefCell<Option<TeiDocument>>,
    revision_attempt: RefCell<Option<Result<RevisionChange, HeaderValidationError>>>,
    pending_revision_description: RefCell<Option<String>>,
}

impl HeaderState {
    fn set_title(&self, title: String) {
        *self.title.borrow_mut() = Some(title);
    }

    fn title(&self) -> Result<String> {
        self.title
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must declare a document title")
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

    fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
    }

    fn document(&self) -> Result<TeiDocument> {
        self.document
            .borrow()
            .as_ref()
            .cloned()
            .context("document construction must run before assertions")
    }

    fn set_revision_attempt(&self, attempt: Result<RevisionChange, HeaderValidationError>) {
        *self.revision_attempt.borrow_mut() = Some(attempt);
    }

    fn revision_attempt(&self) -> Result<Result<RevisionChange, HeaderValidationError>> {
        self.revision_attempt
            .borrow()
            .as_ref()
            .cloned()
            .context("revision attempt must run before assertions")
    }

    fn set_pending_revision_description(&self, description: String) {
        *self.pending_revision_description.borrow_mut() = Some(description);
    }

    fn pending_revision_description(&self) -> Option<String> {
        self.pending_revision_description.borrow().clone()
    }
}

fn expect_document(state: &HeaderState) -> Result<TeiDocument> {
    state.document()
}

fn expect_profile_desc(state: &HeaderState) -> Result<ProfileDesc> {
    expect_document(state)?
        .header()
        .profile_desc()
        .cloned()
        .context("profile metadata should be present")
}

fn expect_encoding_desc(state: &HeaderState) -> Result<EncodingDesc> {
    expect_document(state)?
        .header()
        .encoding_desc()
        .cloned()
        .context("encoding metadata should be present")
}

fn expect_revision_desc(state: &HeaderState) -> Result<RevisionDesc> {
    expect_document(state)?
        .header()
        .revision_desc()
        .cloned()
        .context("revision metadata should be present")
}

fn build_state() -> Result<HeaderState> {
    let state = HeaderState::default();
    ensure!(
        state.title.borrow().is_none(),
        "fresh state should not carry a title"
    );
    ensure!(
        state.document.borrow().is_none(),
        "fresh state should not carry a document"
    );
    ensure!(
        state.revision_attempt.borrow().is_none(),
        "fresh state should not carry revision attempts"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> HeaderState {
    match build_state() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialise header state: {error}"),
    }
}

#[fixture]
fn validated_state_result() -> Result<HeaderState> {
    build_state()
}

#[given("a document title \"{title}\"")]
fn a_document_title(#[from(validated_state)] state: &HeaderState, title: String) -> Result<()> {
    state.set_title(title);
    let _ = state.title()?;
    Ok(())
}

#[given("a profile synopsis \"{synopsis}\"")]
fn a_profile_synopsis(
    #[from(validated_state)] state: &HeaderState,
    synopsis: String,
) -> Result<()> {
    let updated = state.profile().with_synopsis(synopsis);
    *state.profile_mut() = updated;
    ensure!(
        state.profile().synopsis().is_some(),
        "synopsis should be recorded"
    );
    Ok(())
}

#[given("a recording language \"{language}\"")]
fn a_recording_language(
    #[from(validated_state)] state: &HeaderState,
    language: String,
) -> Result<()> {
    state
        .profile_mut()
        .add_language(language)
        .context("language should be recorded")?;
    Ok(())
}

#[given("a cast member \"{speaker}\"")]
fn a_cast_member(#[from(validated_state)] state: &HeaderState, speaker: String) -> Result<()> {
    state
        .profile_mut()
        .add_speaker(speaker)
        .context("speaker should be recorded")?;
    Ok(())
}

#[given("an annotation system \"{identifier}\" described as \"{description}\"")]
fn an_annotation_system(
    #[from(validated_state)] state: &HeaderState,
    identifier: String,
    description: String,
) -> Result<()> {
    let system = AnnotationSystem::new(identifier, description)
        .context("annotation system should validate")?;
    state.encoding_mut().add_annotation_system(system);
    Ok(())
}

#[given("a revision change \"{description}\"")]
fn a_revision_change(
    #[from(validated_state)] state: &HeaderState,
    description: String,
) -> Result<()> {
    let change =
        RevisionChange::new(description, "").context("revision description should validate")?;
    state.revision_mut().add_change(change);
    ensure!(
        !state.revision().is_empty(),
        "revision history should record changes"
    );
    Ok(())
}

#[given("an empty revision description")]
fn an_empty_revision_description(#[from(validated_state)] state: &HeaderState) -> Result<()> {
    state.set_pending_revision_description(String::new());
    ensure!(
        state.pending_revision_description().as_deref() == Some(""),
        "pending revision description should be staged"
    );
    Ok(())
}

#[when("I assemble the TEI document")]
fn i_assemble_the_tei_document(#[from(validated_state)] state: &HeaderState) -> Result<()> {
    let title = state.title()?;
    let result = (|| -> Result<TeiDocument, TeiError> {
        let file_desc = FileDesc::from_title_str(&title)?;
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

    let document = result.context("document should be valid")?;
    state.set_document(document);
    Ok(())
}

#[when("I attempt to record the revision")]
fn i_attempt_to_record_the_revision(#[from(validated_state)] state: &HeaderState) -> Result<()> {
    let description = state
        .pending_revision_description()
        .context("scenario must configure the revision attempt")?;
    let attempt = RevisionChange::new(description, "");
    state.set_revision_attempt(attempt);
    Ok(())
}

#[then("the document title should be \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_document_title_should_be(
    #[from(validated_state)] state: &HeaderState,
    expected: String,
) -> Result<()> {
    let document = expect_document(state)?;
    let actual_title = document.title().as_str();
    ensure!(
        actual_title == expected.as_str(),
        "document title mismatch: expected {expected}, found {actual_title}"
    );
    Ok(())
}

#[then("the profile languages should include \"{language}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_profile_languages_should_include(
    #[from(validated_state)] state: &HeaderState,
    language: String,
) -> Result<()> {
    let profile = expect_profile_desc(state)?;
    ensure!(
        profile
            .languages()
            .iter()
            .any(|item| item.as_str() == language.as_str()),
        "language missing from profile: {language}"
    );
    Ok(())
}

#[then("the profile speakers should include \"{speaker}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_profile_speakers_should_include(
    #[from(validated_state)] state: &HeaderState,
    speaker: String,
) -> Result<()> {
    let profile = expect_profile_desc(state)?;
    ensure!(
        profile
            .speakers()
            .iter()
            .any(|item| item.as_str() == speaker.as_str()),
        "speaker missing from profile: {speaker}"
    );
    Ok(())
}

#[then("the header should record an annotation system \"{identifier}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_header_should_record_an_annotation_system(
    #[from(validated_state)] state: &HeaderState,
    identifier: String,
) -> Result<()> {
    let encoding = expect_encoding_desc(state)?;
    ensure!(
        encoding
            .annotation_systems()
            .iter()
            .any(|system| system.identifier() == identifier.as_str()),
        "annotation system not recorded: {identifier}"
    );
    Ok(())
}

#[then("the header should record the revision note \"{description}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn the_header_should_record_the_revision_note(
    #[from(validated_state)] state: &HeaderState,
    description: String,
) -> Result<()> {
    let revision = expect_revision_desc(state)?;
    ensure!(
        revision
            .changes()
            .iter()
            .any(|change| change.description() == description.as_str()),
        "revision note not recorded: {description}"
    );
    Ok(())
}

#[then("header validation fails with \"{message}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn header_validation_fails_with(
    #[from(validated_state)] state: &HeaderState,
    message: String,
) -> Result<()> {
    let attempt = state.revision_attempt()?;
    let Err(error) = attempt else {
        bail!("expected revision validation to fail");
    };
    let actual_message = error.to_string();
    let error_type = std::any::type_name_of_val(&error);
    ensure!(
        actual_message == message,
        "revision validation mismatch: expected {message}, found {actual_message}; error_type={error_type}, error={error:?}"
    );
    Ok(())
}

fn expect_validated_header_state(result: Result<HeaderState>) {
    if let Err(error) = result {
        panic!("header scenarios must initialise their state successfully: {error}");
    }
}

#[scenario(path = "tests/features/header.feature", index = 0)]
fn assembles_a_header(
    #[from(validated_state)] state: HeaderState,
    #[from(validated_state_result)] validated_state: Result<HeaderState>,
) {
    drop(state);
    expect_validated_header_state(validated_state);
}

#[scenario(path = "tests/features/header.feature", index = 1)]
fn rejects_blank_revision_notes(
    #[from(validated_state)] state: HeaderState,
    #[from(validated_state_result)] validated_state: Result<HeaderState>,
) {
    drop(state);
    expect_validated_header_state(validated_state);
}
