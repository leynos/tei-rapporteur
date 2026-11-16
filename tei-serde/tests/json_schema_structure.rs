//! The published JSON Schema covers the structural body definitions: divisions, lists, items,
//! labels and heads.

mod json_schema_support;

use anyhow::{Context, Result, ensure};
use json_schema_support::{collect_validation_errors, compile_validator};
use tei_core::{
    BodyBlock, Div, DivContent, Head, Item, Label, List, P, TeiBody, TeiDocument, TeiHeader,
    TeiText, Utterance,
};

fn structural_document() -> Result<TeiDocument> {
    let div = structural_show_notes_div(structural_guest_bios_div()?)?;

    Ok(TeiDocument::new(
        TeiHeader::new(
            tei_core::FileDesc::from_title_str("fixture").context("title should validate")?,
        ),
        TeiText::new(TeiBody::new([BodyBlock::Div(div)])),
    ))
}

/// Builds the nested `guest-bio` division: a heading, an utterance and a labelled list.
fn structural_guest_bios_div() -> Result<Div> {
    let mut item = Item::from_text_segments(["Reference"]).context("item should validate")?;
    item.set_label(Label::from_text("1.").context("label should validate")?);
    let list = List::new([item]).context("list should validate")?;
    let mut child = Div::new("segment").context("child div should validate")?;
    child
        .set_subtype("guest-bio")
        .context("child subtype should validate")?;
    child.set_head(Head::from_text("Guest bios").context("head should validate")?);
    child.push_utterance(
        Utterance::from_text_segments(Some("host"), ["Meet our guest"])
            .context("utterance should validate")?,
    );
    child.push_list(list);
    Ok(child)
}

/// Builds the outer `show-notes` division that holds an introduction and `child`.
fn structural_show_notes_div(child: Div) -> Result<Div> {
    let mut div = Div::new("show-notes").context("div should validate")?;
    div.set_subtype("chapter-markers")
        .context("subtype should validate")?;
    div.set_head(Head::from_text("Chapter markers").context("head should validate")?);
    div.push_paragraph(P::from_text_segments(["Intro"]).context("paragraph should validate")?);
    div.push_div(child);
    Ok(div)
}

/// A `oneOf` variant the schema must offer: where the list sits, which property of a variant
/// holds the reference, and what that reference must end with.
struct ExpectedVariant<'a> {
    list_path: &'a str,
    property_path: &'a str,
    ref_suffix: &'a str,
}

fn schema_has_variant(schema: &serde_json::Value, expected: &ExpectedVariant<'_>) -> bool {
    schema
        .pointer(expected.list_path)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|variants| {
            variants.iter().any(|variant| {
                variant
                    .pointer(expected.property_path)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|reference| reference.ends_with(expected.ref_suffix))
            })
        })
}

fn schema_property_references(
    schema: &serde_json::Value,
    property_path: &str,
    expected_ref_suffix: &str,
) -> bool {
    let Some(property) = schema.pointer(property_path) else {
        return false;
    };

    property
        .get("$ref")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|reference| reference.ends_with(expected_ref_suffix))
        || property
            .get("anyOf")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|variants| {
                variants.iter().any(|variant| {
                    variant
                        .get("$ref")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|reference| reference.ends_with(expected_ref_suffix))
                })
            })
}

fn assert_schema_definitions(schema: &serde_json::Value) -> Result<()> {
    for definition in [
        "/$defs/div",
        "/$defs/list",
        "/$defs/item",
        "/$defs/label",
        "/$defs/head",
        "/$defs/DivSubtype",
    ] {
        ensure!(
            schema.pointer(definition).is_some(),
            "schema should define {definition}"
        );
    }
    Ok(())
}

fn assert_schema_variants(schema: &serde_json::Value) -> Result<()> {
    for (list_path, property_path, ref_suffix, message) in [
        (
            "/$defs/BodyBlock/oneOf",
            "/properties/div/$ref",
            "#/$defs/div",
            "BodyBlock schema should include the `div` variant",
        ),
        (
            "/$defs/DivContent/oneOf",
            "/properties/p/$ref",
            "#/$defs/p",
            "DivContent schema should include the `p` variant",
        ),
        (
            "/$defs/DivContent/oneOf",
            "/properties/u/$ref",
            "#/$defs/u",
            "DivContent schema should include the `u` variant",
        ),
        (
            "/$defs/DivContent/oneOf",
            "/properties/list/$ref",
            "#/$defs/list",
            "DivContent schema should include the `list` variant",
        ),
        (
            "/$defs/DivContent/oneOf",
            "/properties/div/$ref",
            "#/$defs/div",
            "DivContent schema should include the nested `div` variant",
        ),
    ] {
        let expected = ExpectedVariant {
            list_path,
            property_path,
            ref_suffix,
        };
        ensure!(schema_has_variant(schema, &expected), "{message}");
    }
    ensure!(
        schema_property_references(
            schema,
            "/$defs/div/properties/@subtype",
            "#/$defs/DivSubtype"
        ),
        "div schema should reference DivSubtype for @subtype"
    );
    ensure!(
        schema_property_references(schema, "/$defs/div/properties/head", "#/$defs/head"),
        "div schema should reference head"
    );
    Ok(())
}

fn assert_document_shape(document: &TeiDocument) -> Result<()> {
    ensure!(
        matches!(
            document.text().body().blocks().first(),
            Some(BodyBlock::Div(Div { .. }))
        ),
        "fixture document should contain a `Div` body block"
    );
    ensure!(
        matches!(
            document
                .text()
                .body()
                .blocks()
                .first()
                .and_then(|block| match block {
                    BodyBlock::Div(division) => division.content().get(1),
                    _ => None,
                }),
            Some(DivContent::Div(_))
        ),
        "fixture division should contain a nested div"
    );
    Ok(())
}

fn assert_instance_shape(instance: &serde_json::Value) -> Result<()> {
    ensure!(
        instance
            .pointer("/text/body/$value/0/div/head")
            .is_some_and(|head| !head.is_null()),
        "serialized document should include the division head"
    );
    ensure!(
        instance
            .pointer("/text/body/$value/0/div/@subtype")
            .is_some_and(|subtype| subtype == "chapter-markers"),
        "serialized document should include the expected division subtype"
    );
    ensure!(
        instance
            .pointer("/text/body/$value/0/div/$value/1/div/head")
            .is_some_and(|head| !head.is_null()),
        "serialized document should include the nested division head"
    );
    ensure!(
        instance
            .pointer("/text/body/$value/0/div/$value/1/div/@subtype")
            .is_some_and(|subtype| subtype == "guest-bio"),
        "serialized document should include the expected nested division subtype"
    );
    ensure!(
        instance
            .pointer("/text/body/$value/0/div/$value/1/div/$value/1/list/$value/0/label")
            .is_some_and(|label| !label.is_null()),
        "serialized document should include the list item label"
    );
    Ok(())
}

#[test]
fn schema_includes_structural_body_definitions() -> Result<()> {
    let document = structural_document()?;
    let schema = tei_serde::schema::tei_document_schema().to_value();
    let validator = compile_validator(&schema)?;
    let instance =
        tei_serde::json::to_value(&document).context("document should serialize to JSON")?;
    let errors = collect_validation_errors(validator.iter_errors(&instance));

    let described: Vec<String> = errors
        .iter()
        .map(|error| {
            format!(
                "{} at {} (schema {})",
                error.message, error.instance_path, error.schema_path
            )
        })
        .collect();
    ensure!(
        errors.is_empty(),
        "document should satisfy schema: {described:?}"
    );
    assert_schema_definitions(&schema)?;
    assert_schema_variants(&schema)?;
    assert_document_shape(&document)?;
    assert_instance_shape(&instance)?;
    Ok(())
}
