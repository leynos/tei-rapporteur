Feature: Python streaming parser

  Scenario: Minimal document yields start, header, and end events
    Given the tei_rapporteur Python module is initialized for streaming
    And the minimal TEI fixture
    When I stream parse the events
    Then the event sequence is "document_start, header, document_end"

  Scenario: Paragraph events carry tagged inline content
    Given the tei_rapporteur Python module is initialized for streaming
    And the paragraph TEI fixture
    When I stream parse the events
    Then a paragraph event is emitted with inline tags

  Scenario: Utterance events carry speaker metadata and tagged inline content
    Given the tei_rapporteur Python module is initialized for streaming
    And the utterance TEI fixture
    When I stream parse the events
    Then an utterance event is emitted with speaker "host"
    And the utterance event exposes provenance metadata

  Scenario: Header event exposes the document title
    Given the tei_rapporteur Python module is initialized for streaming
    And the minimal TEI fixture
    When I stream parse the events
    Then the header event title equals "Wolf 359"

  Scenario: Malformed XML raises ValueError and then exhausts
    Given the tei_rapporteur Python module is initialized for streaming
    And the malformed TEI fixture
    When I stream parse the events
    Then streaming fails mentioning "XML processing error"
    And the iterator is exhausted after the error

  Scenario: Missing header raises ValueError and then exhausts
    Given the tei_rapporteur Python module is initialized for streaming
    And the headerless TEI fixture
    When I stream parse the events
    Then streaming fails mentioning "unexpected end of document"
    And the iterator is exhausted after the error

  Scenario: Events decode into published structs
    Given the tei_rapporteur Python module is initialized for streaming
    And the paragraph TEI fixture
    When I stream parse the events
    Then all events decode into msgspec Event instances
