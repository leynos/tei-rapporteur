#![expect(
    clippy::too_many_arguments,
    reason = "PyO3 expands #[pyfunction] wrappers with ABI parameters"
)]

//! Python-visible streaming parser built on the Rust `TeiPullParser`.
//!
//! The iterator exposes `tei_rapporteur.iter_parse(xml: str)` and yields
//! msgspec-friendly tagged dictionaries representing streaming events.

use std::io::Cursor;

use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_serde::to_pyobject;
use tei_xml::streaming::{TeiEvent, TeiPullParser};

use crate::projection::py_event_from_core;

#[pyclass(module = "tei_rapporteur", name = "TeiEventIterator")]
pub struct TeiEventIterator {
    parser: Option<TeiPullParser<Cursor<Vec<u8>>>>,
    exhausted: bool,
}

impl TeiEventIterator {
    fn new(xml: &str) -> Self {
        let cursor = Cursor::new(xml.as_bytes().to_vec());
        let parser = TeiPullParser::new(cursor);
        Self {
            parser: Some(parser),
            exhausted: false,
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
        if self.exhausted {
            return Ok(None);
        }

        let Some(parser) = &mut self.parser else {
            self.exhausted = true;
            return Ok(None);
        };

        let next_event = py.allow_threads(|| parser.next());

        match next_event {
            None => {
                self.exhausted = true;
                Ok(None)
            }
            Some(Err(error)) => {
                self.exhausted = true;
                Err(PyValueError::new_err(error.to_string()))
            }
            Some(Ok(event)) => {
                if matches!(event, TeiEvent::DocumentEnd) {
                    self.exhausted = true;
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
#[pyfunction(name = "iter_parse")]
pub(crate) fn iter_parse_py(xml: &str) -> TeiEventIterator {
    TeiEventIterator::new(xml)
}
