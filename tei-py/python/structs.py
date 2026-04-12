"""msgspec.Struct projections of the TEI data model."""

from __future__ import annotations

import msgspec
from typing import TypeAlias

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
    """Paragraph block from the TEI body."""
    xml_id: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)


class Utterance(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Spoken utterance with inline content and local provenance metadata."""
    xml_id: str | None = None
    speaker: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list)
    n: str | None = None
    source: list[str] = msgspec.field(default_factory=list)
    resp: list[str] = msgspec.field(default_factory=list)
    cert: str | None = None
    corresp: list[str] = msgspec.field(default_factory=list)
    ana: list[str] = msgspec.field(default_factory=list)


TextBlock: TypeAlias = Paragraph | Utterance


class Label(msgspec.Struct, omit_defaults=True):
    """Summary
    -------
    Label prefix attached to a structured list item.

    Attributes
    ----------
    content : list[Inline]
        Inline nodes rendered before the item's main content.

    Notes
    -----
    ``Label`` values typically hold numbering or short lead-in text and attach
    to :class:`Item` via ``Item.label``.
    """
    content: list[Inline] = msgspec.field(default_factory=list)


class Item(msgspec.Struct, omit_defaults=True):
    """Summary
    -------
    Structured list item contained within a division list.

    Attributes
    ----------
    content : list[Inline]
        Inline body content for the list item.
    xml_id : str | None
        Optional XML identifier for pointer resolution and linking.
    n : str | None
        Optional display number or ordinal marker.
    corresp : list[str]
        TEI pointer targets associated with the item.
    label : Label | None
        Optional prefix rendered before ``content``.

    Notes
    -----
    ``Item`` values belong inside :class:`ListBlock.items`. Use ``label`` for
    visible prefixes such as numbered bullets while keeping the main text in
    ``content``.
    """
    content: list[Inline] = msgspec.field(default_factory=list)
    xml_id: str | None = None
    n: str | None = None
    corresp: list[str] = msgspec.field(default_factory=list)
    label: Label | None = None


class ListBlock(msgspec.Struct, tag="list", tag_field="type", omit_defaults=True):
    """Summary
    -------
    List block nested inside a structural division.

    Attributes
    ----------
    items : list[Item]
        Ordered structured items contained by the list.
    xml_id : str | None
        Optional XML identifier for the list element.

    Notes
    -----
    ``ListBlock`` values appear inside :class:`DivBlock.content` and group one
    or more :class:`Item` values under a shared list container.
    """
    items: list[Item] = msgspec.field(default_factory=list)
    xml_id: str | None = None

    def __post_init__(self) -> None:
        if not self.items:
            raise ValueError("ListBlock must contain at least one Item")


class Head(msgspec.Struct, omit_defaults=True):
    """Division heading attached to the start of a structural division."""

    content: list[Inline] = msgspec.field(default_factory=list)


class DivBlock(msgspec.Struct, tag="div", tag_field="type", omit_defaults=True):
    """Summary
    -------
    Structural division within the TEI body.

    Attributes
    ----------
    div_type : str
        Required TEI ``@type`` value describing the division's role.
    subtype : str | None
        Optional TEI ``@subtype`` value refining ``div_type``.
    head : Head | None
        Optional heading emitted before the division's child blocks.
    content : list[DivContent]
        Ordered child blocks within the division.
    xml_id : str | None
        Optional XML identifier for the division.

    Notes
    -----
    ``DivBlock`` groups paragraphs, utterances, and lists into a named section
    such as chaptered material or show notes.
    """
    div_type: str
    subtype: str | None = None
    head: Head | None = None
    content: list[DivContent] = msgspec.field(default_factory=list)
    xml_id: str | None = None

    def __post_init__(self) -> None:
        if not self.div_type.strip():
            raise ValueError("DivBlock.div_type must contain non-whitespace text")
        if self.subtype is not None and not self.subtype.strip():
            raise ValueError("DivBlock.subtype must contain non-whitespace text")


DivContent: TypeAlias = TextBlock | ListBlock | DivBlock


BodyBlock: TypeAlias = TextBlock | DivBlock


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
    """Property extraction rule nested inside a citation declaration.

    Attributes
    ----------
    property : str
        Property name extracted from the matched TEI node.
    use_expr : str | None
        Optional TEI expression used to compute the property value.

    Notes
    -----
    ``CiteData`` entries hang off :class:`CiteStructure` and describe which
    citation metadata to pull from each matched node.
    """
    property: str
    use_expr: str | None = None


class CiteStructure(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Canonical citation declaration with optional nesting and extraction.

    Attributes
    ----------
    match_expr : str
        Required TEI match expression selecting the cited nodes.
    unit : str | None
        Optional human-facing citation unit label.
    use_expr : str | None
        Optional expression used when formatting the citation unit.
    delim : str | None
        Optional delimiter applied between nested citation units.
    cite_data : list[CiteData]
        Citation metadata extraction entries for the matched nodes.
    cite_structures : list[CiteStructure]
        Nested citation structures for hierarchical citation schemes.

    Notes
    -----
    ``CiteStructure`` models the TEI ``<citeStructure>`` tree inside
    :class:`RefsDecl`.
    """
    match_expr: str
    unit: str | None = None
    use_expr: str | None = None
    delim: str | None = None
    cite_data: list[CiteData] = msgspec.field(default_factory=list)
    cite_structures: list[CiteStructure] = msgspec.field(default_factory=list)


class RefsDecl(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Container for canonical citation declarations.

    Attributes
    ----------
    cite_structures : list[CiteStructure]
        Top-level citation declaration entries.

    Notes
    -----
    ``RefsDecl`` belongs under :class:`EncodingDesc` and documents how callers
    derive canonical references from the TEI body.
    """
    cite_structures: list[CiteStructure] = msgspec.field(default_factory=list)


class EncodingDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Encoding metadata for annotation systems and canonical citations.

    Attributes
    ----------
    annotation_systems : list[AnnotationSystem]
        Annotation-system declarations associated with the document.
    refs_decl : RefsDecl | None
        Optional canonical citation declaration tree.

    Notes
    -----
    ``EncodingDesc`` combines annotation-system documentation with
    :class:`RefsDecl` so projection consumers can inspect both in one place.
    """
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


class Episode(msgspec.Struct, omit_defaults=True):
    """Top-level TEI document projection."""
    header: TeiHeader
    text: TeiText
    stand_off: StandOff | None = None


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


Event: TypeAlias = (
    DocumentStart | HeaderEvent | ParagraphEvent | UtteranceEvent | DivEvent | DocumentEnd
)
