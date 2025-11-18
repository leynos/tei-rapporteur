Feature: tei_rapporteur Python module

  Scenario: Construct a Document from a valid title
    Given the tei_rapporteur Python module is initialised
    When I construct a Document titled "Wolf 359"
    Then the document title equals "Wolf 359"

  Scenario: Reject blank titles in the Python constructor
    Given the tei_rapporteur Python module is initialised
    When I construct a Document titled "   "
    Then construction fails mentioning "document title may not be empty"

  Scenario: Emit title markup via the Python helper
    Given the tei_rapporteur Python module is initialised
    When I emit title markup for "Archive 81"
    Then the markup equals "<title>Archive 81</title>"

  Scenario: Document method escapes XML special characters
    Given the tei_rapporteur Python module is initialised
    When I construct a Document with the XML special characters fixture
    And I emit markup from the constructed Document
    Then the markup equals "<title>Special &lt;Title&gt; &amp; &quot;Quotes&quot; and &apos;Apostrophes&apos;</title>"

  Scenario: Deserialize a Document from MessagePack bytes
    Given the tei_rapporteur Python module is initialised
    And I encode a MessagePack document titled "Bridgewater"
    When I decode the MessagePack payload
    Then the document title equals "Bridgewater"

  Scenario: Reject invalid MessagePack payloads
    Given the tei_rapporteur Python module is initialised
    And I provide an invalid MessagePack payload
    When I decode the MessagePack payload
    Then construction fails mentioning "invalid MessagePack payload"

  Scenario: Reject MessagePack payloads missing required fields
    Given the tei_rapporteur Python module is initialised
    And I encode a MessagePack document missing required fields
    When I decode the MessagePack payload
    Then construction fails mentioning "missing field"

  Scenario: Encode a Document to MessagePack bytes
    Given the tei_rapporteur Python module is initialised
    When I construct a Document titled "Bridgewater"
    And I encode the constructed Document to MessagePack
    Then decoding the MessagePack payload yields a Document titled "Bridgewater"

  Scenario: Reject to_msgpack when a Document is not provided
    Given the tei_rapporteur Python module is initialised
    When I encode MessagePack without providing a Document
    Then construction fails mentioning "cannot be converted to 'Document'"
