Feature: Title serialisation

  Scenario: Serialise a valid document title
    Given a document title "The Magnus Archives"
    When I serialise the document title
    Then the XML output is "<title>The Magnus Archives</title>"

  Scenario: Reject an empty document title
    Given no document title
    When I attempt to build the document
    Then title creation fails with "document title may not be empty"
