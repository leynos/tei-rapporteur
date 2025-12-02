"""msgspec.Struct projections of the TEI data model.

The classes mirror the serde layout used by the Rust `tei-core` crate so that
MessagePack produced by `tei_rapporteur.to_msgpack` decodes directly into
Python objects. Likewise, encoding these structs with ``msgspec.msgpack.encode``
produces a payload that ``tei_rapporteur.from_msgpack`` accepts.

Note
----
`msgspec` limits unions to one mapping/struct type. To stay compatible with
the untagged serde layout, the runtime types for inline nodes and body blocks
are left as plain ``dict``/``list`` instances even though the static type
annotations reference :class:`Hi`, :class:`Pause`, :class:`ParagraphBlock`, and
:class:`UtteranceBlock`. Callers should treat decoded inline/body content as
untyped containers.

Examples
--------
Create an :class:`Episode`, encode it, and round-trip via Rust helpers::

    from tei_rapporteur.structs import Episode, FileDesc, TeiHeader, TeiText, TeiBody
    import msgspec
    import tei_rapporteur as tei

    episode = Episode(
        header=TeiHeader(
            file_desc=FileDesc(title="Bridgewater"),
        ),
        text=TeiText(body=TeiBody()),
    )

    payload = msgspec.msgpack.encode(episode)
    document = tei.from_msgpack(payload)
    assert document.title == "Bridgewater"
"""

from __future__ import annotations

import msgspec
from typing import TYPE_CHECKING, Any, TypedDict, TypeAlias

__all__ = [
    "AnnotationSystem",
    "BodyBlock",
    "EncodingDesc",
    "Episode",
    "FileDesc",
    "Hi",
    "Inline",
    "Paragraph",
    "ParagraphBlock",
    "Pause",
    "ProfileDesc",
    "RevisionChange",
    "RevisionDesc",
    "TeiBody",
    "TeiHeader",
    "TeiText",
    "Utterance",
    "UtteranceBlock",
]


class Hi(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Emphasised inline span corresponding to ``<hi>``.

    Attributes
    ----------
    rend:
        Optional rendering hint from ``@rend``.
    content:
        Nested inline content contained within the emphasis span.
    """

    rend: str | None = None
    content: list[Inline] = msgspec.field(default_factory=list, name="$value")


Pause = TypedDict(
    "Pause",
    {
        "@dur": str | None,
        "@type": str | None,
    },
    total=False,
)

# Inline can be plain text, emphasised spans, or pause maps. For static
# type-checking we expose the full union, while msgspec receives `Any` at
# runtime to avoid the restriction on multiple dict-like types in a single
# union.
if TYPE_CHECKING:
    Inline: TypeAlias = str | Hi | Pause
else:
    Inline = Any


class Paragraph(msgspec.Struct, kw_only=True):
    """Paragraph block (``<p>``) containing inline content.

    Attributes
    ----------
    xml_id:
        Optional XML identifier mapped from ``@xml:id``.
    content:
        Ordered list of inline nodes comprising the paragraph. At runtime this
        is a plain ``list`` of ``dict``/``str`` values because msgspec cannot
        materialise multiple mapping types in one union.
    """

    xml_id: str | None = msgspec.field(default=None, name="@xml:id")
    content: list[Inline] = msgspec.field(default_factory=list, name="$value")


class Utterance(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Spoken utterance (``<u>``) with an optional speaker reference.

    Attributes
    ----------
    xml_id:
        Optional XML identifier mapped from ``@xml:id``.
    speaker:
        Optional speaker identifier mapped from ``@who``.
    content:
        Ordered list of inline nodes comprising the utterance. At runtime this
        is a plain ``list`` of ``dict``/``str`` values because msgspec cannot
        materialise multiple mapping types in one union.
    """

    xml_id: str | None = msgspec.field(default=None, name="@xml:id")
    speaker: str | None = msgspec.field(default=None, name="@who")
    content: list[Inline] = msgspec.field(default_factory=list, name="$value")


class ParagraphBlock(msgspec.Struct):
    """Externally tagged paragraph wrapper emitted by serde.

    Attributes
    ----------
    paragraph:
        The wrapped :class:`Paragraph` stored under the ``p`` tag.
    """

    paragraph: Paragraph = msgspec.field(name="p")


class UtteranceBlock(msgspec.Struct):
    """Externally tagged utterance wrapper emitted by serde.

    Attributes
    ----------
    utterance:
        The wrapped :class:`Utterance` stored under the ``u`` tag.
    """

    utterance: Utterance = msgspec.field(name="u")


# Body blocks are externally tagged in serde as either `p` or `u`. As with
# `Inline`, we keep static typing precise while relaxing runtime typing for
# msgspec compatibility.
if TYPE_CHECKING:
    BodyBlock: TypeAlias = ParagraphBlock | UtteranceBlock
else:
    BodyBlock = Any


class TeiBody(msgspec.Struct):
    """Ordered TEI body content.

    Attributes
    ----------
    blocks:
        Sequence of :class:`ParagraphBlock` and :class:`UtteranceBlock` items
        mapped from the TEI ``<body>`` content. At runtime this is a plain
        ``list`` of dicts for compatibility with untagged unions.
    """

    blocks: list[BodyBlock] = msgspec.field(default_factory=list, name="$value")


class TeiText(msgspec.Struct):
    """Text node containing the TEI body.

    Attributes
    ----------
    body:
        The :class:`TeiBody` element nested under ``<text>``.
    """

    body: TeiBody


class RevisionChange(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Single revision note within ``<revisionDesc>``.

    Attributes
    ----------
    description:
        Free-text description stored in ``<change>`` contents.
    resp:
        Optional responsible party mapped from ``@resp``.
    """

    description: str = msgspec.field(name="$value")
    resp: str | None = msgspec.field(default=None, name="resp")


class RevisionDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Revision history container.

    Attributes
    ----------
    changes:
        List of :class:`RevisionChange` entries mapped from ``<change>`` nodes.
    """

    changes: list[RevisionChange] = msgspec.field(
        default_factory=list, name="change"
    )


class AnnotationSystem(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Metadata describing an annotation system.

    Attributes
    ----------
    xml_id:
        Identifier mapped from ``@xml:id``.
    desc:
        Optional description mapped from ``desc``.
    """

    xml_id: str = msgspec.field(name="@xml:id")
    desc: str | None = msgspec.field(default=None, name="desc")


class EncodingDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Collection of annotation systems.

    Attributes
    ----------
    annotation_systems:
        List of :class:`AnnotationSystem` items mapped from ``annotationSystem``.
    """

    annotation_systems: list[AnnotationSystem] = msgspec.field(
        default_factory=list, name="annotationSystem"
    )


class ProfileDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Audience and linguistic profile metadata.

    Attributes
    ----------
    synopsis:
        Optional profile summary.
    speakers:
        Speaker identifiers mapped from ``speaker`` elements.
    languages:
        Language tags mapped from ``lang`` elements.
    """

    synopsis: str | None = None
    speakers: list[str] = msgspec.field(default_factory=list, name="speaker")
    languages: list[str] = msgspec.field(default_factory=list, name="lang")


class FileDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Bibliographic file description.

    Attributes
    ----------
    title:
        Document title.
    series:
        Optional series name.
    synopsis:
        Optional synopsis text.
    """

    title: str
    series: str | None = None
    synopsis: str | None = None


class TeiHeader(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Aggregated TEI header sections.

    Attributes
    ----------
    file_desc:
        Mandatory :class:`FileDesc` mapped from ``fileDesc``.
    profile_desc:
        Optional :class:`ProfileDesc` mapped from ``profileDesc``.
    encoding_desc:
        Optional :class:`EncodingDesc` mapped from ``encodingDesc``.
    revision_desc:
        Optional :class:`RevisionDesc` mapped from ``revisionDesc``.
    """

    file_desc: FileDesc = msgspec.field(name="fileDesc")
    profile_desc: ProfileDesc | None = msgspec.field(
        default=None, name="profileDesc"
    )
    encoding_desc: EncodingDesc | None = msgspec.field(
        default=None, name="encodingDesc"
    )
    revision_desc: RevisionDesc | None = msgspec.field(
        default=None, name="revisionDesc"
    )


class Episode(msgspec.Struct):
    """Top-level TEI document.

    Attributes
    ----------
    header:
        The :class:`TeiHeader` node mapped from ``teiHeader``.
    text:
        The :class:`TeiText` node.
    """

    header: TeiHeader = msgspec.field(name="teiHeader")
    text: TeiText
