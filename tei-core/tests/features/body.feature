Feature: TEI body content
  The TEI body records narrative paragraphs and spoken utterances in order so
  tooling can replay a script without reordering segments.

  Scenario: Recording a paragraph and an utterance
    Given an empty TEI body
    When I add a paragraph containing "Welcome to the show"
    And I add an utterance for "host" saying "Hello listeners"
    Then the body should report 2 blocks
    And block 1 should be a paragraph with "Welcome to the show"
    And block 2 should be an utterance for "host" with "Hello listeners"

  Scenario: Rejecting empty utterance content
    Given an empty TEI body
    When I attempt to record an utterance for "guest" saying "   "
    Then body validation fails with "utterance content must include at least one non-empty segment"

  Scenario: Rejecting empty paragraph content
    Given an empty TEI body
    When I attempt to add a paragraph containing "   "
    Then body validation fails with "paragraph content must include at least one non-empty segment"

  Scenario: Rejecting whitespace in paragraph identifiers
    Given an empty TEI body
    When I attempt to set paragraph identifier to "identifier with space"
    Then body validation fails with "paragraph identifiers must not contain whitespace"
