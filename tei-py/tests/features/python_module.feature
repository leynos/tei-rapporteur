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
