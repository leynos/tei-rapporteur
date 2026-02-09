"""PyO3 bindings for the TEI Rapporteur toolkit.

Re-exports the public API from the native extension so that
``import tei_rapporteur`` provides direct access to :class:`Document`,
:func:`parse_xml`, :func:`emit_xml`, and the remaining helpers.
"""

from . import tei_rapporteur as _tei_rapporteur
from .tei_rapporteur import *

__doc__ = _tei_rapporteur.__doc__
if hasattr(_tei_rapporteur, "__all__"):
    __all__ = _tei_rapporteur.__all__
