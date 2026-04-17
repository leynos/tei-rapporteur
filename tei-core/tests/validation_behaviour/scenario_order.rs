//! Guard test that keeps BDD scenario indices aligned with the feature file.

#[test]
fn validation_feature_scenario_order_matches_expectations() {
    use gherkin::Feature;

    let feature = Feature::parse_path(
        "tests/features/validation.feature",
        gherkin::GherkinEnv::default(),
    )
    .expect("parse validation.feature");
    let names: Vec<&str> = feature
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .collect();

    let expected = [
        "Accepting unique ids and declared speakers",
        "Rejecting duplicate xml:id values",
        "Rejecting header and body identifier clashes",
        "Rejecting duplicate header annotation system identifiers",
        "Rejecting unknown speaker references",
        "Rejecting speakers when the cast is empty",
        "Allowing speakers when the cast list is absent",
        "Accepting stand-off spans that target existing utterances",
        "Rejecting stand-off spans that target missing ids",
        "Rejecting stand-off spans without anchors",
        "Rejecting duplicate xml:id values inside divisions",
        "Rejecting unresolved item corresp pointers inside divisions",
        "Rejecting duplicate xml:id values inside nested divisions",
        "Rejecting unresolved item corresp pointers inside nested divisions",
    ];

    assert_eq!(
        names, expected,
        "Scenario indices in validation_behaviour.rs must stay aligned with validation.feature"
    );
}
