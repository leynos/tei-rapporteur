Feature: Emit TEI XML

  Scenario: Emit a minimal TEI document
    Given the document title fixture "wolf-359"
    When I emit the TEI document
    Then emitting succeeds
    And the TEI output equals the minimal fixture

  Scenario: Reject control characters during emission
    Given the document title fixture "null-control"
    When I emit the TEI document
    Then emitting fails mentioning "U+0000"
