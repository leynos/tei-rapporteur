Feature: TEI document serialization

  Scenario: Serialize a document to MessagePack and back
    Given a TEI document titled "Wolf 359"
    When I serialize the document as MessagePack
    And I deserialize the MessagePack payload
    Then the deserialized document title is "Wolf 359"

  Scenario: Reject invalid MessagePack payloads
    Given an invalid MessagePack payload
    When I deserialize the MessagePack payload
    Then MessagePack deserialization fails

  Scenario: Serialize a document to JSON and back
    Given a TEI document titled "Wolf 359"
    When I serialize the document as JSON
    And I deserialize the JSON payload
    Then the deserialized document title is "Wolf 359"

  Scenario: Reject JSON payloads with blank titles
    Given a JSON payload with a blank title
    When I deserialize the JSON payload
    Then JSON deserialization fails

  Scenario: Reject invalid JSON payloads
    Given an invalid JSON payload
    When I deserialize the JSON payload
    Then JSON deserialization fails with a syntax error
