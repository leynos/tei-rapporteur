//! Guard tests for validation feature scenario bindings.
//!
//! The `rstest-bdd` scenario macro binds scenarios by their display names.
//! These tests make scenario renames fail loudly instead of silently unbinding
//! a behaviour test from the feature file.

use std::collections::BTreeSet;

const FEATURE_SOURCE: &str = include_str!("../features/validation.feature");
const PARENT_TEST_SOURCE: &str = include_str!("mod.rs");
const STAND_OFF_TEST_SOURCE: &str = include_str!("stand_off.rs");

#[test]
fn validation_feature_scenarios_have_matching_test_bindings() {
    let feature_scenarios = feature_scenario_names(FEATURE_SOURCE);
    let test_bindings = bound_scenario_names([PARENT_TEST_SOURCE, STAND_OFF_TEST_SOURCE]);

    assert_eq!(
        feature_scenarios, test_bindings,
        "validation.feature scenarios must match validation behaviour test \
         bindings exactly by name",
    );
}

fn feature_scenario_names(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("Scenario: "))
        .map(str::to_owned)
        .collect()
}

fn bound_scenario_names<const N: usize>(sources: [&str; N]) -> BTreeSet<String> {
    sources
        .into_iter()
        .flat_map(scenario_names_bound_in_source)
        .collect()
}

fn scenario_names_bound_in_source(source: &str) -> impl Iterator<Item = String> + '_ {
    source.lines().filter_map(|line| {
        line.trim_start()
            .strip_prefix("name = \"")
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_owned)
    })
}
