"""Python-side smoke tests for the :mod:`tei_rapporteur` bindings.

The suite exercises the public helpers exposed by the freshly built wheel so
packaging mistakes surface immediately. Typical usage mirrors the assertions
below:

>>> import tei_rapporteur as tr
>>> tr.emit_title_markup("Example")
'<title>Example</title>'
"""

import tei_rapporteur as tr

def test_emit_title_markup_helper() -> None:
    """Ensure the module-level helper produces escaped TEI title markup."""

    expected = "<title>Radio Revel</title>"
    actual = tr.emit_title_markup("Radio Revel")
    assert actual == expected, f"Expected {expected!r}, found {actual!r}"


def test_document_round_trip() -> None:
    """Verify the PyO3 `Document` wrapper exposes title accessors and emitters."""

    document = tr.Document("Wolf 359")
    assert document.title == "Wolf 359", "Document should expose the title property"
    expected_markup = "<title>Wolf 359</title>"
    actual_markup = document.emit_title_markup()
    assert (
        actual_markup == expected_markup
    ), f"emit_title_markup should wrap the title, found {actual_markup!r}"
