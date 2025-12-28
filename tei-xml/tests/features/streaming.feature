Feature: Streaming TEI parsing

  The streaming parser yields high-level domain events incrementally,
  allowing processing of large TEI documents without loading the entire
  content into memory.

  Scenario: Parse minimal document incrementally
    Given a streaming parser for the "minimal" TEI fixture
    When I collect all events
    Then the event sequence is "DocumentStart, Header, DocumentEnd"

  Scenario: Yield paragraphs as body blocks
    Given a streaming parser for the "paragraphs" TEI fixture
    When I collect all events
    Then I receive 2 BodyBlock events
    And each BodyBlock is a Paragraph

  Scenario: Yield utterances with speaker attribution
    Given a streaming parser for the "utterances" TEI fixture
    When I collect all events
    Then I receive 2 BodyBlock events
    And each BodyBlock is an Utterance with a speaker

  Scenario: Parse inline emphasis correctly
    Given a streaming parser for the "emphasis" TEI fixture
    When I collect all events
    Then the first paragraph contains emphasis

  Scenario: Parse pause markers correctly
    Given a streaming parser for the "pause" TEI fixture
    When I collect all events
    Then the first utterance contains a pause

  Scenario: Header is accessible after parsing
    Given a streaming parser for the "minimal" TEI fixture
    When I consume up to the Header event
    Then the parser header method returns the same header

  Scenario: Report malformed XML during streaming
    Given a streaming parser for the "unterminated" TEI fixture
    When I request the next event after DocumentStart
    Then an error is returned

  Scenario: Report missing header
    Given a streaming parser for the "missing-header" TEI fixture
    When I collect all events
    Then an error is returned mentioning "unexpected end"

  Scenario: Handle CDATA in body content
    Given a streaming parser for the "cdata" TEI fixture
    When I collect all events
    Then the first paragraph contains CDATA text

  Scenario: Handle EOF after body closes
    Given a streaming parser for the "eof-after-body" TEI fixture
    When I collect all events
    Then the event sequence is "DocumentStart, Header, BodyBlock, DocumentEnd"

  Scenario: Header is None before Header event
    Given a streaming parser for the "minimal" TEI fixture
    When I check header before the Header event
    Then the parser header method returns None before the Header event
