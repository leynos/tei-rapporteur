//! Stand-off specific validation steps and helpers.
//!
//! This module serves the parent `validation_behaviour` test module by adding
//! stand-off setup steps and scenarios that share the parent validation state.

use super::*;
use tei_core::{AnnotationSystem, EncodingDesc, PointerList, Span, SpanGroup, StandOff};

#[given("the encoding includes annotation system \"{identifier}\"")]
fn the_encoding_includes_annotation_system(
    #[from(validated_state)] state: &ValidationState,
    identifier: String,
) -> Result<()> {
    add_annotation_system_step(state, &identifier)
}

#[given("the encoding also includes annotation system \"{identifier}\"")]
fn the_encoding_also_includes_annotation_system(
    #[from(validated_state)] state: &ValidationState,
    identifier: String,
) -> Result<()> {
    add_annotation_system_step(state, &identifier)
}

#[when("I add a stand-off span group \"{kind}\" with id \"{identifier}\"")]
fn i_add_a_stand_off_span_group(
    #[from(validated_state)] state: &ValidationState,
    kind: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| add_span_group(document, &kind, &identifier))
}

#[when("I add a stand-off span \"{span_id}\" in group \"{group_id}\" targeting \"{target}\"")]
fn i_add_a_stand_off_span_targeting(
    #[from(validated_state)] state: &ValidationState,
    span_id: String,
    group_id: String,
    target: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut span = Span::new();
        span.set_id(span_id.as_str())
            .context("span id should validate")?;
        span.set_target(
            PointerList::new([target.as_str()]).context("target pointers should validate")?,
        );
        add_span_to_group(document, &group_id, span)
    })
}

#[when("I add an anchorless stand-off span \"{span_id}\" in group \"{group_id}\"")]
fn i_add_an_anchorless_stand_off_span(
    #[from(validated_state)] state: &ValidationState,
    span_id: String,
    group_id: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut span = Span::new();
        span.set_id(span_id.as_str())
            .context("span id should validate")?;
        add_span_to_group(document, &group_id, span)
    })
}

fn add_annotation_system_step(state: &ValidationState, identifier: &str) -> Result<()> {
    state.update_document(|document| add_annotation_system(document, identifier, "annotations"))
}

fn add_annotation_system(
    document: &TeiDocument,
    identifier: &str,
    description: &str,
) -> Result<TeiDocument> {
    let mut encoding = document
        .header()
        .encoding_desc()
        .cloned()
        .unwrap_or_else(EncodingDesc::new);
    let system = AnnotationSystem::new(identifier, description)
        .context("annotation system should validate")?;
    encoding.add_annotation_system(system);

    let header = document.header().clone().with_encoding_desc(encoding);

    Ok(TeiDocument::new(header, document.text().clone()))
}

fn add_span_group(document: &TeiDocument, kind: &str, identifier: &str) -> Result<TeiDocument> {
    let mut stand_off = document.stand_off().cloned().unwrap_or_else(StandOff::new);
    let mut span_group = SpanGroup::new(kind)?;
    span_group
        .set_id(identifier)
        .context("span group id should validate")?;
    stand_off.add_span_group(span_group);

    Ok(
        TeiDocument::new(document.header().clone(), document.text().clone())
            .with_stand_off(stand_off),
    )
}

fn add_span_to_group(document: &TeiDocument, group_id: &str, span: Span) -> Result<TeiDocument> {
    let mut stand_off = document.stand_off().cloned().unwrap_or_else(StandOff::new);
    let span_group = stand_off
        .find_span_group_mut(group_id)
        .context("span group should exist before adding a span")?;
    span_group.add_span(span);

    Ok(
        TeiDocument::new(document.header().clone(), document.text().clone())
            .with_stand_off(stand_off),
    )
}

#[scenario(
    path = "tests/features/validation.feature",
    name = "Accepting stand-off spans that target existing utterances"
)]
fn accepts_stand_off_spans_that_target_existing_utterances(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(
    path = "tests/features/validation.feature",
    name = "Rejecting stand-off spans that target missing ids"
)]
fn rejects_stand_off_spans_that_target_missing_ids(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}

#[scenario(
    path = "tests/features/validation.feature",
    name = "Rejecting stand-off spans without anchors"
)]
fn rejects_stand_off_spans_without_anchors(
    #[from(validated_state)] _: ValidationState,
    #[from(validated_state_result)] validated_state: Result<ValidationState>,
) {
    expect_validated_state(validated_state, "validation");
}
