//! Behaviour-driven scenarios for published JSON Schema snapshots.

use anyhow::{Context, Result, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{BodyBlock, Hi, Inline, P, TeiBody, TeiDocument, TeiHeader, TeiText};
use tei_test_helpers::expect_validated_state;

fn compile_validator(schema: &serde_json::Value) -> Result<jsonschema::Validator> {
    jsonschema::validator_for(schema).context("compiled JSON schema must be valid")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedSchemaFailure {
    MissingTeiHeader,
    UnknownHiProperty,
}

#[derive(Clone, Debug)]
struct ValidationErrorRecord {
    message: String,
    instance_path: String,
    schema_path: String,
}

fn collect_validation_errors<'i>(
    errors: impl Iterator<Item = jsonschema::ValidationError<'i>>,
) -> Vec<ValidationErrorRecord> {
    errors
        .map(|error| ValidationErrorRecord {
            message: error.to_string(),
            instance_path: error.instance_path().to_string(),
            schema_path: error.schema_path().to_string(),
        })
        .collect()
}

fn find_hi_object_mut(
    instance: &mut serde_json::Value,
) -> Option<&mut serde_json::Map<String, serde_json::Value>> {
    fn is_hi_object(map: &serde_json::Map<String, serde_json::Value>) -> bool {
        matches!(map.get("@rend"), Some(serde_json::Value::String(_)))
            && matches!(map.get("$value"), Some(serde_json::Value::Array(_)))
            && map.get("@xml:id").is_none()
            && map.get("@who").is_none()
    }

    fn find_in_object_mut(
        map: &mut serde_json::Map<String, serde_json::Value>,
    ) -> Option<&mut serde_json::Map<String, serde_json::Value>> {
        if is_hi_object(map) {
            return Some(map);
        }

        map.values_mut().find_map(find_hi_object_mut)
    }

    fn find_in_array_mut(
        items: &mut [serde_json::Value],
    ) -> Option<&mut serde_json::Map<String, serde_json::Value>> {
        items.iter_mut().find_map(find_hi_object_mut)
    }

    match instance {
        serde_json::Value::Object(map) => find_in_object_mut(map),
        serde_json::Value::Array(items) => find_in_array_mut(items),
        _ => None,
    }
}

#[derive(Default)]
struct JsonSchemaState {
    schema: RefCell<Option<serde_json::Value>>,
    document: RefCell<Option<TeiDocument>>,
    instance: RefCell<Option<serde_json::Value>>,
    validation_errors: RefCell<Option<Vec<ValidationErrorRecord>>>,
    expected_failure: RefCell<Option<ExpectedSchemaFailure>>,
}

impl JsonSchemaState {
    fn set_schema(&self, schema: serde_json::Value) {
        *self.schema.borrow_mut() = Some(schema);
    }

    fn schema(&self) -> Result<std::cell::Ref<'_, serde_json::Value>> {
        std::cell::Ref::filter_map(self.schema.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must configure a JSON Schema"))
    }

    fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
        self.validation_errors.borrow_mut().take();
    }

    fn document(&self) -> Result<std::cell::Ref<'_, TeiDocument>> {
        std::cell::Ref::filter_map(self.document.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must configure a document"))
    }

    fn set_instance(&self, instance: serde_json::Value) {
        *self.instance.borrow_mut() = Some(instance);
        self.validation_errors.borrow_mut().take();
    }

    fn instance(&self) -> Result<std::cell::Ref<'_, serde_json::Value>> {
        std::cell::Ref::filter_map(self.instance.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must configure a JSON instance"))
    }

    fn set_expected_failure(&self, expected_failure: ExpectedSchemaFailure) {
        *self.expected_failure.borrow_mut() = Some(expected_failure);
    }

    fn expected_failure(&self) -> Option<ExpectedSchemaFailure> {
        *self.expected_failure.borrow()
    }

    fn set_validation_errors(&self, errors: Vec<ValidationErrorRecord>) {
        *self.validation_errors.borrow_mut() = Some(errors);
    }

    fn validation_errors(&self) -> Result<std::cell::Ref<'_, Vec<ValidationErrorRecord>>> {
        std::cell::Ref::filter_map(self.validation_errors.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must validate JSON before assertions"))
    }
}

fn build_state() -> Result<JsonSchemaState> {
    let state = JsonSchemaState::default();
    ensure!(
        state.schema.borrow().is_none(),
        "fresh state must not contain schema payloads"
    );
    ensure!(
        state.document.borrow().is_none(),
        "fresh state must not contain documents"
    );
    ensure!(
        state.instance.borrow().is_none(),
        "fresh state must not contain JSON instances"
    );
    ensure!(
        state.validation_errors.borrow().is_none(),
        "fresh state must not contain validation errors"
    );
    ensure!(
        state.expected_failure.borrow().is_none(),
        "fresh state must not contain expected failure state"
    );
    Ok(state)
}

#[fixture]
fn validated_state_result() -> Result<JsonSchemaState> {
    build_state()
}

#[fixture]
fn validated_state() -> JsonSchemaState {
    expect_validated_state(build_state(), "json schema")
}

#[given("the TeiDocument JSON Schema")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "rstest-bdd step signatures stay uniform even when storing literals"
)]
fn the_tei_document_json_schema(#[from(validated_state)] state: &JsonSchemaState) -> Result<()> {
    let schema = tei_serde::schema::tei_document_schema().to_value();
    state.set_schema(schema);
    Ok(())
}

#[given("a TEI document titled \"{title}\"")]
fn a_tei_document_titled(
    #[from(validated_state)] state: &JsonSchemaState,
    title: String,
) -> Result<()> {
    let document =
        TeiDocument::from_title_str(title.as_str()).context("fixture document must construct")?;
    state.set_document(document);
    Ok(())
}

#[given("a JSON instance missing the TEI header")]
fn a_json_instance_missing_the_tei_header(
    #[from(validated_state)] state: &JsonSchemaState,
) -> Result<()> {
    let document =
        TeiDocument::from_title_str("fixture").context("fixture document must construct")?;
    let mut instance =
        tei_serde::json::to_value(&document).context("fixture document must serialize to JSON")?;

    let Some(object) = instance.as_object_mut() else {
        return Err(anyhow::anyhow!(
            "expected document to serialize into a JSON object"
        ));
    };

    object.remove("teiHeader");
    state.set_instance(instance);
    state.set_expected_failure(ExpectedSchemaFailure::MissingTeiHeader);
    Ok(())
}

#[given("a JSON instance containing a hi node with an unknown property")]
fn a_json_instance_containing_a_hi_node_with_an_unknown_property(
    #[from(validated_state)] state: &JsonSchemaState,
) -> Result<()> {
    let file_desc = tei_core::FileDesc::from_title_str("fixture").context("file description")?;
    let header = TeiHeader::new(file_desc);

    let hi =
        Hi::try_with_rend("emph", [Inline::text("hello")]).context("hi content must validate")?;
    let paragraph = P::from_inline([Inline::Hi(hi)]).context("paragraph must validate")?;
    let body = TeiBody::new([BodyBlock::Paragraph(paragraph)]);
    let text = TeiText::new(body);
    let document = TeiDocument::new(header, text);

    let mut instance =
        tei_serde::json::to_value(&document).context("fixture document must serialize to JSON")?;

    let Some(object) = find_hi_object_mut(&mut instance) else {
        return Err(anyhow::anyhow!(
            "expected JSON instance to contain a hi node"
        ));
    };
    object.insert(
        "unknown".to_owned(),
        serde_json::Value::String("field".to_owned()),
    );

    state.set_instance(instance);
    state.set_expected_failure(ExpectedSchemaFailure::UnknownHiProperty);
    Ok(())
}

#[when("I validate the document JSON against the schema")]
fn i_validate_the_document_json_against_the_schema(
    #[from(validated_state)] state: &JsonSchemaState,
) -> Result<()> {
    let document = state.document()?;
    let document_instance =
        tei_serde::json::to_value(&*document).context("document must serialize to JSON")?;
    state.set_instance(document_instance);

    let schema = state.schema()?;
    let validator = compile_validator(&schema)?;
    let instance = state.instance()?;
    state.set_validation_errors(collect_validation_errors(validator.iter_errors(&instance)));

    Ok(())
}

#[when("I validate the JSON instance against the schema")]
fn i_validate_the_json_instance_against_the_schema(
    #[from(validated_state)] state: &JsonSchemaState,
) -> Result<()> {
    let schema = state.schema()?;
    let validator = compile_validator(&schema)?;
    let instance = state.instance()?;

    state.set_validation_errors(collect_validation_errors(validator.iter_errors(&instance)));

    Ok(())
}

#[then("schema validation succeeds")]
fn schema_validation_succeeds(#[from(validated_state)] state: &JsonSchemaState) -> Result<()> {
    let errors = state.validation_errors()?;
    ensure!(
        errors.is_empty(),
        "expected validation to succeed, got {errors:?}"
    );
    Ok(())
}

#[then("schema validation fails")]
fn schema_validation_fails(#[from(validated_state)] state: &JsonSchemaState) -> Result<()> {
    let errors = state.validation_errors()?;
    ensure!(
        !errors.is_empty(),
        "expected validation to fail but no errors were recorded"
    );

    match state.expected_failure() {
        Some(ExpectedSchemaFailure::MissingTeiHeader) => ensure!(
            errors
                .iter()
                .any(|error| error.message.contains("teiHeader")
                    && error.schema_path.contains("/required")),
            "expected at least one validation error to mention missing `teiHeader`, got {errors:?}"
        ),
        Some(ExpectedSchemaFailure::UnknownHiProperty) => ensure!(
            errors.iter().any(|error| {
                error.message.contains("unknown")
                    && (error.message.contains("additional")
                        || error.schema_path.contains("additionalProperties")
                        || error.schema_path.contains("/oneOf")
                        || error.instance_path.contains("/unknown"))
            }),
            "expected at least one validation error to mention unknown property on `hi`, got {errors:?}"
        ),
        None => {}
    }

    Ok(())
}

/// Scenario: Validate a serialized document against the schema.
#[scenario(path = "tests/features/json_schema.feature", index = 0)]
pub fn validates_serialized_document(
    #[from(validated_state)] _: JsonSchemaState,
    #[from(validated_state_result)] result: Result<JsonSchemaState>,
) {
    expect_validated_state(result, "json schema");
}

/// Scenario: Reject JSON instances missing required properties.
#[scenario(path = "tests/features/json_schema.feature", index = 1)]
pub fn rejects_instances_missing_required_properties(
    #[from(validated_state)] _: JsonSchemaState,
    #[from(validated_state_result)] result: Result<JsonSchemaState>,
) {
    expect_validated_state(result, "json schema");
}

/// Scenario: Reject hi nodes with unknown properties.
#[scenario(path = "tests/features/json_schema.feature", index = 2)]
pub fn rejects_hi_nodes_with_unknown_properties(
    #[from(validated_state)] _: JsonSchemaState,
    #[from(validated_state_result)] result: Result<JsonSchemaState>,
) {
    expect_validated_state(result, "json schema");
}
