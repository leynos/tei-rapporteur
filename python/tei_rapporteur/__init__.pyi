"""Type stubs for the ``tei_rapporteur`` native extension module (PEP 561)."""

from typing import Any

from tei_rapporteur.structs import Event as Event

__version__: str
"""Package version sourced from Cargo metadata."""

__py_runtime__: str
"""Python runtime version string."""

class Document:
    """PyO3 wrapper around the Rust ``TeiDocument``."""

    def __init__(self, title: str) -> None:
        """Construct a document with the provided title.

        Raises:
            ValueError: When the trimmed *title* is empty.
        """
        ...

    @property
    def title(self) -> str:
        """Return the validated document title."""
        ...

    def emit_title_markup(self) -> str:
        """Emit the document title as TEI markup.

        Raises:
            ValueError: When the stored document title is invalid.
        """
        ...

    def validate(self) -> None:
        """Validate document-wide invariants.

        Raises:
            ValueError: When duplicated identifiers or unknown speaker
                references are detected.
        """
        ...

class TeiEventIterator:
    """Streaming iterator yielding tagged TEI event dictionaries.

    Each element is a ``dict`` whose ``"type"`` key discriminates the
    event kind.  The dicts conform to the shapes defined by
    :data:`tei_rapporteur.structs.Event` and can be materialised into
    typed structs via ``msgspec.convert(event, Event)``.
    """

    def __iter__(self) -> TeiEventIterator: ...
    def __next__(self) -> dict[str, Any]:
        """Yield the next streaming event as a tagged dictionary.

        The returned ``dict`` always contains a ``"type"`` key with one
        of the values ``"document_start"``, ``"header"``,
        ``"paragraph"``, ``"utterance"``, or ``"document_end"``.  Use
        ``msgspec.convert(event, tei_rapporteur.structs.Event)`` to
        decode into the corresponding typed struct.

        Raises:
            ValueError: On malformed XML or TEI validation failures.
            StopIteration: When the stream is exhausted.
        """
        ...

def emit_title_markup(raw_title: str) -> str:
    """Wrap *raw_title* in TEI ``<title>`` markup.

    Raises:
        ValueError: When the trimmed title is blank.
    """
    ...

def parse_xml(xml: str) -> Document:
    """Parse a TEI XML string into a :class:`Document`.

    Raises:
        ValueError: On malformed XML or invalid TEI content.
    """
    ...

def emit_xml(document: Document) -> str:
    """Serialise a :class:`Document` to a TEI XML string.

    Raises:
        ValueError: On forbidden control characters or emission failure.
    """
    ...

def from_msgpack(data: bytes) -> Document:
    """Deserialise MessagePack *data* into a :class:`Document`.

    Raises:
        ValueError: When the payload cannot be decoded.
    """
    ...

def to_msgpack(document: Document) -> bytes:
    """Serialise a :class:`Document` to MessagePack bytes.

    Raises:
        ValueError: When encoding fails.
    """
    ...

def from_dict(payload: Any) -> Document:
    """Construct a :class:`Document` from a Python dict/list tree.

    Raises:
        ValueError: When the payload cannot be deserialised.
        TypeError: On type-mismatch errors in the payload structure.
    """
    ...

def to_dict(document: Document) -> Any:
    """Convert a :class:`Document` to a Python dict/list tree.

    Raises:
        ValueError: When conversion fails.
    """
    ...

def iter_parse(xml: str) -> TeiEventIterator:
    """Return a streaming iterator over TEI events for the given XML.

    Raises:
        ValueError: When initial parsing setup fails.
    """
    ...
