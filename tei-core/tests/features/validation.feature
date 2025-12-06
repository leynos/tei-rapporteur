Feature: TEI document validation
  Document-level validation guards unique xml:id values and aligns speaker
  references with the declared cast so downstream tools can trust document
  integrity.

  Scenario: Accepting unique ids and declared speakers
    Given a TEI document titled "Night Vale"
    And the profile includes speaker "host"
    When I add a paragraph "Intro" with id "p1"
    And I add an utterance for "host" saying "Hello" with id "u1"
    And I validate the document
    Then validation succeeds

  Scenario: Rejecting duplicate xml:id values
    Given a TEI document titled "Night Vale"
    And the profile includes speaker "host"
    When I add a paragraph "Intro" with id "dup"
    And I add an utterance for "host" saying "Hello" with id "dup"
    And I validate the document
    Then validation fails with "duplicate xml:id 'dup'"

  Scenario: Rejecting header and body identifier clashes
    Given a TEI document titled "Night Vale"
    And the encoding includes annotation system "shared"
    When I add a paragraph "Intro" with id "shared"
    And I validate the document
    Then validation fails with "duplicate xml:id 'shared'"

  Scenario: Rejecting duplicate header annotation system identifiers
    Given a TEI document titled "Night Vale"
    And the encoding includes annotation system "shared"
    And the encoding also includes annotation system "shared"
    When I validate the document
    Then validation fails with "duplicate xml:id 'shared'"

  Scenario: Rejecting unknown speaker references
    Given a TEI document titled "Night Vale"
    And the profile includes speaker "host"
    When I add an utterance for "guest" saying "Hello" with id "u1"
    And I validate the document
    Then validation fails with "utterance speaker 'guest' is not declared in the profile"

  Scenario: Rejecting speakers when the cast is empty
    Given a TEI document titled "Night Vale"
    And the profile is declared but empty
    When I add an utterance for "ghost" saying "Boo" with id "u1"
    And I validate the document
    Then validation fails with "utterance speaker 'ghost' is not declared in the profile"

  Scenario: Allowing speakers when the cast list is absent
    Given a TEI document titled "Night Vale"
    When I add an utterance for "ghost" saying "Boo" with id "u1"
    And I validate the document
    Then validation succeeds
