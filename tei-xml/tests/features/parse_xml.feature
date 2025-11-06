Feature: Parse TEI XML

  Scenario: Parse a valid TEI document
    Given the TEI fixture "minimal"
    When I parse the TEI input
    Then parsing succeeds
    And the parsed title is "Wolf 359"

  Scenario: Surface structural errors for missing headers
    Given the TEI fixture "missing-header"
    When I parse the TEI input
    Then parsing fails mentioning "teiHeader"

  Scenario: Surface syntax errors for malformed XML
    Given the TEI fixture "unterminated"
    When I parse the TEI input
    Then parsing fails mentioning "start tag not closed"

  Scenario: Reject blank titles via constructors
    Given the TEI fixture "blank-title"
    When I parse the TEI input
    Then parsing fails mentioning "document title may not be empty"
