Feature: Emit TEI XML

  Scenario: Canonicalise pretty-printed TEI
    Given the TEI fixture "pretty-with-ids"
    When I parse the TEI input
    And I emit the parsed document
    Then emission succeeds
    And the XML matches the canonical fixture "canonical-with-ids"

  Scenario: Preserve xml:id attributes
    Given the TEI fixture "canonical-with-ids"
    When I parse the TEI input
    And I emit the parsed document
    Then emission succeeds
    And the emitted XML contains "xml:id=\"intro\""
    And the emitted XML contains "xml:id=\"u1\""

  Scenario: Reject invalid control characters
    Given a parsed document containing invalid control characters
    When I emit the parsed document
    Then emission fails mentioning "character"
