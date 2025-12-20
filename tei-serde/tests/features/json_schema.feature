Feature: TeiDocument JSON Schema

  Scenario: Validate a serialized document against the schema
    Given the TeiDocument JSON Schema
    And a TEI document titled "Wolf 359"
    When I validate the document JSON against the schema
    Then schema validation succeeds

  Scenario: Reject JSON instances missing required properties
    Given the TeiDocument JSON Schema
    And a JSON instance missing the TEI header
    When I validate the JSON instance against the schema
    Then schema validation fails

  Scenario: Reject hi nodes with unknown properties
    Given the TeiDocument JSON Schema
    And a JSON instance containing a hi node with an unknown property
    When I validate the JSON instance against the schema
    Then schema validation fails

