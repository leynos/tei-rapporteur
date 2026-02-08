"""Static type validation harness for :mod:`tei_rapporteur` PEP 561 stubs.

This module is **not** executed at runtime.  It exists solely so that a
static type checker (``ty check``, pyright, mypy) can verify that the
public API exposed by the type stubs is correctly reflected at type
checking time.  Every :func:`typing.assert_type` call asks the checker
to confirm that the inferred type of the first argument matches the
second argument exactly.

Run with::

    ty check python/tests/test_type_stubs.py
"""

from __future__ import annotations

from typing import TYPE_CHECKING

__test__ = False

if TYPE_CHECKING:
    from typing import Any, assert_type

    import tei_rapporteur as tr
    from tei_rapporteur import structs

    # -- Module-level attributes ----------------------------------------------

    assert_type(tr.__version__, str)
    assert_type(tr.__py_runtime__, str)

    # -- Document class -------------------------------------------------------

    _doc = tr.Document("Example")
    assert_type(_doc, tr.Document)
    assert_type(_doc.title, str)
    assert_type(_doc.emit_title_markup(), str)
    assert_type(_doc.validate(), None)

    # -- Free functions: XML round-trip ---------------------------------------

    assert_type(tr.emit_title_markup("Example"), str)
    assert_type(tr.parse_xml("<TEI/>"), tr.Document)
    assert_type(tr.emit_xml(_doc), str)

    # -- Free functions: MessagePack round-trip -------------------------------

    assert_type(tr.to_msgpack(_doc), bytes)
    assert_type(tr.from_msgpack(b""), tr.Document)

    # -- Free functions: dict round-trip --------------------------------------

    assert_type(tr.to_dict(_doc), Any)
    assert_type(tr.from_dict({}), tr.Document)

    # -- Streaming iterator ---------------------------------------------------

    _iter = tr.iter_parse("<TEI/>")
    assert_type(_iter, tr.TeiEventIterator)
    assert_type(_iter.__iter__(), tr.TeiEventIterator)

    # -- structs: inline content types (tagged union) -------------------------

    _inline_text = structs.InlineText(value="hello")
    assert_type(_inline_text, structs.InlineText)
    assert_type(_inline_text.value, str)

    _inline_hi = structs.InlineHi()
    assert_type(_inline_hi, structs.InlineHi)
    assert_type(_inline_hi.rend, str | None)

    _inline_pause = structs.InlinePause()
    assert_type(_inline_pause, structs.InlinePause)
    assert_type(_inline_pause.dur, str | None)
    assert_type(_inline_pause.kind, str | None)

    # -- structs: body block types (tagged union) -----------------------------

    _para = structs.Paragraph()
    assert_type(_para, structs.Paragraph)
    assert_type(_para.xml_id, str | None)

    _utt = structs.Utterance()
    assert_type(_utt, structs.Utterance)
    assert_type(_utt.xml_id, str | None)
    assert_type(_utt.speaker, str | None)

    # -- structs: body / text structure ---------------------------------------

    _body = structs.TeiBody(blocks=[])
    assert_type(_body, structs.TeiBody)

    _text = structs.TeiText(body=_body)
    assert_type(_text, structs.TeiText)
    assert_type(_text.body, structs.TeiBody)

    # -- structs: header metadata ---------------------------------------------

    _file_desc = structs.FileDesc(title="T")
    assert_type(_file_desc, structs.FileDesc)
    assert_type(_file_desc.title, str)
    assert_type(_file_desc.series, str | None)
    assert_type(_file_desc.synopsis, str | None)

    _header = structs.TeiHeader(file_desc=_file_desc)
    assert_type(_header, structs.TeiHeader)
    assert_type(_header.file_desc, structs.FileDesc)
    assert_type(_header.profile_desc, structs.ProfileDesc | None)
    assert_type(_header.encoding_desc, structs.EncodingDesc | None)
    assert_type(_header.revision_desc, structs.RevisionDesc | None)

    _rev_change = structs.RevisionChange(description="Initial")
    assert_type(_rev_change, structs.RevisionChange)

    _rev_desc = structs.RevisionDesc(changes=[_rev_change])
    assert_type(_rev_desc, structs.RevisionDesc)

    _ann = structs.AnnotationSystem(xml_id="a1")
    assert_type(_ann, structs.AnnotationSystem)

    _enc = structs.EncodingDesc(annotation_systems=[_ann])
    assert_type(_enc, structs.EncodingDesc)

    _prof = structs.ProfileDesc()
    assert_type(_prof, structs.ProfileDesc)
    assert_type(_prof.synopsis, str | None)

    # -- structs: top-level document ------------------------------------------

    _episode = structs.Episode(header=_header, text=_text)
    assert_type(_episode, structs.Episode)
    assert_type(_episode.header, structs.TeiHeader)
    assert_type(_episode.text, structs.TeiText)

    # -- structs: streaming event types (tagged union) ------------------------

    _doc_start = structs.DocumentStart()
    assert_type(_doc_start, structs.DocumentStart)

    _doc_end = structs.DocumentEnd()
    assert_type(_doc_end, structs.DocumentEnd)

    _hdr_event = structs.HeaderEvent(header=_header)
    assert_type(_hdr_event, structs.HeaderEvent)
    assert_type(_hdr_event.header, structs.TeiHeader)

    _para_event = structs.ParagraphEvent()
    assert_type(_para_event, structs.ParagraphEvent)

    _utt_event = structs.UtteranceEvent()
    assert_type(_utt_event, structs.UtteranceEvent)

    # -- Type aliases ---------------------------------------------------------

    _inline_alias: structs.Inline = _inline_text
    assert_type(
        _inline_alias,
        structs.InlineText,  # narrowed by assignment
    )

    _block_alias: structs.BodyBlock = _para
    assert_type(
        _block_alias,
        structs.Paragraph,  # narrowed by assignment
    )

    _event_alias: structs.Event = _doc_start
    assert_type(
        _event_alias,
        structs.DocumentStart,  # narrowed by assignment
    )
