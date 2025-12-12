Feature: Provide a Relax NG schema

  Scenario: Write the embedded schema to disk
    Given a temporary output directory
    When I write the Relax NG schema to "tei-episodic-profile.rng"
    Then writing succeeds
    And the written schema contains a grammar element

  Scenario: Report errors when the output path is invalid
    Given a temporary output directory
    When I write the Relax NG schema to "missing-parent/tei-episodic-profile.rng"
    Then writing fails mentioning "failed to write Relax NG schema"

