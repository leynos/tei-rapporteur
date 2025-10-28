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

  Scenario: Recording emphasised inline content
    Given an empty TEI body
    When I add a paragraph emphasising "Critical"
    Then the body should report 1 blocks
    And block 1 should emphasise "Critical"

  Scenario: Recording a pause inline
    Given an empty TEI body
    When I add an utterance for "host" with a pause cue
    Then the body should report 1 blocks
    And block 1 should include a pause inline

  Scenario: Rejecting empty utterance content
    Given an empty TEI body
    When I attempt to record an utterance for "guest" saying "   "
    Then body validation fails with "utterance segments may not be empty"

  Scenario: Rejecting empty paragraph content
    Given an empty TEI body
    When I attempt to add a paragraph containing "   "
    Then body validation fails with "paragraph segments may not be empty"

  Scenario: Rejecting whitespace in paragraph identifiers
    Given an empty TEI body
    When I attempt to set paragraph identifier to "identifier with space"
    Then body validation fails with "paragraph identifiers must not contain whitespace"

  Scenario: Rejecting blank speaker reference
    Given an empty TEI body
    When I attempt to record an utterance for "   " saying "Hello"
    Then body validation fails with "speaker references must not be empty"

  Scenario: Rejecting whitespace in utterance identifiers
    Given an empty TEI body
    When I attempt to set utterance identifier to "identifier with space"
    Then body validation fails with "utterance identifiers must not contain whitespace"

  Scenario: Rejecting empty inline emphasis
    Given an empty TEI body
    When I attempt to add a paragraph emphasising "   "
    Then body validation fails with "paragraph segments may not be empty"

  Scenario: Recording mixed inline content
    Given an empty TEI body
    When I add a paragraph mixing "Welcome" with emphasis "back" rendered as "stress"
    Then the body should report 1 blocks
    And block 1 should reflect the mixed inline paragraph

  Scenario: Recording a measured pause inline
    Given an empty TEI body
    When I add an utterance for "host" with a "breath" pause lasting "PT1S"
    Then the body should report 1 blocks
    And block 1 should include the measured pause inline
