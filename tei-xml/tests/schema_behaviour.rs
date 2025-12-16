//! Behaviour-driven scenarios for Relax NG schema availability.

use anyhow::{Context, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::{
    cell::RefCell,
    path::{Component, Path, PathBuf},
};
use tei_xml::{relax_ng_schema, write_relax_ng_schema};
use tempfile::TempDir;

// Force Cargo to recompile the test binary when the feature file changes so the
// embedded scenarios stay in sync with expectations.
const _: &str = include_str!("features/schema.feature");

type WriteOutcome = std::result::Result<(), String>;

#[derive(Default)]
struct SchemaState {
    output_dir: RefCell<Option<TempDir>>,
    output_path: RefCell<Option<PathBuf>>,
    outcome: RefCell<Option<WriteOutcome>>,
}

impl SchemaState {
    fn set_output_dir(&self, dir: TempDir) {
        *self.output_dir.borrow_mut() = Some(dir);
    }

    fn output_dir_path(&self) -> anyhow::Result<PathBuf> {
        self.output_dir
            .borrow()
            .as_ref()
            .map(|dir| dir.path().to_path_buf())
            .context("scenario must create a temporary output directory")
    }

    fn set_output_path(&self, path: PathBuf) {
        *self.output_path.borrow_mut() = Some(path);
    }

    fn output_path(&self) -> anyhow::Result<PathBuf> {
        self.output_path
            .borrow()
            .as_ref()
            .cloned()
            .context("schema write path must be set before assertions")
    }

    fn set_outcome(&self, outcome: WriteOutcome) {
        *self.outcome.borrow_mut() = Some(outcome);
    }

    fn outcome(&self) -> anyhow::Result<WriteOutcome> {
        self.outcome
            .borrow()
            .as_ref()
            .cloned()
            .context("write_relax_ng_schema must run before assertions")
    }
}

#[fixture]
fn validated_state_result() -> anyhow::Result<SchemaState> {
    let state = SchemaState::default();
    ensure!(
        state.output_dir.borrow().is_none(),
        "output dir slot must start empty"
    );
    ensure!(
        state.output_path.borrow().is_none(),
        "output path slot must start empty"
    );
    ensure!(
        state.outcome.borrow().is_none(),
        "outcome slot must start empty"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> SchemaState {
    tei_test_helpers::expect_validated_state(validated_state_result(), "schema")
}

#[given("a temporary output directory")]
fn a_temporary_output_directory(
    #[from(validated_state)] state: &SchemaState,
) -> anyhow::Result<()> {
    let dir = tempfile::tempdir().context("temp dir should be created")?;
    state.set_output_dir(dir);
    let _ = state.output_dir_path()?;
    Ok(())
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value.
#[when("I write the Relax NG schema to \"{path}\"")]
fn i_write_the_relax_ng_schema_to(
    #[from(validated_state)] state: &SchemaState,
    path: String,
) -> anyhow::Result<()> {
    let dir_path = state.output_dir_path()?;
    let relative = Path::new(&path);
    ensure!(
        !relative.is_absolute(),
        "schema write path must be relative"
    );
    ensure!(
        relative
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "schema write path must not contain traversal components"
    );
    let output_path = dir_path.join(relative);
    let outcome = write_relax_ng_schema(&output_path).map_err(|error| error.to_string());
    state.set_output_path(output_path);
    state.set_outcome(outcome);
    Ok(())
}

#[then("writing succeeds")]
fn writing_succeeds(#[from(validated_state)] state: &SchemaState) -> anyhow::Result<()> {
    let outcome = state.outcome()?;
    if let Err(error) = outcome {
        bail!("expected schema write to succeed: {error}");
    }
    Ok(())
}

#[then("the written schema contains a grammar element")]
fn written_schema_contains_grammar(
    #[from(validated_state)] state: &SchemaState,
) -> anyhow::Result<()> {
    let path = state.output_path()?;
    let written = std::fs::read_to_string(&path)
        .with_context(|| format!("schema should be readable at {}", path.display()))?;
    ensure!(
        written.contains("<grammar"),
        "written schema missing grammar element"
    );
    ensure!(
        written == relax_ng_schema(),
        "written schema should match embedded schema"
    );
    Ok(())
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value.
#[then("writing fails mentioning \"{snippet}\"")]
fn writing_fails_mentioning(
    #[from(validated_state)] state: &SchemaState,
    snippet: String,
) -> anyhow::Result<()> {
    let outcome = state.outcome()?;
    let Err(message) = outcome else {
        bail!("expected schema write to fail");
    };
    ensure!(
        message.contains(&snippet),
        "error should mention {snippet:?}, found {message:?}"
    );
    Ok(())
}

#[scenario(path = "tests/features/schema.feature", index = 0)]
fn writes_schema_successfully(
    #[from(validated_state)] _: SchemaState,
    #[from(validated_state_result)] result: anyhow::Result<SchemaState>,
) {
    tei_test_helpers::expect_validated_state(result, "schema");
}

#[scenario(path = "tests/features/schema.feature", index = 1)]
fn reports_schema_write_errors(
    #[from(validated_state)] _: SchemaState,
    #[from(validated_state_result)] result: anyhow::Result<SchemaState>,
) {
    tei_test_helpers::expect_validated_state(result, "schema");
}
