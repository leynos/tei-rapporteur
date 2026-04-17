"""Streaming-event projections for the embedded Python structs module."""

from __future__ import annotations

import msgspec
from typing import TypeAlias

from _tei_rapporteur_structs_body import DivContent, Head, _validate_div_invariants
from _tei_rapporteur_structs_common import Inline, TeiHeader


class DocumentStart(msgspec.Struct, tag="document_start", tag_field="type"):
    """Summary
    -------
    Streaming event emitted when TEI document parsing begins.

    ``DocumentStart`` marks the start of the event stream before any header or
    body content has been yielded.

    Attributes
    ----------
    This event has no public attributes.
    """


class DocumentEnd(msgspec.Struct, tag="document_end", tag_field="type"):
    """Summary
    -------
    Streaming event emitted when TEI document parsing completes.

    ``DocumentEnd`` marks the end of the event stream after all header and body
    content has been yielded.

    Attributes
    ----------
    This event has no public attributes.
    """


class HeaderEvent(msgspec.Struct, tag="header", tag_field="type"):
    """Summary
    -------
    Streaming event carrying the parsed TEI header.

    ``HeaderEvent`` is emitted once the parser has assembled the document
    header.

    Attributes
    ----------
    header : TeiHeader
        Parsed TEI header projection for the current document.
    """

    header: TeiHeader


class ParagraphEvent(
    msgspec.Struct, tag="paragraph", tag_field="type", omit_defaults=True
):
    """Summary
    -------
    Streaming event carrying a paragraph body block.

    ``ParagraphEvent`` is emitted whenever the parser completes a TEI ``<p>``
    element in the body stream.

    Attributes
    ----------
    xml_id : str | None
        Optional XML identifier preserved from ``@xml:id``. Defaults to
        ``None``.
    content : list[Inline]
        Inline nodes contained by the paragraph. Defaults to an empty list.
    """

    xml_id: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)


class UtteranceEvent(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Summary
    -------
    Streaming event carrying an utterance body block.

    ``UtteranceEvent`` is emitted whenever the parser completes a TEI ``<u>``
    element in the body stream.

    Attributes
    ----------
    xml_id : str | None
        Optional XML identifier preserved from ``@xml:id``. Defaults to
        ``None``.
    speaker : str | None
        Optional speaker reference or label from ``@who``. Defaults to
        ``None``.
    content : list[Inline]
        Inline nodes contained by the utterance. Defaults to an empty list.
    n : str | None
        Optional utterance number or label from ``@n``. Defaults to ``None``.
    source : list[str]
        Source pointers copied from ``@source``. Defaults to an empty list.
    resp : list[str]
        Responsibility pointers copied from ``@resp``. Defaults to an empty
        list.
    cert : str | None
        Optional certainty value from ``@cert``. Defaults to ``None``.
    corresp : list[str]
        Correspondence pointers copied from ``@corresp``. Defaults to an empty
        list.
    ana : list[str]
        Analysis pointers copied from ``@ana``. Defaults to an empty list.
    """

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
