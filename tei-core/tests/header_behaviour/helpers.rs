use anyhow::{Context, Result};
use tei_core::{EncodingDesc, ProfileDesc, RevisionDesc, TeiDocument};

use crate::HeaderState;

pub(crate) fn expect_document(state: &HeaderState) -> Result<TeiDocument> {
    state.document()
}

pub(crate) fn expect_profile_desc(state: &HeaderState) -> Result<ProfileDesc> {
    expect_document(state)?
        .header()
        .profile_desc()
        .cloned()
        .context("profile metadata should be present")
}

pub(crate) fn expect_encoding_desc(state: &HeaderState) -> Result<EncodingDesc> {
    expect_document(state)?
        .header()
        .encoding_desc()
        .cloned()
        .context("encoding metadata should be present")
}

pub(crate) fn expect_revision_desc(state: &HeaderState) -> Result<RevisionDesc> {
    expect_document(state)?
        .header()
        .revision_desc()
        .cloned()
        .context("revision metadata should be present")
}
