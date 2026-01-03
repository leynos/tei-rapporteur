//! Python-visible streaming parser built on the Rust `TeiPullParser`.
//!
//! The iterator exposes `tei_rapporteur.iter_parse(xml: str)` and yields
//! msgspec-friendly tagged dictionaries representing streaming events.

use std::io::{BufRead, Read};
use std::sync::Arc;

use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_serde::to_pyobject;
use tei_xml::streaming::{TeiEvent, TeiPullParser};

use crate::projection::py_event_from_core;

/// Lightweight reader over an owned byte buffer without copying.
#[derive(Clone)]
struct SliceReader {
    data: Arc<[u8]>,
    pos: usize,
}

impl SliceReader {
    #[expect(clippy::missing_const_for_fn, reason = "Arc construction is not const")]
    fn new(data: Arc<[u8]>) -> Self {
        Self { data, pos: 0 }
    }
}

impl Read for SliceReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let available = self.data.get(self.pos..).unwrap_or_default();
        let len = available.len().min(buf.len());
        let dest = buf.get_mut(..len).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "buffer shorter than len")
        })?;
        let src = available.get(..len).unwrap_or_default();
        dest.copy_from_slice(src);
        self.pos += len;
        Ok(len)
    }
}

impl BufRead for SliceReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(self.data.get(self.pos..).unwrap_or_default())
    }

    fn consume(&mut self, amt: usize) {
        self.pos = usize::min(self.pos + amt, self.data.len());
    }
}

#[pyclass(module = "tei_rapporteur", name = "TeiEventIterator")]
pub struct TeiEventIterator {
    parser: Option<TeiPullParser<SliceReader>>,
}

impl TeiEventIterator {
    fn new(xml: &str) -> Self {
        let buffer: Arc<[u8]> = Arc::from(xml.as_bytes());
        let reader = SliceReader::new(buffer);
        let parser = TeiPullParser::new(reader);
        Self {
            parser: Some(parser),
        }
    }
}

#[pymethods]
impl TeiEventIterator {
    #[expect(
        clippy::missing_const_for_fn,
        reason = "PyO3 iterator signature cannot be const"
    )]
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__<'py>(&'py mut self, py: Python<'py>) -> PyResult<Option<PyObject>> {
        let Some(parser) = self.parser.as_mut() else {
            return Ok(None);
        };

        let next_event = py.allow_threads(|| parser.next());

        match next_event {
            None => {
                self.parser = None;
                Ok(None)
            }
            Some(Err(error)) => {
                self.parser = None;
                Err(PyValueError::new_err(error.to_string()))
            }
            Some(Ok(event)) => {
                if matches!(event, TeiEvent::DocumentEnd) {
                    self.parser = None;
                }
                let projected = py_event_from_core(event);
                let py_obj = to_pyobject(py, &projected)
                    .map_err(|error| PyValueError::new_err(error.to_string()))?;
                Ok(Some(py_obj.unbind()))
            }
        }
    }
}

/// Exposes `iter_parse` to Python, yielding streaming events for the provided
/// XML string.
pub(crate) fn iter_parse_py(xml: &str) -> TeiEventIterator {
    TeiEventIterator::new(xml)
}
