use anyhow::{Context, Result, ensure};
use rstest_bdd_macros::given;
use tei_core::{AnnotationSystem, RevisionChange};

use crate::HeaderState;

#[given("a document title \"{title}\"")]
pub(crate) fn a_document_title(
    #[from(validated_state)] state: &HeaderState,
    title: String,
) -> Result<()> {
    state.set_title(title);
    let _ = state.title()?;
    Ok(())
}

#[given("a profile synopsis \"{synopsis}\"")]
pub(crate) fn a_profile_synopsis(
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
pub(crate) fn a_recording_language(
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
pub(crate) fn a_cast_member(
    #[from(validated_state)] state: &HeaderState,
    speaker: String,
) -> Result<()> {
    state
        .profile_mut()
        .add_speaker(speaker)
        .context("speaker should be recorded")?;
    Ok(())
}

#[given("an annotation system \"{identifier}\" described as \"{description}\"")]
pub(crate) fn an_annotation_system(
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
pub(crate) fn a_revision_change(
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
pub(crate) fn an_empty_revision_description(
    #[from(validated_state)] state: &HeaderState,
) -> Result<()> {
    state.set_pending_revision_description(String::new());
    ensure!(
        state.pending_revision_description().as_deref() == Some(""),
        "pending revision description should be staged"
    );
    Ok(())
}
