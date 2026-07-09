//! Proptest strategies for TEI header components.
//!
//! Generates `TeiHeader` with `FileDesc`, `ProfileDesc`, `EncodingDesc`, and
//! `RevisionDesc` respecting all validation constraints.

use proptest::prelude::*;
use tei_core::{
    AnnotationSystem, EncodingDesc, FileDesc, ProfileDesc, RevisionChange, RevisionDesc, TeiHeader,
};

use super::ExpectValid;
use super::primitives::{
    annotation_id_strategy, document_title_strategy, language_tag_strategy, speaker_strategy,
    text_segment_strategy,
};

/// Generates a `FileDesc` with required title and optional series/synopsis.
pub fn file_desc_strategy() -> impl Strategy<Value = FileDesc> {
    (
        document_title_strategy(),
        proptest::option::of(text_segment_strategy()),
        proptest::option::of(text_segment_strategy()),
    )
        .prop_map(|(title, series, synopsis)| {
            let mut fd =
                FileDesc::from_title_str(&title).expect_valid("generated title should be valid");
            if let Some(s) = series {
                fd = fd.with_series(s);
            }
            if let Some(s) = synopsis {
                fd = fd.with_synopsis(s);
            }
            fd
        })
}

/// Generates an optional `ProfileDesc` with speakers and languages.
pub fn profile_desc_strategy() -> impl Strategy<Value = Option<ProfileDesc>> {
    proptest::option::of((
        proptest::option::of(text_segment_strategy()),
        prop::collection::vec(speaker_strategy(), 0..=5),
        prop::collection::vec(language_tag_strategy(), 0..=3),
    ))
    .prop_map(|opt| {
        opt.map(|(synopsis, speakers, languages)| {
            let mut pd = ProfileDesc::new();
            if let Some(s) = synopsis {
                pd = pd.with_synopsis(s);
            }
            for s in speakers {
                pd.add_speaker(&s).expect_valid("speaker should validate");
            }
            for l in languages {
                pd.add_language(&l).expect_valid("language should validate");
            }
            pd
        })
    })
}

/// Generates an optional `EncodingDesc` with annotation systems.
pub fn encoding_desc_strategy() -> impl Strategy<Value = Option<EncodingDesc>> {
    proptest::option::of(prop::collection::vec(
        (
            annotation_id_strategy(),
            proptest::option::of(text_segment_strategy()),
        ),
        0..=3,
    ))
    .prop_map(|opt| {
        opt.map(|systems| {
            let mut ed = EncodingDesc::new();
            for (id, desc) in systems {
                let description = desc.unwrap_or_default();
                let sys = AnnotationSystem::new(&id, &description)
                    .expect_valid("annotation system should validate");
                ed.add_annotation_system(sys);
            }
            ed
        })
    })
}

/// Generates an optional `RevisionDesc` with revision changes.
pub fn revision_desc_strategy() -> impl Strategy<Value = Option<RevisionDesc>> {
    proptest::option::of(prop::collection::vec(
        (
            text_segment_strategy(),
            proptest::option::of(speaker_strategy()),
        ),
        0..=5,
    ))
    .prop_map(|opt| {
        opt.map(|changes| {
            let mut rd = RevisionDesc::new();
            for (desc, resp) in changes {
                let resp_str = resp.as_deref().unwrap_or("");
                let change = RevisionChange::new(&desc, resp_str)
                    .expect_valid("revision change should validate");
                rd.add_change(change);
            }
            rd
        })
    })
}

/// Generates a `TeiHeader` with all optional sections.
pub fn tei_header_strategy() -> impl Strategy<Value = TeiHeader> {
    (
        file_desc_strategy(),
        profile_desc_strategy(),
        encoding_desc_strategy(),
        revision_desc_strategy(),
    )
        .prop_map(|(file_desc, profile, encoding, revision)| {
            let mut header = TeiHeader::new(file_desc);
            if let Some(p) = profile {
                header = header.with_profile_desc(p);
            }
            if let Some(e) = encoding {
                header = header.with_encoding_desc(e);
            }
            if let Some(r) = revision {
                header = header.with_revision_desc(r);
            }
            header
        })
}

#[cfg(test)]
mod tests {
    //! Tests for header strategies.
    use super::*;
    use crate::arbitrary::test_utils::assert_strategy_produces_valid_values;

    #[test]
    fn file_desc_strategy_produces_valid_titles() {
        assert_strategy_produces_valid_values(file_desc_strategy(), |fd| {
            !fd.title().as_str().trim().is_empty()
        });
    }

    #[test]
    fn tei_header_strategy_produces_valid_headers() {
        assert_strategy_produces_valid_values(tei_header_strategy(), |header| {
            !header.file_desc().title().as_str().trim().is_empty()
        });
    }
}
