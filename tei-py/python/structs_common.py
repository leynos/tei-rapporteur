"""Common `msgspec.Struct` projections shared by Python body and event models."""

from __future__ import annotations

import msgspec
from typing import TypeAlias


class InlineText(msgspec.Struct, tag="text", tag_field="type"):
    """Summary
    -------
    Plain text inline node within TEI inline content.

    ``InlineText`` represents unadorned character data inside paragraphs,
    utterances, headings, and other inline-capable containers.

    Attributes
    ----------
    value : str
        Raw text content preserved from the source document.
    """

    value: str


class InlineHi(msgspec.Struct, tag="hi", tag_field="type", omit_defaults=True):
    """Summary
    -------
    Emphasized inline span within paragraph, utterance, or heading content.

    ``InlineHi`` represents TEI ``<hi>`` markup and wraps nested inline nodes
    with an optional rendering hint for downstream styling or interpretation.

    Attributes
    ----------
    content : list[Inline]
        Nested inline nodes contained by the emphasized span. Defaults to an
        empty list.
    rend : str | None
        Optional TEI ``@rend`` hint describing the intended presentation or
        emphasis style. Defaults to ``None``.
    """

    content: list[Inline] = msgspec.field(default_factory=list)
    rend: str | None = None


class InlinePause(msgspec.Struct, tag="pause", tag_field="type", omit_defaults=True):
    """Summary
    -------
    Pause marker corresponding to TEI ``<pause/>`` inline markup.

    ``InlinePause`` captures empty-element pause annotations that may carry
    optional duration and pause-kind metadata.

    Attributes
    ----------
    dur : str | None
        Optional TEI ``@dur`` value describing the pause duration. Defaults to
        ``None``.
    kind : str | None
        Optional TEI ``@type`` value classifying the pause. Defaults to
        ``None``.
    """

    dur: str | None = None
    kind: str | None = None


Inline: TypeAlias = InlineText | InlineHi | InlinePause


class Paragraph(
    msgspec.Struct, tag="paragraph", tag_field="type", omit_defaults=True
):
    """Summary
    -------
    Paragraph body block with inline TEI content.

    ``Paragraph`` models a TEI ``<p>`` element after projection into the
    Python structs layer.

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


class Utterance(
    msgspec.Struct, tag="utterance", tag_field="type", omit_defaults=True
):
    """Summary
    -------
    Spoken utterance with inline TEI content and local provenance metadata.

    ``Utterance`` models a TEI ``<u>`` element together with the speech and
    annotation attributes preserved by the projection layer.

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


#: Summary
#: -------
#: Union of paragraph and utterance body blocks.
#:
#: ``TextBlock`` captures the textual body-block variants that may appear in
#: the TEI body and inside division content.
#:
#: Attributes
#: ----------
#: Paragraph
#:     Paragraph block with inline content and an optional XML identifier.
#: Utterance
#:     Spoken utterance block with inline content and local provenance
#:     metadata.
TextBlock: TypeAlias = Paragraph | Utterance


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
    """Summary
    -------
    Audience and linguistic profile metadata from ``<profileDesc>``.

    ``ProfileDesc`` collects high-level descriptive metadata about the
    document's synopsis, speakers, and languages.

    Attributes
    ----------
    synopsis : str | None
        Optional descriptive synopsis for the document. Defaults to ``None``.
    speakers : list[str]
        Speaker identifiers or labels associated with the document. Defaults to
        an empty list.
    languages : list[str]
        Language identifiers or labels associated with the document. Defaults
        to an empty list.
    """

    synopsis: str | None = None
    speakers: list[str] = msgspec.field(default_factory=list, name="speakers")
    languages: list[str] = msgspec.field(default_factory=list, name="languages")


class FileDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Summary
    -------
    Bibliographic file description from ``<fileDesc>``.

    ``FileDesc`` captures the core title-level metadata required for the TEI
    header projection.

    Attributes
    ----------
    title : str
        Document title.
    series : str | None
        Optional series title or grouping label. Defaults to ``None``.
    synopsis : str | None
        Optional descriptive synopsis associated with the file description.
        Defaults to ``None``.
    """

    title: str
    series: str | None = None
    synopsis: str | None = None


class TeiHeader(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Summary
    -------
    Aggregated TEI header sections projected into Python structs.

    ``TeiHeader`` groups the major TEI header subsections exposed by the
    embedded Python API.

    Attributes
    ----------
    file_desc : FileDesc
        Required bibliographic file description for the document.
    profile_desc : ProfileDesc | None
        Optional audience and linguistic profile metadata. Defaults to
        ``None``.
    encoding_desc : EncodingDesc | None
        Optional encoding metadata, including annotation systems and canonical
        citations. Defaults to ``None``.
    revision_desc : RevisionDesc | None
        Optional revision-history metadata. Defaults to ``None``.
    """

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
    """Summary
    -------
    Stand-off span with many-to-many or range-based targets.

    ``Span`` models TEI stand-off annotation links that may target explicit
    pointers, a from/to range, or both, together with provenance metadata.

    Attributes
    ----------
    xml_id : str | None
        Optional XML identifier preserved from ``@xml:id``. Defaults to
        ``None``.
    target : list[str]
        Explicit target pointers copied from ``@target``. Defaults to an empty
        list.
    from_ref : str | None
        Optional range start pointer copied from ``@from``. Defaults to
        ``None``.
    to_ref : str | None
        Optional range end pointer copied from ``@to``. Defaults to ``None``.
    source : list[str]
        Source pointers copied from ``@source``. Defaults to an empty list.
    resp : list[str]
        Responsibility pointers copied from ``@resp``. Defaults to an empty
        list.
    cert : str | None
        Optional certainty value copied from ``@cert``. Defaults to ``None``.
    corresp : list[str]
        Correspondence pointers copied from ``@corresp``. Defaults to an empty
        list.
    ana : list[str]
        Analysis pointers copied from ``@ana``. Defaults to an empty list.
    """

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
    """Summary
    -------
    Logical stand-off grouping of related annotation spans.

    ``SpanGroup`` corresponds to TEI ``<spanGrp>`` and groups related spans
    under a shared kind and optional provenance metadata.

    Attributes
    ----------
    xml_id : str | None
        Optional XML identifier preserved from ``@xml:id``. Defaults to
        ``None``.
    kind : str
        Required TEI ``@type`` value describing the span group.
    resp : list[str]
        Responsibility pointers copied from ``@resp``. Defaults to an empty
        list.
    corresp : list[str]
        Correspondence pointers copied from ``@corresp``. Defaults to an empty
        list.
    ana : list[str]
        Analysis pointers copied from ``@ana``. Defaults to an empty list.
    spans : list[Span]
        Span entries contained by the group. Defaults to an empty list.
    """

    xml_id: str | None = None
    kind: str
    resp: list[str] = msgspec.field(default_factory=list)
    corresp: list[str] = msgspec.field(default_factory=list)
    ana: list[str] = msgspec.field(default_factory=list)
    spans: list[Span] = msgspec.field(default_factory=list)


class StandOff(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Summary
    -------
    Root stand-off annotation layer for TEI overlays.

    ``StandOff`` collects the top-level stand-off annotation groups attached to
    a TEI document.

    Attributes
    ----------
    span_groups : list[SpanGroup]
        Top-level span groups contained by the stand-off layer. Defaults to an
        empty list.
    """

    span_groups: list[SpanGroup] = msgspec.field(default_factory=list)
