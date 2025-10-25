use anyhow::{Result, bail, ensure};
use rstest_bdd_macros::then;

use crate::{
    helpers::{expect_document, expect_encoding_desc, expect_profile_desc, expect_revision_desc},
    HeaderState,
};

#[then("the document title should be \"{expected}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
pub(crate) fn the_document_title_should_be(
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
pub(crate) fn the_profile_languages_should_include(
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
pub(crate) fn the_profile_speakers_should_include(
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
pub(crate) fn the_header_should_record_an_annotation_system(
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
pub(crate) fn the_header_should_record_the_revision_note(
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
pub(crate) fn header_validation_fails_with(
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
