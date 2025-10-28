use anyhow::{Context, Result};
use rstest_bdd_macros::when;
use tei_core::{FileDesc, RevisionChange, TeiDocument, TeiError, TeiHeader, TeiText};

use crate::HeaderState;

#[when("I assemble the TEI document")]
pub(crate) fn i_assemble_the_tei_document(
    #[from(validated_state)] state: &HeaderState,
) -> Result<()> {
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
pub(crate) fn i_attempt_to_record_the_revision(
    #[from(validated_state)] state: &HeaderState,
) -> Result<()> {
    let description = state
        .pending_revision_description()
        .context("scenario must configure the revision attempt")?;
    let attempt = RevisionChange::new(description, "");
    state.set_revision_attempt(attempt);
    Ok(())
}
