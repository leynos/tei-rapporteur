"""msgspec.Struct projections of the TEI data model."""

from __future__ import annotations

import msgspec

from _tei_rapporteur_structs_body import (
    BodyBlock,
    DivBlock,
    DivContent,
    Head,
    Item,
    Label,
    ListBlock,
    TeiBody,
    TeiText,
)
from _tei_rapporteur_structs_common import (
    AnnotationSystem,
    CiteData,
    CiteStructure,
    EncodingDesc,
    FileDesc,
    Inline,
    InlineHi,
    InlinePause,
    InlineText,
    Paragraph,
    ProfileDesc,
    RefsDecl,
    RevisionChange,
    RevisionDesc,
    Span,
    SpanGroup,
    StandOff,
    TeiHeader,
    TextBlock,
    Utterance,
)
from _tei_rapporteur_structs_events import (
    DivEvent,
    DocumentEnd,
    DocumentStart,
    Event,
    HeaderEvent,
    ParagraphEvent,
    UtteranceEvent,
)

__all__ = [
    "AnnotationSystem",
    "BodyBlock",
    "CiteData",
    "CiteStructure",
    "DivBlock",
    "DivContent",
    "DivEvent",
    "DocumentEnd",
    "DocumentStart",
    "EncodingDesc",
    "Episode",
    "Event",
    "FileDesc",
    "Head",
    "HeaderEvent",
    "Inline",
    "InlineHi",
    "InlinePause",
    "InlineText",
    "Item",
    "Label",
    "ListBlock",
    "Paragraph",
    "ParagraphEvent",
    "ProfileDesc",
    "RefsDecl",
    "RevisionChange",
    "RevisionDesc",
    "Span",
    "SpanGroup",
    "SpokenTextSegment",
    "StandOff",
    "TeiBody",
    "TeiHeader",
    "TeiText",
    "TextBlock",
    "Utterance",
    "UtteranceEvent",
]


class Episode(msgspec.Struct, omit_defaults=True):
    """Top-level TEI document projection."""

    header: TeiHeader
    text: TeiText
    stand_off: StandOff | None = None


class SpokenTextSegment(msgspec.Struct, omit_defaults=True):
    """Normalized spoken text segment with source provenance."""

    text: str
    locator: str
    xml_id: str | None = None


for _name in (
    "AnnotationSystem",
    "CiteData",
    "CiteStructure",
    "DivBlock",
    "DivEvent",
    "DocumentEnd",
    "DocumentStart",
    "EncodingDesc",
    "FileDesc",
    "Head",
    "HeaderEvent",
    "InlineHi",
    "InlinePause",
    "InlineText",
    "Item",
    "Label",
    "ListBlock",
    "Paragraph",
    "ParagraphEvent",
    "ProfileDesc",
    "RefsDecl",
    "RevisionChange",
    "RevisionDesc",
    "Span",
    "SpanGroup",
    "StandOff",
    "TeiBody",
    "TeiHeader",
    "TeiText",
    "Utterance",
    "UtteranceEvent",
):
    globals()[_name].__module__ = __name__

del _name
