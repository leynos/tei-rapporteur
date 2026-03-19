"""msgspec.Struct projections of the TEI data model (tagged unions).

The projection mirrors the Rust-side Python-facing representation rather than
the canonical XML/serde layout. Inline content is fully typed using internally
tagged unions so `msgspec` can materialise precise Python objects without
falling back to `Any`. TEI pointer-list attributes are exposed as `list[str]`
values, while stand-off annotations and citation declarations use snake_case
field names throughout the Python surface.
"""

from __future__ import annotations

import msgspec
from typing import TypeAlias

__all__ = [
    "AnnotationSystem",
    "BodyBlock",
    "CiteData",
    "CiteStructure",
    "DocumentEnd",
    "DocumentStart",
    "EncodingDesc",
    "Episode",
    "Event",
    "FileDesc",
    "HeaderEvent",
    "Inline",
    "InlineHi",
    "InlinePause",
    "InlineText",
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
]


class InlineText(msgspec.Struct, tag="text", tag_field="type"):
    """Plain text inline node."""

    value: str


class InlineHi(msgspec.Struct, tag="hi", tag_field="type", omit_defaults=True):
    """Emphasised inline span."""

    content: list[Inline] = msgspec.field(default_factory=list)
    rend: str | None = None


class InlinePause(msgspec.Struct, tag="pause", tag_field="type", omit_defaults=True):
    """Pause marker corresponding to ``<pause/>``."""

    dur: str | None = None
    kind: str | None = None


Inline: TypeAlias = InlineText | InlineHi | InlinePause


class Paragraph(
    msgspec.Struct, tag="paragraph", tag_field="type", omit_defaults=True
):
    """Paragraph block (``<p>``) containing inline content."""

    xml_id: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)


class Utterance(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Spoken utterance (``<u>``) with local provenance metadata."""

    xml_id: str | None = None
    n: str | None = None
    speaker: str | None = None
    source: list[str] = msgspec.field(default_factory=list)
    resp: list[str] = msgspec.field(default_factory=list)
    cert: str | None = None
    corresp: list[str] = msgspec.field(default_factory=list)
    ana: list[str] = msgspec.field(default_factory=list)
    content: list[Inline] = msgspec.field(default_factory=list)


BodyBlock: TypeAlias = Paragraph | Utterance


class TeiBody(msgspec.Struct):
    """Ordered TEI body content."""

    blocks: list[BodyBlock] = msgspec.field(default_factory=list)


class TeiText(msgspec.Struct):
    """Text node containing the TEI body."""

    body: TeiBody


class RevisionChange(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Single revision note within ``<revisionDesc>``."""

    description: str = msgspec.field(name="desc")
    resp: str | None = msgspec.field(default=None, name="resp")


class RevisionDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Revision history container."""

    changes: list[RevisionChange] = msgspec.field(
        default_factory=list, name="change"
    )


class AnnotationSystem(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Metadata describing an annotation system."""

    xml_id: str
    desc: str | None = msgspec.field(default=None, name="desc")


class CiteData(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Property extraction entry within a citation structure."""

    property: str
    use_expr: str | None = None


class CiteStructure(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Canonical citation declaration entry."""

    match_expr: str
    unit: str | None = None
    use_expr: str | None = None
    delim: str | None = None
    cite_data: list[CiteData] = msgspec.field(default_factory=list)
    cite_structures: list[CiteStructure] = msgspec.field(default_factory=list)


class RefsDecl(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Container for canonical citation declarations."""

    cite_structures: list[CiteStructure] = msgspec.field(default_factory=list)


class EncodingDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Collection of annotation systems and citation declarations."""

    annotation_systems: list[AnnotationSystem] = msgspec.field(default_factory=list)
    refs_decl: RefsDecl | None = None


class ProfileDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Audience and linguistic profile metadata."""

    synopsis: str | None = None
    speakers: list[str] = msgspec.field(default_factory=list, name="speakers")
    languages: list[str] = msgspec.field(default_factory=list, name="languages")


class FileDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Bibliographic file description."""

    title: str
    series: str | None = None
    synopsis: str | None = None


class TeiHeader(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Aggregated TEI header sections."""

    file_desc: FileDesc = msgspec.field(name="file_desc")
    profile_desc: ProfileDesc | None = msgspec.field(
        default=None, name="profile_desc"
    )
    encoding_desc: EncodingDesc | None = msgspec.field(
        default=None, name="encoding_desc"
    )
    revision_desc: RevisionDesc | None = msgspec.field(
        default=None, name="revision_desc"
    )


class Span(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Stand-off span with many-to-many or range-based targets."""

    xml_id: str | None = None
    target: list[str] = msgspec.field(default_factory=list)
    from_ref: str | None = msgspec.field(default=None, name="from")
    to_ref: str | None = msgspec.field(default=None, name="to")
    source: list[str] = msgspec.field(default_factory=list)
    resp: list[str] = msgspec.field(default_factory=list)
    cert: str | None = None
    corresp: list[str] = msgspec.field(default_factory=list)
    ana: list[str] = msgspec.field(default_factory=list)


class SpanGroup(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Logical stand-off grouping of related spans."""

    xml_id: str | None = None
    kind: str
    resp: list[str] = msgspec.field(default_factory=list)
    corresp: list[str] = msgspec.field(default_factory=list)
    ana: list[str] = msgspec.field(default_factory=list)
    spans: list[Span] = msgspec.field(default_factory=list)


class StandOff(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Root stand-off annotation layer."""

    span_groups: list[SpanGroup] = msgspec.field(default_factory=list)


class Episode(msgspec.Struct):
    """Top-level TEI document."""

    header: TeiHeader
    stand_off: StandOff | None = None
    text: TeiText


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
    n: str | None = None
    speaker: str | None = None
    source: list[str] = msgspec.field(default_factory=list)
    resp: list[str] = msgspec.field(default_factory=list)
    cert: str | None = None
    corresp: list[str] = msgspec.field(default_factory=list)
    ana: list[str] = msgspec.field(default_factory=list)
    content: list[Inline] = msgspec.field(default_factory=list)


Event: TypeAlias = (
    DocumentStart | HeaderEvent | ParagraphEvent | UtteranceEvent | DocumentEnd
)
