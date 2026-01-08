Feature: Benchmark fixture generation

  The benchmark fixture generator produces TEI documents of varying sizes
  for performance testing. Generated documents must pass validation and
  round-trip through both the full-document and streaming parsers.

  Scenario: Generate a small benchmark fixture
    Given a small benchmark configuration
    When a benchmark document is generated
    Then the document passes validation
    And the document contains 10 utterances
    And the document contains 2 paragraphs

  Scenario: Generate a medium benchmark fixture
    Given a medium benchmark configuration
    When a benchmark document is generated
    Then the document passes validation
    And the document contains 100 utterances
    And the document contains 10 paragraphs

  Scenario: Generate a large benchmark fixture
    Given a large benchmark configuration
    When a benchmark document is generated
    Then the document passes validation
    And the document contains 1000 utterances
    And the document contains 50 paragraphs

  Scenario: Small fixture parses with streaming parser
    Given a small benchmark configuration
    When benchmark XML is generated
    And the XML is parsed with the streaming parser
    Then all events are yielded without errors
    And the body block count matches the configuration

  Scenario: Medium fixture parses with streaming parser
    Given a medium benchmark configuration
    When benchmark XML is generated
    And the XML is parsed with the streaming parser
    Then all events are yielded without errors
    And the body block count matches the configuration

  Scenario: Large fixture parses with streaming parser
    Given a large benchmark configuration
    When benchmark XML is generated
    And the XML is parsed with the streaming parser
    Then all events are yielded without errors
    And the body block count matches the configuration

  Scenario: Small fixture round-trips through full parser
    Given a small benchmark configuration
    When benchmark XML is generated
    And the XML is parsed with the full parser
    Then the parsed document has the same utterance count
    And the parsed document has the same paragraph count

  Scenario: Medium fixture round-trips through full parser
    Given a medium benchmark configuration
    When benchmark XML is generated
    And the XML is parsed with the full parser
    Then the parsed document has the same utterance count
    And the parsed document has the same paragraph count

  Scenario: Large fixture round-trips through full parser
    Given a large benchmark configuration
    When benchmark XML is generated
    And the XML is parsed with the full parser
    Then the parsed document has the same utterance count
    And the parsed document has the same paragraph count
