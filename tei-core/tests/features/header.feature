Feature: TEI header metadata
  Documents expose high level metadata through the TEI header so downstream
  tools can reason about the recording without parsing raw XML.

  Scenario: Assembling a document header
    Given a document title "The Silt Verses"
    And a profile synopsis "Investigative horror podcast"
    And a recording language "en-GB"
    And a cast member "Carpenter"
    And an annotation system "bromide" described as "Cliché detection"
    And a revision change "Initial ingest"
    When I assemble the TEI document
    Then the document title should be "The Silt Verses"
    And the profile languages should include "en-GB"
    And the profile speakers should include "Carpenter"
    And the header should record an annotation system "bromide"
    And the header should record the revision note "Initial ingest"

  Scenario: Rejecting blank revision notes
    Given a document title "Placeholder"
    And an empty revision description
    When I attempt to record the revision
    Then header validation fails with "revision note may not be empty"
