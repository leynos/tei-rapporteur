"""Type stubs for the ``tei_rapporteur.structs`` submodule (PEP 561).

The runtime definitions live in ``tei-py/python/structs.py`` and are
embedded into the native extension at compile time via ``include_str!``.
"""

import msgspec
from typing import TypeAlias

__all__: list[str]

class InlineText(msgspec.Struct, tag="text", tag_field="type"):
    """Plain text inline node."""

    value: str

class InlineHi(
    msgspec.Struct, tag="hi", tag_field="type", omit_defaults=True
):
    """Emphasised inline span."""

    content: list[Inline] = ...  # type: ignore[assignment]
    rend: str | None = None

class InlinePause(
    msgspec.Struct, tag="pause", tag_field="type", omit_defaults=True
):
    """Pause marker corresponding to ``<pause/>``."""

    dur: str | None = None
    kind: str | None = None

Inline: TypeAlias = InlineText | InlineHi | InlinePause

class Paragraph(
    msgspec.Struct, tag="paragraph", tag_field="type", omit_defaults=True
):
    """Paragraph block (``<p>``) containing inline content."""

    xml_id: str | None = None
    content: list[Inline] = ...  # type: ignore[assignment]

class Utterance(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Spoken utterance (``<u>``) with local provenance metadata."""

    xml_id: str | None = None
    n: str | None = None
    speaker: str | None = None
    source: list[str] = ...  # type: ignore[assignment]
    resp: list[str] = ...  # type: ignore[assignment]
    cert: str | None = None
    corresp: list[str] = ...  # type: ignore[assignment]
    ana: list[str] = ...  # type: ignore[assignment]
    content: list[Inline] = ...  # type: ignore[assignment]

BodyBlock: TypeAlias = Paragraph | Utterance

class TeiBody(msgspec.Struct):
    """Ordered TEI body content."""

    blocks: list[BodyBlock] = ...  # type: ignore[assignment]

class TeiText(msgspec.Struct):
    """Text node containing the TEI body."""

    body: TeiBody

class RevisionChange(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Single revision note within ``<revisionDesc>``."""

    description: str
    resp: str | None = None

class RevisionDesc(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Revision history container."""

    changes: list[RevisionChange] = ...  # type: ignore[assignment]

class AnnotationSystem(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Metadata describing an annotation system."""

    xml_id: str
    desc: str | None = None

class CiteData(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Property extraction entry within a citation structure."""

    property: str
    use_expr: str | None = None

class CiteStructure(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Canonical citation declaration entry."""

    match_expr: str
    unit: str | None = None
    use_expr: str | None = None
    delim: str | None = None
    cite_data: list[CiteData] = ...  # type: ignore[assignment]
    cite_structures: list[CiteStructure] = ...  # type: ignore[assignment]

class RefsDecl(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Container for canonical citation declarations."""

    cite_structures: list[CiteStructure] = ...  # type: ignore[assignment]

class EncodingDesc(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Collection of annotation systems and citation declarations."""

    annotation_systems: list[AnnotationSystem] = ...  # type: ignore[assignment]
    refs_decl: RefsDecl | None = None

class ProfileDesc(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Audience and linguistic profile metadata."""

    synopsis: str | None = None
    speakers: list[str] = ...  # type: ignore[assignment]
    languages: list[str] = ...  # type: ignore[assignment]

class FileDesc(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Bibliographic file description."""

    title: str
    series: str | None = None
    synopsis: str | None = None

class TeiHeader(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Aggregated TEI header sections."""

    file_desc: FileDesc
    profile_desc: ProfileDesc | None = None
    encoding_desc: EncodingDesc | None = None
    revision_desc: RevisionDesc | None = None

class Span(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Stand-off span with many-to-many or range-based targets."""

    xml_id: str | None = None
    target: list[str] = ...  # type: ignore[assignment]
    from_ref: str | None = None
    to_ref: str | None = None
    source: list[str] = ...  # type: ignore[assignment]
    resp: list[str] = ...  # type: ignore[assignment]
    cert: str | None = None
    corresp: list[str] = ...  # type: ignore[assignment]
    ana: list[str] = ...  # type: ignore[assignment]

class SpanGroup(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Logical stand-off grouping of related spans."""

    xml_id: str | None = None
    kind: str
    resp: list[str] = ...  # type: ignore[assignment]
    corresp: list[str] = ...  # type: ignore[assignment]
    ana: list[str] = ...  # type: ignore[assignment]
    spans: list[Span] = ...  # type: ignore[assignment]

class StandOff(
    msgspec.Struct, kw_only=True, omit_defaults=True
):
    """Root stand-off annotation layer."""

    span_groups: list[SpanGroup] = ...  # type: ignore[assignment]

class Episode(msgspec.Struct):
    """Top-level TEI document."""

    header: TeiHeader
    text: TeiText
    stand_off: StandOff | None = None

class DocumentStart(
    msgspec.Struct, tag="document_start", tag_field="type"
):
    """Streaming event signalling the start of parsing."""

class DocumentEnd(
    msgspec.Struct, tag="document_end", tag_field="type"
):
    """Streaming event signalling parsing completion."""

class HeaderEvent(
    msgspec.Struct, tag="header", tag_field="type"
):
    """Streaming event carrying the parsed header."""

    header: TeiHeader

class ParagraphEvent(
    msgspec.Struct, tag="paragraph", tag_field="type", omit_defaults=True
):
    """Streaming event carrying a paragraph."""

    xml_id: str | None = None
    content: list[Inline] = ...  # type: ignore[assignment]

class UtteranceEvent(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Streaming event carrying an utterance."""

    xml_id: str | None = None
    n: str | None = None
    speaker: str | None = None
    source: list[str] = ...  # type: ignore[assignment]
    resp: list[str] = ...  # type: ignore[assignment]
    cert: str | None = None
    corresp: list[str] = ...  # type: ignore[assignment]
    ana: list[str] = ...  # type: ignore[assignment]
    content: list[Inline] = ...  # type: ignore[assignment]

Event: TypeAlias = (
    DocumentStart | HeaderEvent | ParagraphEvent | UtteranceEvent
    | DocumentEnd
)
