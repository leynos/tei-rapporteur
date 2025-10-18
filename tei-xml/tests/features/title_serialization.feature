Feature: Title serialization

  Scenario: Serialize a valid document title
    Given a document title "The Magnus Archives"
    When I serialize the document title
    Then the XML output is "<title>The Magnus Archives</title>"

  Scenario: Escape markup-significant characters
    Given a document title "R&D <Test>"
    When I serialize the document title
    Then the XML output is "<title>R&amp;D &lt;Test&gt;</title>"

  Scenario: Reject an empty document title
    Given no document title
    When I attempt to build the document
    Then title creation fails with "document title may not be empty"
