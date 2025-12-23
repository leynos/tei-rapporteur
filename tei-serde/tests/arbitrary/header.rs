//! Proptest strategies for TEI header components.
//!
//! Generates `TeiHeader` with `FileDesc`, `ProfileDesc`, `EncodingDesc`, and
//! `RevisionDesc` respecting all validation constraints.

use proptest::prelude::*;
use tei_core::{
    AnnotationSystem, EncodingDesc, FileDesc, ProfileDesc, RevisionChange, RevisionDesc, TeiHeader,
};

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
            let mut fd = FileDesc::from_title_str(&title)
                .unwrap_or_else(|error| panic!("generated title should be valid: {error}"));
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
                pd.add_speaker(&s)
                    .unwrap_or_else(|error| panic!("speaker should validate: {error}"));
            }
            for l in languages {
                pd.add_language(&l)
                    .unwrap_or_else(|error| panic!("language should validate: {error}"));
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
                    .unwrap_or_else(|error| panic!("annotation system should validate: {error}"));
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
                    .unwrap_or_else(|error| panic!("revision change should validate: {error}"));
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
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    #[test]
    fn file_desc_strategy_produces_valid_titles() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let fd = file_desc_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            assert!(
                !fd.title().as_str().trim().is_empty(),
                "title must not be empty"
            );
        }
    }

    #[test]
    fn tei_header_strategy_produces_valid_headers() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let header = tei_header_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            assert!(
                !header.file_desc().title().as_str().trim().is_empty(),
                "header title must not be empty"
            );
        }
    }
}
