//! Speaker-reference validation helpers.

use std::collections::HashSet;

use crate::{TeiDocument, Utterance};

use super::ValidationError;

pub(super) fn extract_known_speakers(document: &TeiDocument) -> Option<HashSet<String>> {
    document.header().profile_desc().map(|profile| {
        profile
            .speakers()
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect::<HashSet<_>>()
    })
}

pub(super) fn validate_speaker_reference(
    utterance: &Utterance,
    known_speakers: Option<&HashSet<String>>,
) -> Result<(), ValidationError> {
    let Some(speakers) = known_speakers else {
        return Ok(());
    };
    let Some(speaker) = utterance.speaker() else {
        return Ok(());
    };

    if speakers.contains(speaker.as_str()) {
        Ok(())
    } else {
        Err(ValidationError::UnknownSpeaker {
            speaker: speaker.as_str().to_owned(),
        })
    }
}
