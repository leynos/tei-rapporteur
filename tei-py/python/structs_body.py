"""Body and structural-division projections for the embedded Python structs."""

from __future__ import annotations

import msgspec
from typing import TypeAlias

from _tei_rapporteur_structs_common import Inline, Paragraph, TextBlock, Utterance


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
        """Reject empty list blocks before they cross the Python boundary."""
        if not self.items:
            raise ValueError("ListBlock must contain at least one Item")


class Head(msgspec.Struct, omit_defaults=True):
    """Summary
    -------
    Division heading emitted at the start of a structural division.

    Attributes
    ----------
    content : list[Inline]
        Ordered inline nodes that make up the visible heading text.

    Notes
    -----
    ``Head`` is used by :class:`DivBlock` for optional division titles and must
    contain at least one inline node.
    """

    content: list[Inline] = msgspec.field(default_factory=list)

    def __post_init__(self) -> None:
        """Reject headings that would serialize without visible content."""
        if not self.content:
            raise ValueError("Head must contain at least one Inline node")


def _validate_div_invariants(div_type: str, subtype: str | None) -> None:
    """Validate the shared division type and subtype constraints."""
    if not div_type.strip():
        raise ValueError("div_type must contain non-whitespace text")
    if subtype is not None and not subtype.strip():
        raise ValueError("subtype must contain non-whitespace text")


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
        Ordered child blocks within the division, including paragraphs,
        utterances, lists, and nested ``DivBlock`` instances via
        ``DivContent``.
    xml_id : str | None
        Optional XML identifier for the division.

    Notes
    -----
    ``DivBlock`` groups paragraphs, utterances, lists, and recursive child
    divisions into a named section such as chaptered material or show notes.
    ``DivContent = TextBlock | ListBlock | DivBlock`` means a division can nest
    further structural divisions beneath itself.
    """

    div_type: str
    subtype: str | None = None
    head: Head | None = None
    content: list[DivContent] = msgspec.field(default_factory=list)
    xml_id: str | None = None

    def __post_init__(self) -> None:
        """Apply shared division validation after struct decoding."""
        _validate_div_invariants(self.div_type, self.subtype)


DivContent: TypeAlias = TextBlock | ListBlock | DivBlock


BodyBlock: TypeAlias = TextBlock | DivBlock


class TeiBody(msgspec.Struct):
    """Ordered TEI body content."""

    blocks: list[BodyBlock] = msgspec.field(default_factory=list)


class TeiText(msgspec.Struct):
    """Text node containing the TEI body."""

    body: TeiBody
