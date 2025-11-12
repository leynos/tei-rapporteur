"""Python-side smoke tests for the tei_rapporteur module."""

import tei_rapporteur as tr


def test_emit_title_markup_helper() -> None:
    assert tr.emit_title_markup("Radio Revel") == "<title>Radio Revel</title>"


def test_document_round_trip() -> None:
    document = tr.Document("Wolf 359")
    assert document.title == "Wolf 359"
    assert document.emit_title_markup() == "<title>Wolf 359</title>"
