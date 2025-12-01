"""msgspec.Struct projections of the TEI data model.

The classes mirror the serde layout used by the Rust `tei-core` crate so that
MessagePack produced by `tei_rapporteur.to_msgpack` decodes directly into
Python objects. Likewise, encoding these structs with `msgspec.msgpack.encode`
produces a payload that `tei_rapporteur.from_msgpack` accepts.
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
    """Emphasised inline span corresponding to <hi>."""

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
    """Paragraph block (<p>) containing inline content."""

    xml_id: str | None = msgspec.field(default=None, name="@xml:id")
    content: list[Inline] = msgspec.field(default_factory=list, name="$value")


class Utterance(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Spoken utterance (<u>) with an optional speaker reference."""

    xml_id: str | None = msgspec.field(default=None, name="@xml:id")
    speaker: str | None = msgspec.field(default=None, name="@who")
    content: list[Inline] = msgspec.field(default_factory=list, name="$value")


class ParagraphBlock(msgspec.Struct):
    """Externally tagged paragraph wrapper emitted by serde."""

    paragraph: Paragraph = msgspec.field(name="p")


class UtteranceBlock(msgspec.Struct):
    """Externally tagged utterance wrapper emitted by serde."""

    utterance: Utterance = msgspec.field(name="u")


# Body blocks are externally tagged in serde as either `p` or `u`. As with
# `Inline`, we keep static typing precise while relaxing runtime typing for
# msgspec compatibility.
if TYPE_CHECKING:
    BodyBlock: TypeAlias = ParagraphBlock | UtteranceBlock
else:
    BodyBlock = Any


class TeiBody(msgspec.Struct):
    """Ordered TEI body content."""

    blocks: list[BodyBlock] = msgspec.field(default_factory=list, name="$value")


class TeiText(msgspec.Struct):
    """Text node containing the TEI body."""

    body: TeiBody


class RevisionChange(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Single revision note within <revisionDesc>."""

    description: str = msgspec.field(name="$value")
    resp: str | None = msgspec.field(default=None, name="resp")


class RevisionDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Revision history container."""

    changes: list[RevisionChange] = msgspec.field(
        default_factory=list, name="change"
    )


class AnnotationSystem(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Metadata describing an annotation system."""

    xml_id: str = msgspec.field(name="@xml:id")
    desc: str | None = msgspec.field(default=None, name="desc")


class EncodingDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Collection of annotation systems."""

    annotation_systems: list[AnnotationSystem] = msgspec.field(
        default_factory=list, name="annotationSystem"
    )


class ProfileDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Audience and linguistic profile metadata."""

    synopsis: str | None = None
    speakers: list[str] = msgspec.field(default_factory=list, name="speaker")
    languages: list[str] = msgspec.field(default_factory=list, name="lang")


class FileDesc(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Bibliographic file description."""

    title: str
    series: str | None = None
    synopsis: str | None = None


class TeiHeader(msgspec.Struct, kw_only=True, omit_defaults=True):
    """Aggregated TEI header sections."""

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
    """Top-level TEI document."""

    header: TeiHeader = msgspec.field(name="teiHeader")
    text: TeiText
