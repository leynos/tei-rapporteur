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

  Scenario: Parse TEI XML into a Document
    Given the tei_rapporteur Python module is initialised
    And I provide TEI XML titled "Wolf 359"
    When I parse the TEI XML payload
    Then the document title equals "Wolf 359"

  Scenario: Reject malformed TEI XML payloads
    Given the tei_rapporteur Python module is initialised
    And I provide an invalid TEI XML payload missing the header
    When I parse the TEI XML payload
    Then construction fails mentioning "teiHeader"

  Scenario: Emit TEI XML for a Document
    Given the tei_rapporteur Python module is initialised
    When I construct a Document titled "Wolf 359"
    And I emit the constructed Document to TEI XML
    Then the TEI XML output equals the canonical payload for "Wolf 359"

  Scenario: Reject TEI emission when XML would contain forbidden characters
    Given the tei_rapporteur Python module is initialised
    When I construct a Document with the XML control character fixture
    And I emit the constructed Document to TEI XML
    Then construction fails mentioning "U+0000"

  Scenario: Decode a Document from a dictionary payload
    Given the tei_rapporteur Python module is initialised
    And I provide a dictionary payload titled "Bridgewater"
    When I decode the dictionary payload
    Then the document title equals "Bridgewater"

  Scenario: Reject dictionary payloads missing required fields
    Given the tei_rapporteur Python module is initialised
    And I provide an invalid dictionary payload missing required fields
    When I decode the dictionary payload
    Then construction fails mentioning "missing field"

  Scenario: Reject dictionary payloads with blank titles
    Given the tei_rapporteur Python module is initialised
    And I provide a dictionary payload with a blank title
    When I decode the dictionary payload
    Then construction fails mentioning "document title may not be empty"

  Scenario: Encode a Document to a dictionary payload
    Given the tei_rapporteur Python module is initialised
    When I construct a Document titled "Bridgewater"
    And I encode the constructed Document to a dictionary
    Then the dictionary payload title equals "Bridgewater"

  Scenario: Reject to_dict when a Document is not provided
    Given the tei_rapporteur Python module is initialised
    When I encode a dictionary without providing a Document
    Then construction fails mentioning "cannot be converted to 'Document'"

  Scenario: Round-trip MessagePack via the Episode struct
    Given the tei_rapporteur Python module is initialised
    When I construct a Document titled "Bridgewater"
    And I encode the constructed Document to MessagePack
    And I convert the MessagePack payload to an Episode and retitle it "Bridgewater Remix"
    And I decode the MessagePack payload
    Then the document title equals "Bridgewater Remix"

  Scenario: Report msgspec errors for malformed payloads
    Given the tei_rapporteur Python module is initialised
    And I encode a MessagePack document missing required fields
    When I decode the MessagePack payload to an Episode struct
    Then construction fails mentioning "body"
