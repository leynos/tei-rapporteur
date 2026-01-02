"""msgspec.Struct projections of the TEI data model (tagged unions).

The projection mirrors the Rust-side Python-facing representation rather than
the canonical XML/serde layout. Inline content is fully typed using internally
tagged unions so `msgspec` can materialise precise Python objects without
falling back to `Any`. Streaming events share the same tagged shapes, making
`iter_parse` output directly decodable into these structs.
"""

from __future__ import annotations

import msgspec
from typing import TypeAlias

__all__ = [
    "AnnotationSystem",
    "BodyBlock",
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
    "RevisionChange",
    "RevisionDesc",
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
    """Spoken utterance (``<u>``) with optional speaker reference."""

    xml_id: str | None = None
    speaker: str | None = None
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

    xml_id: str = msgspec.field(name="xml_id")
    desc: str | None = msgspec.field(default=None, name="desc")


class EncodingDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Collection of annotation systems."""

    annotation_systems: list[AnnotationSystem] = msgspec.field(
        default_factory=list, name="annotation_systems"
    )


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


class Episode(msgspec.Struct):
    """Top-level TEI document."""

    header: TeiHeader
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
    """Streaming event carrying a paragraph.

    Field duplication mirrors ``Paragraph`` so events stay flat for msgspec's
    tagged-union decoding."""

    xml_id: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)


class UtteranceEvent(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Streaming event carrying an utterance.

    Fields are mirrored instead of composed to keep the tagged payload shape
    stable and unambiguous."""

    xml_id: str | None = None
    speaker: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)


Event: TypeAlias = (
    DocumentStart | HeaderEvent | ParagraphEvent | UtteranceEvent | DocumentEnd
)
