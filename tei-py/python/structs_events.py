"""Streaming-event projections for the embedded Python structs module."""

from __future__ import annotations

import msgspec
from typing import TypeAlias

from _tei_rapporteur_structs_body import DivContent, Head, _validate_div_invariants
from _tei_rapporteur_structs_common import Inline, TeiHeader


class DocumentStart(msgspec.Struct, tag="document_start", tag_field="type"):
    """Streaming event signalling the start of parsing."""


class DocumentEnd(msgspec.Struct, tag="document_end", tag_field="type"):
    """Streaming event signalling parsing completion."""


class HeaderEvent(msgspec.Struct, tag="header", tag_field="type"):
    """Streaming event carrying the parsed header."""

    header: TeiHeader


class ParagraphEvent(
    msgspec.Struct, tag="paragraph", tag_field="type", omit_defaults=True
):
    """Streaming event carrying a paragraph."""

    xml_id: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)


class UtteranceEvent(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Streaming event carrying an utterance."""

    xml_id: str | None = None
    speaker: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)
    n: str | None = None
    source: list[str] = msgspec.field(default_factory=list)
    resp: list[str] = msgspec.field(default_factory=list)
    cert: str | None = None
    corresp: list[str] = msgspec.field(default_factory=list)
    ana: list[str] = msgspec.field(default_factory=list)


class DivEvent(msgspec.Struct, tag="div", tag_field="type", omit_defaults=True):
    """Summary
    -------
    Streaming event carrying an assembled division body block.

    Attributes
    ----------
    div_type : str
        Required TEI ``@type`` value describing the emitted division.
    subtype : str | None
        Optional TEI ``@subtype`` value refining ``div_type``.
    head : Head | None
        Optional heading emitted before the division's child blocks.
    content : list[DivContent]
        Division children collected before the event is yielded.
    xml_id : str | None
        Optional XML identifier preserved from the source document.

    Notes
    -----
    ``DivEvent`` is emitted by streaming parses whenever a complete division is
    assembled, mirroring :class:`DivBlock` in the event union.
    """

    div_type: str
    subtype: str | None = None
    head: Head | None = None
    content: list[DivContent] = msgspec.field(default_factory=list)
    xml_id: str | None = None

    def __post_init__(self) -> None:
        _validate_div_invariants(self.div_type, self.subtype)


Event: TypeAlias = (
    DocumentStart
    | HeaderEvent
    | ParagraphEvent
    | UtteranceEvent
    | DivEvent
    | DocumentEnd
)
