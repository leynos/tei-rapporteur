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

  Scenario: Parse TEI with citation declarations and stand-off annotations
    Given the TEI fixture "annotated"
    When I parse the TEI input
    Then parsing succeeds
    And the parsed document includes stand-off annotations and citation declarations

  Scenario: Parse TEI with nested divisions, headings, and subtypes
    Given the TEI fixture "nested-div"
    When I parse the TEI input
    Then parsing succeeds
    And the parsed document includes nested divisions with headings and subtypes

  Scenario: Parse guest biographies with external reference bindings
    Given the TEI fixture "guest-bios"
    When I parse the TEI input
    Then parsing succeeds
    And the parsed document includes guest bios linked to an external reference revision
    And the emitted guest-bios XML round-trips cleanly
