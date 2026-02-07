from . import tei_rapporteur as _tei_rapporteur
from .tei_rapporteur import *

__doc__ = _tei_rapporteur.__doc__
if hasattr(_tei_rapporteur, "__all__"):
    __all__ = _tei_rapporteur.__all__
